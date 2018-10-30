pub mod raster_floating;
pub mod raster_fixed;
pub mod vector;

pub enum Op {
    Over,
    Src,
}

// Raster is a 2-D vector graphics rasterizer.
//
// The zero value is usable, in that it is a Rasterizer whose rendered mask
// image has zero width and zero height. Call Reset to change its bounds.
pub struct Rasterizer {
    // bufXxx are buffers of float32 or uint32 values, holding either the
    // individual or cumulative area values.
    //
    // We don't actually need both values at any given time, and to conserve
    // memory, the integration of the individual to the cumulative could modify
    // the buffer in place. In other words, we could use a single buffer, say
    // of type []uint32, and add some math.Float32bits and math.Float32frombits
    // calls to satisfy the compiler's type checking. As of Go 1.7, though,
    // there is a performance penalty between:
    //	bufF32[i] += x
    // and
    //	bufU32[i] = math.Float32bits(x + math.Float32frombits(bufU32[i]))
    //
    // See golang.org/issue/17220 for some discussion.

    buf: SimdVec,

    use_fpm: bool,

    size: [usize; 2],
    first: [f32; 2],
    pen: [f32; 2],

    // DrawOp is the operator used for the Draw method.
    //
    // The zero value is draw.Over.
    pub draw_op: Op,

    // TODO: an exported field equivalent to the mask point in the
    // draw.DrawMask function in the stdlib image/draw package?
}


#[repr(align(16))]
#[derive(Clone)]
struct SimdAlign([f32; 4]);

pub struct SimdVec {
    v: Vec<SimdAlign>
}

impl SimdVec {
    fn new(len: usize) -> Self {
        let len = len / 4 + ((len % 4) != 0) as usize;
        Self { v: vec![SimdAlign([0.0; 4]); len] }
    }

    fn recycle(&mut self, len: usize) {
        self.v.clear();
        let len = len / 4 + ((len % 4) != 0) as usize;
        self.v.resize(len, SimdAlign([0.0; 4]))
    }

    fn as_slice_f32(&self) -> &[f32] {
        unsafe {
            let data = self.v.as_ptr() as *const f32;
            let len = self.v.len() * 4;
            std::slice::from_raw_parts(data, len)
        }
    }

    fn as_slice_u32(&self) -> &[u32] {
        unsafe {
            let data = self.v.as_ptr() as *const u32;
            let len = self.v.len() * 4;
            std::slice::from_raw_parts(data, len)
        }
    }

    fn as_f32(&mut self) -> &mut [f32] {
        unsafe { self.u_f32() }
    }

    fn as_u32(&mut self) -> &mut [u32] {
        unsafe { self.u_u32() }
    }

    unsafe fn u_f32<'a>(&mut self) -> &'a mut [f32] {
        unsafe {
            let data = self.v.as_mut_ptr() as *mut f32;
            let len = self.v.len() * 4;
            std::slice::from_raw_parts_mut(data, len)
        }
    }

    unsafe fn u_u32<'a>(&mut self) -> &'a mut [u32] {
        unsafe {
            let data = self.v.as_mut_ptr() as *mut u32;
            let len = self.v.len() * 4;
            std::slice::from_raw_parts_mut(data, len)
        }
    }
}

#[inline(always)]
fn lerp(t: f32, px: f32, py: f32, qx: f32, qy: f32) -> (f32, f32) {
    (px + t * (qx - px), py + t * (qy - py))
}

#[inline(always)]
fn clamp(i: i32, width: i32) -> usize {
    if i < 0 {
        return 0
    }
    if i < width {
        return i as usize
    }
    width as usize
}

// devSquared returns a measure of how curvy the sequence (ax, ay) to (bx, by)
// to (cx, cy) is. It determines how many line segments will approximate a
// Bézier curve segment.
//
// http://lists.nongnu.org/archive/html/freetype-devel/2016-08/msg00080.html
// gives the rationale for this evenly spaced heuristic instead of a recursive
// de Casteljau approach:
//
// The reason for the subdivision by n is that I expect the "flatness"
// computation to be semi-expensive (it's done once rather than on each
// potential subdivision) and also because you'll often get fewer subdivisions.
// Taking a circular arc as a simplifying assumption (ie a spherical cow),
// where I get n, a recursive approach would get 2^⌈lg n⌉, which, if I haven't
// made any horrible mistakes, is expected to be 33% more in the limit.
#[inline(always)]
fn dev_squared(ax: f32, ay: f32, bx: f32, by: f32, cx: f32, cy: f32) -> f32{
    let devx = ax - 2.0*bx + cx;
    let devy = ay - 2.0*by + cy;
    devx*devx + devy*devy
}

enum PorterDuff {
    A,
    B,
    AoverB,
    BoverA,
    AinB,
    BinA,
    AoutB,
    BoutA,
    AatopB,
    BatopA,
    AxorB,
    Clear,
}

enum PD {
    Src,
    Over,
    In,
    Out,
    Atop,
    Xor,
}
