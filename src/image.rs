#[derive(Clone, Copy)]
pub struct Point {
    pub x: isize,
    pub y: isize,
}

#[derive(Clone, Copy)]
pub struct Rectangle {
    pub min: Point,
    pub max: Point,
}

impl Rectangle {
    pub fn from_size(w: isize, h: isize) -> Self {
        Self {
            min: Point { x: 0, y: 0 },
            max: Point { x: w, y: h },
        }
    }

    pub fn dx(&self) -> isize { self.max.x - self.min.x }
    pub fn dy(&self) -> isize { self.max.y - self.min.y }
}

// In reports whether p is in r.
pub fn in_rect(p: &Point, r: &Rectangle) -> bool {
    r.min.x <= p.x && p.x < r.max.x &&
    r.min.y <= p.y && p.y < r.max.y
}

// RGBA is an in-memory image whose At method returns color.RGBA values.
pub struct RGBA<'a> {
    /// Pix holds the image's pixels, in R, G, B, A order.
    /// The pixel at (x, y) starts at
    /// Pix[(y-Rect.Min.Y)*Stride + (x-Rect.Min.X)*4].
    pub pix: &'a mut [u8],
    /// Stride is the Pix stride (in bytes) between vertically adjacent pixels.
    pub stride: isize,
    /// Rect is the image's bounds.
    pub rect: Rectangle,
}

impl<'a> RGBA<'a> {
    // NewRGBA returns a new RGBA image with the given bounds.
    pub fn new(buf: &'a mut Vec<u8>, r: Rectangle) -> Self {
        let (w, h) = (r.dx(), r.dy());
        buf.clear();
        buf.resize((4*w*h) as usize, 0);
        Self {
            pix: &mut buf[..],
            stride: 4 * w,
            rect: r,
        }
    }

    pub fn from_buf32(buf: &'a mut [u32], r: Rectangle) -> Self {
        let (w, h) = (r.dx(), r.dy());
        let pix = unsafe {
            let data = buf.as_mut_ptr() as *mut u8;
            let len = buf.len() * 4;
            std::slice::from_raw_parts_mut(data, len)
        };
        Self {
            pix,
            stride: 4 * w,
            rect: r,
        }
    }

    pub fn bounds(&self) -> Rectangle { self.rect }

    pub fn at(&self, x: isize, y: isize) -> [u8; 4] {
        if !in_rect(&Point{x, y}, &self.rect) {
            [0; 4]
        } else {
            let i = self.pix_offset(x, y) as usize;
            [
                self.pix[i+0],
                self.pix[i+1],
                self.pix[i+2],
                self.pix[i+3],
            ]
        }
    }

    /// Returns the index of the first element of `pix`
    /// that corresponds to the pixel at (x, y).
    pub fn pix_offset(&self, x: isize, y: isize) -> isize {
        (y-self.rect.min.y) * self.stride + (x-self.rect.min.x) * 4
    }

    /*
    fn (p *RGBA) Set(x, y int, c color.Color) {
        if !(Point{x, y}.In(p.Rect)) {
            return
        }
        i := p.PixOffset(x, y)
        c1 := color.RGBAModel.Convert(c).(color.RGBA)
        p.Pix[i+0] = c1.R
        p.Pix[i+1] = c1.G
        p.Pix[i+2] = c1.B
        p.Pix[i+3] = c1.A
    }

    fn (p *RGBA) SetRGBA(x, y int, c color.RGBA) {
        if !(Point{x, y}.In(p.Rect)) {
            return
        }
        i := p.PixOffset(x, y)
        p.Pix[i+0] = c.R
        p.Pix[i+1] = c.G
        p.Pix[i+2] = c.B
        p.Pix[i+3] = c.A
    }

    // SubImage returns an image representing the portion of the image p visible
    // through r. The returned value shares pixels with the original image.
    func (p *RGBA) SubImage(r Rectangle) Image {
        r = r.Intersect(p.Rect)
        // If r1 and r2 are Rectangles, r1.Intersect(r2) is not guaranteed to be inside
        // either r1 or r2 if the intersection is empty. Without explicitly checking for
        // this, the Pix[i:] expression below can panic.
        if r.Empty() {
            return &RGBA{}
        }
        i := p.PixOffset(r.Min.X, r.Min.Y)
        return &RGBA{
            Pix:    p.Pix[i:],
            Stride: p.Stride,
            Rect:   r,
        }
    }

    // Opaque scans the entire image and reports whether it is fully opaque.
    func (p *RGBA) Opaque() bool {
        if p.Rect.Empty() {
            return true
        }
        i0, i1 := 3, p.Rect.Dx()*4
        for y := p.Rect.Min.Y; y < p.Rect.Max.Y; y++ {
            for i := i0; i < i1; i += 4 {
                if p.Pix[i] != 0xff {
                    return false
                }
            }
            i0 += p.Stride
            i1 += p.Stride
        }
        return true
    }
    */
}
