//extern crate minifb;
#![feature(int_to_from_bytes)]

use kiss2d::{Canvas, Font, Key, MouseMode, meter::Meter};
use kiss2d::clrs::*;

use std::time::{Instant, Duration};

const WIDTH: usize = 1280;
const HEIGHT: usize = 720;

static TITLE: &str = "Noise Test - Press ESC to exit";
static FONT_DATA: &[u8] = include_bytes!("Roboto-Regular.ttf");

struct Noise {
    seed: u32,
}

impl Noise {
    fn new() -> Self {
        Self { seed: 0xBEEF }
    }
    fn take(&mut self) -> u32 {
        let mut noise = self.seed;
        noise >>= 3;
        noise ^= self.seed;
        let carry = noise & 1;
        self.seed >>= 1;
        self.seed |= carry << 30;
        noise >> 1
    }
}

fn main() -> minifb::Result<()> {
    let mut canvas = Canvas::new(TITLE, WIDTH, HEIGHT)?;

    let font = Font::from_bytes(FONT_DATA).expect("Error constructing Font");

    let mut time = Instant::now();
    let mut frame = 0;
    let mut text = String::new();

    let mut meter = Meter::new();

    let mut noise = Noise::new();
    while canvas.is_open() && !canvas.is_keydown(Key::Escape) {
        let elapsed = time.elapsed();
        frame += 1;
        if elapsed >= Duration::from_secs(1) {
            let n = duration_to_secs(elapsed) / frame as f32;
            text = format!("FPS: {}, ms: {:?}", frame, secs_to_duration(n));
            time += elapsed;
            frame = 0;
        }

        for i in canvas.buffer().iter_mut() {
            let n = noise.take() & 0b01_1111;
            *i = n << 16 | n << 8 | n;
        }

        meter.render(&mut canvas, 0, 0);

        canvas.text(&font, 18.0, (70.0, 0.0), WHITE, &text);
        canvas.circle((80, 80), 30, MAROON);

        for i in 0..400 {
            canvas.line((400, i * 2), (400 - i, 200 + i % 10), AQUA);
        }

        canvas.udpate();
        if let Some(mouse) = canvas.window().get_mouse_pos(MouseMode::Clamp) {
            let mouse = (mouse.0 as isize, mouse.1 as isize);
            canvas.line((0, 0), mouse, RED);
        }

        canvas.keys(|t| match t {
            Key::W => println!("holding w!"),
            Key::T => println!("holding t!"),
            _ => (),
        });

        canvas.redraw()?;
    }

    Ok(())
}

pub fn duration_to_secs(duration: Duration) -> f32 {
    duration.as_secs() as f32 + (duration.subsec_nanos() as f32 / 1.0e9)
}

pub fn secs_to_duration(secs: f32) -> Duration {
    Duration::new(secs as u64, ((secs % 1.0) * 1.0e9) as u32)
}
