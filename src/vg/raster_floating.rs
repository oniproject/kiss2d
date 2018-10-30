/// This file contains a floating point math implementation of the vector graphics rasterizer.

use super::{clamp, Rasterizer};

#[inline(always)] fn fmax(x: f32, y: f32) -> f32 { if x > y { x } else { y } }
#[inline(always)] fn fmin(x: f32, y: f32) -> f32 { if x < y { x } else { y } }
#[inline(always)] fn floor(x: f32) -> i32 { (x as f64).floor() as i32 }
#[inline(always)] fn ceil(x: f32)  -> i32 { (x as f64).ceil()  as i32 }

impl Rasterizer {
    pub fn floating_accumulate_mask(&mut self) {
        let buf = self.buf.as_u32();
        let mut acc = 0f32;
        for v in buf {
            acc += unsafe { *(v as *mut u32 as *mut f32) };
            let a = clamp_alpha(acc);
            *v = (ALMOST65536 * a) as u32;
        }
    }

    pub fn floating_line_to(&mut self, bx: f32, by: f32) {
        let [ax, ay] = self.pen;
        self.pen = [bx, by];

        let (dir, ax, ay, bx, by) = if ay > by {
            (-1f32, bx, by, ax, ay)
        } else {
            (1f32, ax, ay, bx, by)
        };

        // Horizontal line segments yield no change in coverage. Almost horizontal
        // segments would yield some change, in ideal math, but the computation
        // further below, involving 1 / (by - ay), is unstable in floating point
        // math, so we treat the segment as if it was perfectly horizontal.
        if by-ay <= 0.000001 { return }
        let dxdy = (bx - ax) / (by - ay);

        let mut x = ax;
        let mut y = floor(ay);
        let y_max = ceil(by);
        let y_max = if y_max > self.size[1] as i32 {
            self.size[1] as i32
        } else {
            y_max
        };
        let width = self.size[0] as i32;

        while y < y_max {
            let dy = fmin((y+1) as f32, by) - fmax(y as f32, ay);

            // The "float32" in expressions like "float32(foo*bar)" here and below
            // look redundant, since foo and bar already have type float32, but are
            // explicit in order to disable the compiler's Fused Multiply Add (FMA)
            // instruction selection, which can improve performance but can result
            // in different rounding errors in floating point computations.
            //
            // This package aims to have bit-exact identical results across all
            // GOARCHes, and across pure Go code and assembly, so it disables FMA.
            //
            // See the discussion at
            // https://groups.google.com/d/topic/golang-dev/Sti0bl2xUXQ/discussion
            let x_next = x + (dy * dxdy) as f32;
            if y < 0 {
                x = x_next;
                continue;
            }

            let buf = &mut self.buf.as_f32()[(y*width) as usize..];
            let d = (dy * dir) as f32;

            let (x0, x1) = if x > x_next {
                (x_next, x)
            } else {
                (x, x_next)
            };


            let x0i = floor(x0);
            let x0floor = x0i as f32;
            let x1i = ceil(x1);
            let x1ceil = x1i as f32;

            if x1i <= x0i+1 {
                let xmf = (0.5 * (x+x_next)) as f32 - x0floor;
                let i = clamp(x0i+0, width);
                if i < buf.len() {
                    buf[i] += d - (d*xmf) as f32;
                }
                let i = clamp(x0i+1, width);
                if i < buf.len() {
                    buf[i] += (d * xmf) as f32;
                }
            } else {
                let s = 1.0 / (x1 - x0);
                let x0f = x0 - x0floor;
                let one_minus_x0f = 1.0 - x0f;
                let a0 = (0.5 * s * one_minus_x0f * one_minus_x0f) as f32;
                let x1f = x1 - x1ceil + 1.0;
                let am = (0.5 * s * x1f * x1f) as f32;

                let i = clamp(x0i, width);
                if i < buf.len() {
                    buf[i] += (d * a0) as f32;
                }

                if x1i == x0i+2 {
                    let i = clamp(x0i+1, width);
                    if i < buf.len() {
                        buf[i] += (d * (1.0 - a0 - am)) as f32;
                    }
                } else {
                    let a1 = (s * (1.5 - x0f)) as f32;
                    let i = clamp(x0i+1, width);
                    if i < buf.len() {
                        buf[i] += (d * (a1 - a0)) as f32;
                    }

                    let d_times_s = (d * s) as f32;
                    for xi in (x0i + 2)..(x1i-1) {
                        let i = clamp(xi, width);
                        if i < buf.len() {
                            buf[i] += d_times_s;
                        }
                    }

                    let a2 = a1 + (s * (x1i-x0i-3) as f32) as f32;
                    let i = clamp(x1i-1, width);
                    if i < buf.len() {
                        buf[i] += (d * (1.0 - a2 - am)) as f32;
                    }
                }

                let i = clamp(x1i, width);
                if i < buf.len() {
                    buf[i] += (d * am) as f32;
                }
            }

            x = x_next;
            y += 1;
        }
    }
}

// almost256 scales a floating point value in the range [0, 1] to a uint8
// value in the range [0x00, 0xff].
//
// 255 is too small. Floating point math accumulates rounding errors, so a
// fully covered src value that would in ideal math be float32(1) might be
// float32(1-ε), and uint8(255 * (1-ε)) would be 0xfe instead of 0xff. The
// uint8 conversion rounds to zero, not to nearest.
//
// 256 is too big. If we multiplied by 256, below, then a fully covered src
// value of float32(1) would translate to uint8(256 * 1), which can be 0x00
// instead of the maximal value 0xff.
//
// math.Float32bits(almost256) is 0x437fffff.
const ALMOST256: f32 = 255.99998;

// almost65536 scales a floating point value in the range [0, 1] to a
// uint16 value in the range [0x0000, 0xffff].
//
// math.Float32bits(almost65536) is 0x477fffff.
const ALMOST65536: f32 = ALMOST256 * 256.0;

#[inline(always)]
fn clamp_alpha(mut a: f32) -> f32 {
    if a < 0.0 { a = -a; }
    if a > 1.0 { a = 1.0; }
    a
}

pub fn accumulate_op_over(dst: &mut [u8], src: &[f32]) {
    // Sanity check that dst.len() >= src.len().
    if dst.len() < src.len() { return }
    let mut acc = 0f32;
    for (i, v) in src.iter().enumerate() {
        acc += *v;
        let a = clamp_alpha(acc);
        // This algorithm comes from the standard library's image/draw package.
        let dst_a = (dst[i] as u32) * 0x101;
        let mask_a = (ALMOST65536 * a) as u32;
        let out_a = dst_a * (0xFFFF - mask_a) / 0xFFFF + mask_a;
        dst[i] = (out_a >> 8) as u8;
    }
}

pub fn accumulate_op_src(dst: &mut [u8], src: &[f32]) {
    // Sanity check that dst.len() >= src.len().
    if dst.len() < src.len() { return }
    let mut acc = 0f32;
    for (i, v) in src.iter().enumerate() {
        acc += *v;
        let a = clamp_alpha(acc);
        dst[i] = (ALMOST256 * a) as u8;
    }
}

pub fn accumulate_mask(dst: &mut [u32], src: &[f32]) {
    // Sanity check that dst.len() >= src.len().
    if dst.len() < src.len() { return }
    let mut acc = 0f32;
    for (i, v) in src.iter().enumerate() {
        acc += *v;
        let a = clamp_alpha(acc);
        dst[i] = (ALMOST65536 * a) as u32;
    }
}

pub fn accumulate_mask_x(buf: &mut [u32]) {
    let src = unsafe { std::mem::transmute(&buf[..]) };
    accumulate_mask(buf, src)
}

pub fn accumulate_mask_inplace(buf: &mut super::SimdVec) {
    unsafe {
        let dst = buf.u_u32();
        let src = buf.u_f32();
        accumulate_mask(dst, src)
    }
}
