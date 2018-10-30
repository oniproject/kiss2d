// This file contains a fixed point math implementation of the vector
// graphics rasterizer.

use super::{clamp, Rasterizer};

// ϕ is the number of binary digits after the fixed point.
//
// For example, if ϕ == 10 (and int1ϕ is based on the int32 type) then we
// are using 22.10 fixed point math.
//
// When changing this number, also change the assembly code (search for ϕ
// in the .s files).
const ϕ: int1ϕ = 9;

const fxOne:          int1ϕ = 1 << ϕ;
const fxOneAndAHalf:  int1ϕ = 1<<ϕ + 1<<(ϕ-1);
const fxOneMinusIota: int1ϕ = 1<<ϕ - 1; // Used for rounding up.

// int1ϕ is a signed fixed-point number with 1*ϕ binary digits after the fixed
// point.
type int1ϕ = i32;

// int2ϕ is a signed fixed-point number with 2*ϕ binary digits after the fixed
// point.
//
// The Rasterizer's bufU32 field, nominally of type []uint32 (since that slice
// is also used by other code), can be thought of as a []int2ϕ during the
// fixedLineTo method. Lines of code that are actually like:
//	buf[i] += uint32(etc) // buf has type []uint32.
// can be thought of as
//	buf[i] += int2ϕ(etc)  // buf has type []int2ϕ.
type int2ϕ = i32;

#[inline(always)] fn fmax(x: int1ϕ, y: int1ϕ) -> int1ϕ { if x > y { x } else { y } }
#[inline(always)] fn fmin(x: int1ϕ, y: int1ϕ) -> int1ϕ { if x < y { x } else { y } }

#[inline(always)] fn floor(x: int1ϕ) -> i32 { (x >> ϕ) }
#[inline(always)] fn ceil(x: int1ϕ) -> i32  { ((x + fxOneMinusIota) >> ϕ) }

impl Rasterizer {
    pub fn fixed_accumulate_mask(&mut self) {
        let buf = self.buf.as_u32();
        let mut acc = 0i32;
        for v in buf {
            acc += (*v) as i32;
            let mut a = acc;
            if a < 0 { a = -a }
            a >>= 2*ϕ - 16;
            if a > 0xffff { a = 0xffff; }
            *v = a as u32;
        }
    }

    pub fn fixed_line_to(&mut self, bx: f32, by: f32) {
        let [ax, ay] = self.pen;
        self.pen = [bx, by];

        let (dir, ax, ay, bx, by) = if ay > by {
            (-1, bx, by, ax, ay)
        } else {
            (1, ax, ay, bx, by)
        };

        // Horizontal line segments yield no change in coverage. Almost horizontal
        // segments would yield some change, in ideal math, but the computation
        // further below, involving 1 / (by - ay), is unstable in fixed point math,
        // so we treat the segment as if it was perfectly horizontal.
        if by-ay <= 0.000001 { return }
        let dxdy = (bx - ax) / (by - ay);

        let ayϕ = (ay * (fxOne as f32)) as int1ϕ;
        let byϕ = (by * (fxOne as f32)) as int1ϕ;

        let mut x = (ax * (fxOne as f32)) as int1ϕ;
        let mut y = floor(ayϕ);
        let y_max = ceil(byϕ);
        let y_max = if y_max > self.size[1] as i32 {
            self.size[1] as i32
        } else {
            y_max
        };
        let width = self.size[0] as i32;

        while y < y_max {
            let dy = fmin((1 + y as int1ϕ)<<ϕ, byϕ) - fmax((y as int1ϕ)<<ϕ, ayϕ);
            let x_next = x + ((dy as f32)*dxdy) as int1ϕ;
            if y < 0 {
                x = x_next;
                continue;
            }
            let buf = &mut self.buf.as_u32()[(y*width) as usize..];
            let d = dy * dir; // d ranges up to ±1<<(1*ϕ).
            let (x0, x1) = if x > x_next {
                (x_next, x)
            } else {
                (x, x_next)
            };
            let x0i = floor(x0);
            let x0floor = (x0i as int1ϕ) << ϕ;
            let x1i = ceil(x1);
            let x1ceil = (x1i as int1ϕ) << ϕ;

            if x1i <= x0i+1 {
                let xmf = (x+x_next)>>1 - x0floor;
                let i = clamp(x0i+0, width);
                if i < buf.len() {
                    buf[i] += (d * (fxOne - xmf)) as u32
                }
                let i = clamp(x0i+1, width);
                if i < buf.len() {
                    buf[i] += (d * xmf) as u32
                }
            } else {
                let one_over_s = x1 - x0;
                let two_over_s = 2 * one_over_s;
                let x0f = x0 - x0floor;
                let one_minus_x0f = fxOne - x0f;
                let one_minus_x0f_squared = one_minus_x0f * one_minus_x0f;
                let x1f = x1 - x1ceil + fxOne;
                let x1f_squared = x1f * x1f;

                // These next two variables are unused, as rounding errors are
                // minimized when we delay the division by oneOverS for as long as
                // possible. These lines of code (and the "In ideal math" comments
                // below) are commented out instead of deleted in order to aid the
                // comparison with the floating point version of the rasterizer.
                //
                // a0 := ((oneMinusX0f * oneMinusX0f) >> 1) / oneOverS
                // am := ((x1f * x1f) >> 1) / oneOverS

                let i = clamp(x0i, width);
                if i < buf.len() {
                    // In ideal math: buf[i] += uint32(d * a0)
                    let mut D = one_minus_x0f_squared; // D ranges up to ±1<<(2*ϕ).
                    D *= d;                            // D ranges up to ±1<<(3*ϕ).
                    D /= two_over_s;
                    buf[i] += D as u32;
                }

                if x1i == x0i+2 {
                    let i = clamp(x0i+1, width);
                    if i < buf.len() {
                        // In ideal math: buf[i] += uint32(d * (fxOne - a0 - am))
                        //
                        // (x1i == x0i+2) and (twoOverS == 2 * (x1 - x0)) implies
                        // that twoOverS ranges up to +1<<(1*ϕ+2).
                        let mut D = two_over_s<<ϕ - one_minus_x0f_squared - x1f_squared; // D ranges up to ±1<<(2*ϕ+2).
                        D *= d;                                            // D ranges up to ±1<<(3*ϕ+2).
                        D /= two_over_s;
                        buf[i] += D as u32;
                    }
                } else {
                    // This is commented out for the same reason as a0 and am.
                    //
                    // a1 := ((fxOneAndAHalf - x0f) << ϕ) / oneOverS
                    let i = clamp(x0i+1, width);
                    if i < buf.len() {
                        // In ideal math:
                        //	buf[i] += uint32(d * (a1 - a0))
                        // or equivalently (but better in non-ideal, integer math,
                        // with respect to rounding errors),
                        //	buf[i] += uint32(A * d / twoOverS)
                        // where
                        //	A = (a1 - a0) * twoOverS
                        //	  = a1*twoOverS - a0*twoOverS
                        // Noting that twoOverS/oneOverS equals 2, substituting for
                        // a0 and then a1, given above, yields:
                        //	A = a1*twoOverS - oneMinusX0fSquared
                        //	  = (fxOneAndAHalf-x0f)<<(ϕ+1) - oneMinusX0fSquared
                        //	  = fxOneAndAHalf<<(ϕ+1) - x0f<<(ϕ+1) - oneMinusX0fSquared
                        //
                        // This is a positive number minus two non-negative
                        // numbers. For an upper bound on A, the positive number is
                        //	P = fxOneAndAHalf<<(ϕ+1)
                        //	  < (2*fxOne)<<(ϕ+1)
                        //	  = fxOne<<(ϕ+2)
                        //	  = 1<<(2*ϕ+2)
                        //
                        // For a lower bound on A, the two non-negative numbers are
                        //	N = x0f<<(ϕ+1) + oneMinusX0fSquared
                        //	  ≤ x0f<<(ϕ+1) + fxOne*fxOne
                        //	  = x0f<<(ϕ+1) + 1<<(2*ϕ)
                        //	  < x0f<<(ϕ+1) + 1<<(2*ϕ+1)
                        //	  ≤ fxOne<<(ϕ+1) + 1<<(2*ϕ+1)
                        //	  = 1<<(2*ϕ+1) + 1<<(2*ϕ+1)
                        //	  = 1<<(2*ϕ+2)
                        //
                        // Thus, A ranges up to ±1<<(2*ϕ+2). It is possible to
                        // derive a tighter bound, but this bound is sufficient to
                        // reason about overflow.
                        let mut D = (fxOneAndAHalf-x0f)<<(ϕ+1) - one_minus_x0f_squared; // D ranges up to ±1<<(2*ϕ+2).
                        D *= d;                                               // D ranges up to ±1<<(3*ϕ+2).
                        D /= two_over_s;
                        buf[i] += D as u32;
                    }
                    let d_times_s = ((d << (2 * ϕ)) / one_over_s) as u32;
                    for xi in (x0i + 2)..(x1i-1) {
                        let i = clamp(xi, width);
                        if  i < buf.len() {
                            buf[i] += d_times_s;
                        }
                    }

                    // This is commented out for the same reason as a0 and am.
                    //
                    // a2 := a1 + (int1ϕ(x1i-x0i-3)<<(2*ϕ))/oneOverS
                    let i = clamp(x1i-1, width);
                    if i < buf.len() {
                        // In ideal math:
                        //	buf[i] += uint32(d * (fxOne - a2 - am))
                        // or equivalently (but better in non-ideal, integer math,
                        // with respect to rounding errors),
                        //	buf[i] += uint32(A * d / twoOverS)
                        // where
                        //	A = (fxOne - a2 - am) * twoOverS
                        //	  = twoOverS<<ϕ - a2*twoOverS - am*twoOverS
                        // Noting that twoOverS/oneOverS equals 2, substituting for
                        // am and then a2, given above, yields:
                        //	A = twoOverS<<ϕ - a2*twoOverS - x1f*x1f
                        //	  = twoOverS<<ϕ - a1*twoOverS - (int1ϕ(x1i-x0i-3)<<(2*ϕ))*2 - x1f*x1f
                        //	  = twoOverS<<ϕ - a1*twoOverS - int1ϕ(x1i-x0i-3)<<(2*ϕ+1) - x1f*x1f
                        // Substituting for a1, given above, yields:
                        //	A = twoOverS<<ϕ - ((fxOneAndAHalf-x0f)<<ϕ)*2 - int1ϕ(x1i-x0i-3)<<(2*ϕ+1) - x1f*x1f
                        //	  = twoOverS<<ϕ - (fxOneAndAHalf-x0f)<<(ϕ+1) - int1ϕ(x1i-x0i-3)<<(2*ϕ+1) - x1f*x1f
                        //	  = B<<ϕ - x1f*x1f
                        // where
                        //	B = twoOverS - (fxOneAndAHalf-x0f)<<1 - int1ϕ(x1i-x0i-3)<<(ϕ+1)
                        //	  = (x1-x0)<<1 - (fxOneAndAHalf-x0f)<<1 - int1ϕ(x1i-x0i-3)<<(ϕ+1)
                        //
                        // Re-arranging the defintions given above:
                        //	x0Floor := int1ϕ(x0i) << ϕ
                        //	x0f := x0 - x0Floor
                        //	x1Ceil := int1ϕ(x1i) << ϕ
                        //	x1f := x1 - x1Ceil + fxOne
                        // combined with fxOne = 1<<ϕ yields:
                        //	x0 = x0f + int1ϕ(x0i)<<ϕ
                        //	x1 = x1f + int1ϕ(x1i-1)<<ϕ
                        // so that expanding (x1-x0) yields:
                        //	B = (x1f-x0f + int1ϕ(x1i-x0i-1)<<ϕ)<<1 - (fxOneAndAHalf-x0f)<<1 - int1ϕ(x1i-x0i-3)<<(ϕ+1)
                        //	  = (x1f-x0f)<<1 + int1ϕ(x1i-x0i-1)<<(ϕ+1) - (fxOneAndAHalf-x0f)<<1 - int1ϕ(x1i-x0i-3)<<(ϕ+1)
                        // A large part of the second and fourth terms cancel:
                        //	B = (x1f-x0f)<<1 - (fxOneAndAHalf-x0f)<<1 - int1ϕ(-2)<<(ϕ+1)
                        //	  = (x1f-x0f)<<1 - (fxOneAndAHalf-x0f)<<1 + 1<<(ϕ+2)
                        //	  = (x1f - fxOneAndAHalf)<<1 + 1<<(ϕ+2)
                        // The first term, (x1f - fxOneAndAHalf)<<1, is a negative
                        // number, bounded below by -fxOneAndAHalf<<1, which is
                        // greater than -fxOne<<2, or -1<<(ϕ+2). Thus, B ranges up
                        // to ±1<<(ϕ+2). One final simplification:
                        //	B = x1f<<1 + (1<<(ϕ+2) - fxOneAndAHalf<<1)
                        //const C: i32 = 1<<(ϕ+2) - fxOneAndAHalf<<1;
                        #[allow(exceeding_bitshifts)]
                        let mut D = x1f<<1 + (1<<(ϕ+2) - fxOneAndAHalf<<1); // D ranges up to ±1<<(1*ϕ+2).
                        D <<= ϕ;          // D ranges up to ±1<<(2*ϕ+2).
                        D -= x1f_squared; // D ranges up to ±1<<(2*ϕ+3).
                        D *= d;           // D ranges up to ±1<<(3*ϕ+3).
                        D /= two_over_s;
                        buf[i] += D as u32;
                    }
                }
                let i = clamp(x1i, width);
                if i < buf.len() {
                    // In ideal math: buf[i] += uint32(d * am)
                    let mut D = x1f_squared; // D ranges up to ±1<<(2*ϕ).
                    D *= d;         // D ranges up to ±1<<(3*ϕ).
                    D /= two_over_s;
                    buf[i] += D as u32;
                }
            }

            x = x_next;
            y += 1;
        }
    }
}


/*
fn fixedAccumulateOpOver(dst []uint8, src []uint32) {
    // Sanity check that len(dst) >= len(src).
    if len(dst) < len(src) {
        return
    }

    acc := int2ϕ(0)
    for i, v := range src {
        acc += int2ϕ(v)
        a := acc
        if a < 0 {
            a = -a
        }
        a >>= 2*ϕ - 16
        if a > 0xffff {
            a = 0xffff
        }
        // This algorithm comes from the standard library's image/draw package.
        dstA := uint32(dst[i]) * 0x101
        maskA := uint32(a)
        outA := dstA*(0xffff-maskA)/0xffff + maskA
        dst[i] = uint8(outA >> 8)
    }
}

fn fixedAccumulateOpSrc(dst []uint8, src []uint32) {
    // Sanity check that len(dst) >= len(src).
    if len(dst) < len(src) {
        return
    }

    acc := int2ϕ(0)
    for i, v := range src {
        acc += int2ϕ(v)
        a := acc
        if a < 0 {
            a = -a
        }
        a >>= 2*ϕ - 8
        if a > 0xff {
            a = 0xff
        }
        dst[i] = uint8(a)
    }
}
*/
