use std::time::{Instant, Duration};

use super::Canvas;
use super::clrs::*;

pub fn duration_to_secs(duration: Duration) -> f32 {
    duration.as_secs() as f32 + (duration.subsec_nanos() as f32 / 1.0e9)
}

pub fn secs_to_duration(secs: f32) -> Duration {
    Duration::new(secs as u64, ((secs % 1.0) * 1.0e9) as u32)
}

const SIZE: usize = 64;

/// FPS Meter.
pub struct Meter {
    pub last_loop: Instant,
    pub history: [f32; SIZE],
}

impl Meter {
    pub fn new() -> Self {
        Self {
            last_loop: Instant::now(),
            history: [0.0; SIZE],
        }
    }

    pub fn render(&mut self, canvas: &mut Canvas, x: isize, y: isize) {
        let elapsed = self.last_loop.elapsed();
        self.last_loop += elapsed;

        self.history.rotate_left(1);
        self.history[SIZE - 1] = duration_to_secs(elapsed) * 1000.0;

        for (i, v) in self.history.iter().cloned().enumerate() {
            let x = x + i as isize;
            let v = v as isize;
            canvas.vline(x, y, y+v, LIME);
        }
    }
}
