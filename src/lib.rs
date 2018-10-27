#![feature(int_to_from_bytes)]

pub use minifb;
pub use bresenham;
pub use rusttype;
pub use image;

pub mod meter;

use minifb::{Window};
use bresenham::{Point, Bresenham};
use rusttype::{point, Scale};
use image::{Bgra, Pixel};

pub use minifb::{Key, MouseMode};
pub use rusttype::Font;

pub struct Canvas {
    buffer: Vec<u32>,
    window: Window,
    size: (usize, usize),
}

impl Canvas {
    pub fn new(title: &str, width: usize, height: usize) -> minifb::Result<Self> {
        let buffer: Vec<u32> = vec![0; width * height];
        let window = Window::new(title, width, height, Default::default())?;

        Ok(Self { buffer, window, size: (width, height) })
    }

    pub fn window(&self) -> &Window { &self.window }
    pub fn buffer(&mut self) -> &mut [u32] { &mut self.buffer }
    pub fn size(&self) -> (usize, usize) { self.size }

    pub fn is_open(&self) -> bool { self.window.is_open() }
    pub fn is_keydown(&self, key: Key) -> bool { self.window.is_key_down(key) }

    pub fn keys<F: FnMut(Key)>(&self, f: F) {
        self.window.get_keys()
            .map(|mut keys| keys.drain(..).for_each(f));
    }

    pub fn udpate(&mut self) {
        self.window.update()
    }

    pub fn redraw(&mut self) -> minifb::Result<()> {
        self.window.update_with_buffer(&self.buffer)
    }

    pub fn clear(&mut self) {
        self.buffer.iter_mut().for_each(|i| *i = 0);
    }

    pub fn pixel_mut(&mut self, x: usize, y: usize) -> &mut u32 {
        let (w, h) = self.size();
        assert!(x < w && y < h, "{}x{}", x, y);
        let idx = x + y * w;
        unsafe { self.buffer.get_unchecked_mut(idx) }
    }

    pub fn pixel(&mut self, x: usize, y: usize, color: u32) {
        let (w, h) = self.size();
        if x < w && y < h {
            let idx = x + y * w;
            unsafe { *self.buffer.get_unchecked_mut(idx) = color; }
        }
    }

    pub fn line(&mut self, start: Point, end: Point, color: u32) {
        let (w, h) = self.size();
        let (w, h) = (w as isize, h as isize);
        for (x, y) in Bresenham::new(start, end) {
            if x >= 0 && x < w && y >= 0 && y < h {
                let idx = (x + y * w) as usize;
                unsafe { *self.buffer.get_unchecked_mut(idx) = color; }
            }
        }
    }

    pub fn hline(&mut self, x1: isize, x2: isize, y: isize, color: u32) {
        let (w, h) = self.size();
        let (w, h) = (w as isize, h as isize);

        if y < 0 || y >= h { return }
        let x1 = x1.max(0);
        let x2 = x2.min(w);

        for x in x1..x2 {
            let idx = (x + y * w) as usize;
            unsafe { *self.buffer.get_unchecked_mut(idx) = color; }
        }
    }

    pub fn vline(&mut self, x: isize, y1: isize, y2: isize, color: u32) {
        let (w, h) = self.size();
        let (w, h) = (w as isize, h as isize);

        if x < 0 || x >= w { return }
        let y1 = y1.max(0);
        let y2 = y2.min(h);

        for y in y1..y2 {
            let idx = (x + y * w) as usize;
            unsafe { *self.buffer.get_unchecked_mut(idx) = color; }
        }
    }

    pub fn text(&mut self, font: &Font, scale: f32, pos: (f32, f32), color: u32, text: &str) {
        let scale = Scale::uniform(scale);
        let v_metrics = font.v_metrics(scale);
        let glyphs: Vec<_> = font
            .layout(text, scale, point(pos.0, pos.1 + v_metrics.ascent))
            .collect();

        let (w, h) = self.size();

        for glyph in glyphs {
            if let Some(bbox) = glyph.pixel_bounding_box() {
                glyph.draw(|x, y, v| {
                    let x = (x + bbox.min.x as u32) as usize;
                    let y = (y + bbox.min.y as u32) as usize;
                    if v != 0.0 && x < w && y < h {
                        let idx = x + y * w;
                        let pixel = unsafe { self.buffer.get_unchecked_mut(idx) };

                        let mut src = Bgra { data: color.to_le_bytes() };
                        let mut dst = Bgra { data: pixel.to_le_bytes() };

                        src[3] = (v * 255.0) as u8;
                        dst[3] = 0xFF;

                        dst.blend(&src);

                        *pixel = u32::from_le_bytes(dst.data);
                    }
                });
            }
        }
    }

    pub fn circle(&mut self, pos: Point, radius: usize, color: u32) {
        const PI2: f32 = std::f32::consts::PI * 2.0;
        let nsamples = 16;
        let fsamples = nsamples as f32;

        let radius = radius as f32;
        let pos = (pos.0 as f32, pos.1 as f32);

        self.curve(color, true, (0..nsamples).map(|i| {
            let (ax, ay) = ((i as f32) / fsamples * PI2).sin_cos();
            let (ax, ay) = (pos.0 + ax * radius, pos.1 + ay * radius);
            (ax as isize, ay as isize)
        }));
    }

    pub fn curve<I: IntoIterator<Item=Point>>(&mut self, color: u32, loopped: bool, pts: I) {
        let mut pts = pts.into_iter();
        let first = if let Some(p) = pts.next() { p } else { return };

        let mut base = first;
        for p in pts {
            self.line(base, p, color);
            base = p;
        }

        if loopped {
            self.line(base, first, color);
        }
    }
}

pub mod clrs {
    #![allow(clippy::unreadable_literal)]

    // from http://clrs.cc/
    pub const NAVY: u32    = 0x001F3F;
    pub const BLUE: u32    = 0x0074D9;
    pub const AQUA: u32    = 0x7FDBFF;
    pub const TEAL: u32    = 0x39CCCC;
    pub const OLIVE: u32   = 0x3D9970;
    pub const GREEN: u32   = 0x2ECC40;
    pub const LIME: u32    = 0x01FF70;
    pub const YELLOW: u32  = 0xFFDC00;
    pub const ORANGE: u32  = 0xFF851B;
    pub const RED: u32     = 0xFF4136;
    pub const MAROON: u32  = 0x85144B;
    pub const FUCHSIA: u32 = 0xF012BE;
    pub const PURPLE: u32  = 0xB10DC9;
    pub const BLACK: u32   = 0x111111;
    pub const GRAY: u32    = 0xAAAAAA;
    pub const SILVER: u32  = 0xDDDDDD;
    pub const WHITE: u32   = 0xFFFFFF;
}
