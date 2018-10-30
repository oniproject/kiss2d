use super::{lerp, dev_squared, Rasterizer, Op, SimdVec};
use crate::image::{RGBA, Rectangle, Point};

/*

//go:generate go run gen.go
//go:generate asmfmt -w acc_amd64.s

// asmfmt is https://github.com/klauspost/asmfmt

// Package vector provides a rasterizer for 2-D vector graphics.
package vector // import "golang.org/x/image/vector"

// The rasterizer's design follows
// https://medium.com/@raphlinus/inside-the-fastest-font-renderer-in-the-world-75ae5270c445
//
// Proof of concept code is in
// https://github.com/google/font-go
//
// See also:
// http://nothings.org/gamedev/rasterize/
// http://projects.tuxee.net/cl-vectors/section-the-cl-aa-algorithm
// https://people.gnome.org/~mathieu/libart/internals.html#INTERNALS-SCANLINE

import (
    "image"
    "image/color"
    "image/draw"
    "math"
)
*/

// floatingPointMathThreshold is the width or height above which the rasterizer
// chooses to used floating point math instead of fixed point math.
//
// Both implementations of line segmentation rasterization (see raster_fixed.go
// and raster_floating.go) implement the same algorithm (in ideal, infinite
// precision math) but they perform differently in practice. The fixed point
// math version is roughtly 1.25x faster (on GOARCH=amd64) on the benchmarks,
// but at sufficiently large scales, the computations will overflow and hence
// show rendering artifacts. The floating point math version has more
// consistent quality over larger scales, but it is significantly slower.
//
// This constant determines when to use the faster implementation and when to
// use the better quality implementation.
//
// The rationale for this particular value is that TestRasterizePolygon in
// vector_test.go checks the rendering quality of polygon edges at various
// angles, inscribed in a circle of diameter 512. It may be that a higher value
// would still produce acceptable quality, but 512 seems to work.
const FPM_THRESHOLD: usize = 512;

impl Rasterizer {
    /// NewRasterizer returns a new Rasterizer whose rendered mask image is bounded
    /// by the given width and height.
    pub fn new(w: usize, h: usize) -> Self {
        Self {
            size: [w, h],
            first: [0.0, 0.0],
            pen: [0.0, 0.0],
            draw_op: Op::Over,
            use_fpm: w > FPM_THRESHOLD || h > FPM_THRESHOLD,
            buf: SimdVec::new(w * h),
        }
    }

    /// Reset resets a Rasterizer as if it was just returned by NewRasterizer.
    pub fn reset(&mut self, w: usize, h: usize, op: Op) {
        self.size = [w, h];
        self.first = [0.0, 0.0];
        self.pen = [0.0, 0.0];
        self.draw_op = op;
        self.use_fpm = w > FPM_THRESHOLD || h > FPM_THRESHOLD;
        self.buf.recycle(w * h);
    }

    pub fn clear(&mut self) {
        let [w, h] = self.size;
        self.reset(w, h, Op::Over);
    }

    /// Returns the width and height passed to NewRasterizer or Reset.
    pub fn size(&self) -> [usize; 2] { self.size }

    pub fn as_mask_f32(&self) -> &[f32] { self.buf.as_slice_f32() }
    pub fn as_mask_u32(&self) -> &[u32] { self.buf.as_slice_u32() }

    /*
    // Bounds returns the rectangle from (0, 0) to the width and height passed to
    // NewRasterizer or Reset.
    fn (z *Rasterizer) Bounds() image.Rectangle {
        return image.Rectangle{Max: z.size}
    }
    */

    /// Returns the location of the path-drawing pen: the last argument to the most recent XxxTo call.
    pub fn pen(&self) -> [f32; 2] { self.pen }

    /// Closes the current path.
    pub fn close_path(&mut self) {
        self.line_to(self.first[0], self.first[1])
    }

    /// Starts a new path and moves the pen to (ax, ay).
    ///
    /// The coordinates are allowed to be out of the Rasterizer's bounds.
    pub fn move_to(&mut self, ax: f32, ay: f32) {
        self.first = [ax, ay];
        self.pen = [ax, ay];
    }

    /// Adds a line segment, from the pen to (bx, by), and moves the pen to (bx, by).
    ///
    /// The coordinates are allowed to be out of the Rasterizer's bounds.
    pub fn line_to(&mut self, bx: f32, by: f32) {
        if self.use_fpm {
            self.floating_line_to(bx, by)
        } else {
            self.fixed_line_to(bx, by)
        }
    }

    /// Adds a quadratic Bézier segment, from the pen via (bx, by) to (cx, cy), and moves the pen to (cx, cy).
    ///
    /// The coordinates are allowed to be out of the Rasterizer's bounds.
    pub fn quad_to(&mut self, bx: f32, by: f32, cx: f32, cy: f32) {
        let [ax, ay] = self.pen;
        let devsq = dev_squared(ax, ay, bx, by, cx, cy);

        if devsq >= 0.333 {
            const TOL: f64 = 3f64;
            let n = 1 + (TOL * devsq as f64).sqrt().sqrt() as isize;
            let (mut t, n_inv) = (0.0, 1.0 / n as f32);
            for _ in 0..n-1 {
                t += n_inv;
                let (abx, aby) = lerp(t, ax, ay, bx, by);
                let (bcx, bcy) = lerp(t, bx, by, cx, cy);
                let (bx, by) = lerp(t, abx, aby, bcx, bcy);
                self.line_to(bx, by);
            }
        }

        self.line_to(cx, cy);
    }

    /// Adds a cubic Bézier segment,
    /// from the pen via (bx, by) and (cx, cy) to (dx, dy), and moves the pen to (dx, dy).
    ///
    /// The coordinates are allowed to be out of the Rasterizer's bounds.
    pub fn cube_to(&mut self, bx: f32, by: f32, cx: f32, cy: f32, dx: f32, dy: f32) {
        let [ax, ay] = self.pen;
        let devsq = dev_squared(ax, ay, bx, by, dx, dy);
        let devsq_alt = dev_squared(ax, ay, cx, cy, dx, dy);
        let devsq = if devsq < devsq_alt { devsq_alt } else { devsq };

        if devsq >= 0.333 {
            const TOL: f64 = 3f64;
            let n = 1 + (TOL * devsq as f64).sqrt().sqrt() as isize;
            let (mut t, n_inv) = (0.0, 1.0 / n as f32);
            for _ in 0..n-1 {
                t += n_inv;
                let (abx, aby) = lerp(t, ax, ay, bx, by);
                let (bcx, bcy) = lerp(t, bx, by, cx, cy);
                let (cdx, cdy) = lerp(t, cx, cy, dx, dy);
                let (abcx, abcy) = lerp(t, abx, aby, bcx, bcy);
                let (bcdx, bcdy) = lerp(t, bcx, bcy, cdx, cdy);
                let (bx, by) = lerp(t, abcx, abcy, bcdx, bcdy);
                self.line_to(bx, by)
            }
        }
        self.line_to(dx, dy);
    }

    /*
    /// Draw implements the Drawer interface from the standard library's image/draw
    /// package.
    ///
    /// The vector paths previously added via the XxxTo calls become the mask for
    /// drawing src onto dst.
    pub fn (z *Rasterizer) Draw(dst draw.Image, r image.Rectangle, src image.Image, sp image.Point) {
        // TODO: adjust r and sp (and mp?) if src.Bounds() doesn't contain
        // r.Add(sp.Sub(r.Min)).

        if src, ok := src.(*image.Uniform); ok {
            srcR, srcG, srcB, srcA := src.RGBA()
            switch dst := dst.(type) {
            case *image.Alpha:
                // Fast path for glyph rendering.
                if srcA == 0xffff {
                    if z.DrawOp == draw.Over {
                        z.rasterizeDstAlphaSrcOpaqueOpOver(dst, r)
                    } else {
                        z.rasterizeDstAlphaSrcOpaqueOpSrc(dst, r)
                    }
                    return
                }
            case *image.RGBA:
                if z.DrawOp == draw.Over {
                    z.rasterizeDstRGBASrcUniformOpOver(dst, r, srcR, srcG, srcB, srcA)
                } else {
                    z.rasterizeDstRGBASrcUniformOpSrc(dst, r, srcR, srcG, srcB, srcA)
                }
                return
            }
        }

        if z.DrawOp == draw.Over {
            z.rasterizeOpOver(dst, r, src, sp)
        } else {
            z.rasterizeOpSrc(dst, r, src, sp)
        }
    }
    */

    fn accumulate_mask(&mut self) {
        let simd = false;
        if simd {
            unimplemented!("SIMD version")
        } else {
            if self.use_fpm {
                self.floating_accumulate_mask()
            } else {
                self.fixed_accumulate_mask()
            }
        }
    }

    /*
    fn (z *Rasterizer) rasterizeDstAlphaSrcOpaqueOpOver(dst *image.Alpha, r image.Rectangle) {
        // TODO: non-zero vs even-odd winding?
        if r == dst.Bounds() && r == z.Bounds() {
            // We bypass the z.accumulateMask step and convert straight from
            // z.bufF32 or z.bufU32 to dst.Pix.
            if z.useFloatingPointMath {
                if haveFloatingAccumulateSIMD {
                    floatingAccumulateOpOverSIMD(dst.Pix, z.bufF32)
                } else {
                    floatingAccumulateOpOver(dst.Pix, z.bufF32)
                }
            } else {
                if haveFixedAccumulateSIMD {
                    fixedAccumulateOpOverSIMD(dst.Pix, z.bufU32)
                } else {
                    fixedAccumulateOpOver(dst.Pix, z.bufU32)
                }
            }
            return
        }

        z.accumulateMask()
        pix := dst.Pix[dst.PixOffset(r.Min.X, r.Min.Y):]
        for y, y1 := 0, r.Max.Y-r.Min.Y; y < y1; y++ {
            for x, x1 := 0, r.Max.X-r.Min.X; x < x1; x++ {
                ma := z.bufU32[y*z.size.X+x]
                i := y*dst.Stride + x

                // This formula is like rasterizeOpOver's, simplified for the
                // concrete dst type and opaque src assumption.
                a := 0xffff - ma
                pix[i] = uint8((uint32(pix[i])*0x101*a/0xffff + ma) >> 8)
            }
        }
    }

    fn (z *Rasterizer) rasterizeDstAlphaSrcOpaqueOpSrc(dst *image.Alpha, r image.Rectangle) {
        // TODO: non-zero vs even-odd winding?
        if r == dst.Bounds() && r == z.Bounds() {
            // We bypass the z.accumulateMask step and convert straight from
            // z.bufF32 or z.bufU32 to dst.Pix.
            if z.useFloatingPointMath {
                if haveFloatingAccumulateSIMD {
                    floatingAccumulateOpSrcSIMD(dst.Pix, z.bufF32)
                } else {
                    floatingAccumulateOpSrc(dst.Pix, z.bufF32)
                }
            } else {
                if haveFixedAccumulateSIMD {
                    fixedAccumulateOpSrcSIMD(dst.Pix, z.bufU32)
                } else {
                    fixedAccumulateOpSrc(dst.Pix, z.bufU32)
                }
            }
            return
        }

        z.accumulateMask()
        pix := dst.Pix[dst.PixOffset(r.Min.X, r.Min.Y):]
        for y, y1 := 0, r.Max.Y-r.Min.Y; y < y1; y++ {
            for x, x1 := 0, r.Max.X-r.Min.X; x < x1; x++ {
                ma := z.bufU32[y*z.size.X+x]

                // This formula is like rasterizeOpSrc's, simplified for the
                // concrete dst type and opaque src assumption.
                pix[y*dst.Stride+x] = uint8(ma >> 8)
            }
        }
    }
    */

    pub fn rgba_uniform_over(&mut self, dst: &mut RGBA, r: Rectangle, color: [u32; 4]) {
        self.accumulate_mask();

        let [sr, sg, sb, sa] = color;
        let idx = dst.pix_offset(r.min.x, r.min.y);
        let pix = &mut dst.pix[idx as usize..];

        let x1 = r.max.x-r.min.x;
        let y1 = r.max.y-r.min.y;
        for y in 0..y1 {
            for x in 0..x1 {
                let idx = y * self.size[0] as isize + x;
                let ma = self.buf.as_u32()[idx as usize];

                // This formula is like rasterizeOpOver's, simplified for the
                // concrete dst type and uniform src assumption.
                let a = 0xffff - (sa * ma / 0xffff);
                let i = (y * dst.stride + 4 * x) as usize;
                pix[i+0] = ((((pix[i+0] as u32) * 0x101 * a + sr * ma) / 0xffff) >> 8) as u8;
                pix[i+1] = ((((pix[i+1] as u32) * 0x101 * a + sg * ma) / 0xffff) >> 8) as u8;
                pix[i+2] = ((((pix[i+2] as u32) * 0x101 * a + sb * ma) / 0xffff) >> 8) as u8;
                pix[i+3] = ((((pix[i+3] as u32) * 0x101 * a + sa * ma) / 0xffff) >> 8) as u8;
            }
        }
    }

    pub fn rgba_uniform_src(&mut self, dst: &mut RGBA, r: Rectangle, color: [u32; 4]) {
        self.accumulate_mask();
        let [sr, sg, sb, sa] = color;
        let idx = dst.pix_offset(r.min.x, r.min.y);
        let pix = &mut dst.pix[idx as usize..];

        let x1 = r.max.x-r.min.x;
        let y1 = r.max.y-r.min.y;
        for y in 0..y1 {
            for x in 0..x1 {
                let idx = y * self.size[0] as isize + x;
                let ma = self.buf.as_u32()[idx as usize];

                // This formula is like rasterizeOpSrc's, simplified for the
                // concrete dst type and uniform src assumption.
                let i = (y * dst.stride + 4 * x) as usize;
                pix[i+0] = ((sr * ma / 0xffff) >> 8) as u8;
                pix[i+1] = ((sg * ma / 0xffff) >> 8) as u8;
                pix[i+2] = ((sb * ma / 0xffff) >> 8) as u8;
                pix[i+3] = ((sa * ma / 0xffff) >> 8) as u8;
            }
        }
    }

    /*
    fn (z *Rasterizer) rasterizeOpOver(dst draw.Image, r image.Rectangle, src image.Image, sp image.Point) {
        z.accumulateMask()
        out := color.RGBA64{}
        outc := color.Color(&out)
        for y, y1 := 0, r.Max.Y-r.Min.Y; y < y1; y++ {
            for x, x1 := 0, r.Max.X-r.Min.X; x < x1; x++ {
                sr, sg, sb, sa := src.At(sp.X+x, sp.Y+y).RGBA()
                ma := z.bufU32[y*z.size.X+x]

                // This algorithm comes from the standard library's image/draw
                // package.
                dr, dg, db, da := dst.At(r.Min.X+x, r.Min.Y+y).RGBA()
                a := 0xffff - (sa * ma / 0xffff)
                out.R = uint16((dr*a + sr*ma) / 0xffff)
                out.G = uint16((dg*a + sg*ma) / 0xffff)
                out.B = uint16((db*a + sb*ma) / 0xffff)
                out.A = uint16((da*a + sa*ma) / 0xffff)

                dst.Set(r.Min.X+x, r.Min.Y+y, outc)
            }
        }
    }

    fn (z *Rasterizer) rasterizeOpSrc(dst draw.Image, r image.Rectangle, src image.Image, sp image.Point) {
        z.accumulateMask()
        out := color.RGBA64{}
        outc := color.Color(&out)
        for y, y1 := 0, r.Max.Y-r.Min.Y; y < y1; y++ {
            for x, x1 := 0, r.Max.X-r.Min.X; x < x1; x++ {
                sr, sg, sb, sa := src.At(sp.X+x, sp.Y+y).RGBA()
                ma := z.bufU32[y*z.size.X+x]

                // This algorithm comes from the standard library's image/draw
                // package.
                out.R = uint16(sr * ma / 0xffff)
                out.G = uint16(sg * ma / 0xffff)
                out.B = uint16(sb * ma / 0xffff)
                out.A = uint16(sa * ma / 0xffff)

                dst.Set(r.Min.X+x, r.Min.Y+y, outc)
            }
        }
    }
    */

}
