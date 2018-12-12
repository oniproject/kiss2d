#![feature(int_to_from_bytes, decl_macro, non_ascii_idents)]

pub use minifb;
pub use rusttype;
//pub use image as ximage;
//pub extern crate image as ximage;

pub mod meter;
pub mod wu;
pub mod image;
pub mod vg;
pub mod clrs;
pub mod geom;

use minifb::{Window, MouseMode};
use rusttype::{point, Scale};

use self::image::{Rectangle, RGBA};

pub use minifb::{Key, MouseButton, CursorStyle};
pub use rusttype::Font;

pub type Point = (isize, isize);

pub struct Canvas {
    buffer: Vec<u32>,
    window: Window,
    size: (usize, usize),
}

impl std::ops::Deref for Canvas {
    type Target = [u32];
    fn deref(&self) -> &[u32] { &self.buffer }
}

impl std::ops::DerefMut for Canvas {
    fn deref_mut(&mut self) -> &mut [u32] { &mut self.buffer }
}

impl Canvas {
    pub fn new(title: &str, width: usize, height: usize) -> minifb::Result<Self> {
        let buffer: Vec<u32> = vec![0; width * height];
        let window = Window::new(title, width, height, Default::default())?;

        Ok(Self { buffer, window, size: (width, height) })
    }

    pub fn window(&self) -> &Window { &self.window }
    pub fn window_mut(&mut self) -> &mut Window { &mut self.window }
    pub fn buffer(&self) -> &[u32] { &self.buffer }
    pub fn buffer_mut(&mut self) -> &mut [u32] { &mut self.buffer }

    pub fn image_mut(&mut self) -> RGBA {
        let (w, h) = self.size;
        let r = Rectangle::from_size(w as isize, h as isize);
        RGBA::from_buf32(&mut self.buffer, r)
    }

    pub fn size(&self) -> (usize, usize) { self.size }

    pub fn is_open(&self) -> bool { self.window.is_open() }
    pub fn is_keydown(&self, key: Key) -> bool { self.window.is_key_down(key) }

    pub fn set_cursor_style(&mut self, cursor: CursorStyle) {
        self.window.set_cursor_style(cursor)
    }

    pub fn keys<F: FnMut(Key)>(&self, f: F) {
        self.window.get_keys()
            .map(|mut keys| keys.drain(..).for_each(f));
    }

    pub fn mouse_pos(&self) -> Option<(f32, f32)> {
        self.window.get_mouse_pos(MouseMode::Pass)
    }

    pub fn mouse_down(&self, button: MouseButton) -> bool {
        self.window.get_mouse_down(button)
    }

    pub fn mouse_wheel(&self) -> Option<(f32, f32)> {
        self.window.get_scroll_wheel()
    }

    pub fn udpate(&mut self) {
        self.window.update()
    }

    pub fn redraw(&mut self) -> minifb::Result<()> {
        self.window.update_with_buffer(&self.buffer)
    }

    pub fn clear(&mut self) {
        self.fill(0);
    }

    pub fn fill(&mut self, color: u32) {
        self.buffer.iter_mut().for_each(|i| *i = color);
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
        wu::clipped_aaline(start, end, (w, h), |x, y, v| {
            if x >= 0 && x < w && y >= 0 && y < h {
                let idx = (x + y * w) as usize;
                unsafe { self.blend(idx, color, v as f32) }
            }
        })
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
        let (w, h) = self.size();

        for (line, text) in text.lines().enumerate() {
            let point = point(pos.0, pos.1 + v_metrics.ascent * (line + 1) as f32);
            for glyph in font.layout(text, scale, point) {
                if let Some(bbox) = glyph.pixel_bounding_box() {
                    glyph.draw(|x, y, v| {
                        let x = (x + bbox.min.x as u32) as usize;
                        let y = (y + bbox.min.y as u32) as usize;
                        if v != 0.0 && x < w && y < h {
                            unsafe {
                                self.blend(x + y * w, color, v);
                            }
                        }
                    });
                }
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

    unsafe fn blend(&mut self, idx: usize, color: u32, alpha: f32) {
        // http://stackoverflow.com/questions/7438263/alpha-compositing-algorithm-blend-modes#answer-11163848
        const MAX_T: f32 = 255.0;

        let pixel = self.buffer.get_unchecked_mut(idx);

        let [db, dg, dr, _] = pixel.to_le_bytes();
        let [sb, sg, sr, _] = color.to_le_bytes();
        let (dr, dg, db) = (
            dr as f32 / MAX_T,
            dg as f32 / MAX_T,
            db as f32 / MAX_T,
        );
        let (sr, sg, sb) = (
            sr as f32 / MAX_T,
            sg as f32 / MAX_T,
            sb as f32 / MAX_T,
        );

        let inv_alpha = 1.0 - alpha;
        let (r, g, b) = (
            ((sr * alpha + dr * inv_alpha) * MAX_T) as u8,
            ((sg * alpha + dg * inv_alpha) * MAX_T) as u8,
            ((sb * alpha + db * inv_alpha) * MAX_T) as u8,
        );

        // Cast back to our initial type on return
        *pixel = u32::from_le_bytes([b, g, r, 0xFF]);
    }
}
