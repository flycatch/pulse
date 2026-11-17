use std::collections::VecDeque;
use std::num::NonZeroU32;
use std::rc::Rc;

use font8x8::legacy::BASIC_LEGACY;
use softbuffer::{Context, Surface};
use tao::dpi::{LogicalSize, PhysicalPosition};
use tao::event_loop::EventLoopWindowTarget;
use tao::window::{Window, WindowBuilder};

use crate::format;
use crate::stats::{Monitor, Sample, HISTORY};

const W: u32 = 320;
const H: u32 = 270;

const BG: u32 = 0x00_1e_1e_24;
const FG: u32 = 0x00_e6_e6_e6;
const DIM: u32 = 0x00_8a_8a_94;
const DOWN: u32 = 0x00_4c_c9_6b;
const UP: u32 = 0x00_f0_9a_3c;
const GRID: u32 = 0x00_30_30_38;

/// Popup window. Dropping it frees the window and pixel surface.
pub struct Popup {
    window: Rc<Window>,
    surface: Surface<Rc<Window>, Rc<Window>>,
}

impl Popup {
    /// `anchor` is the tray icon rect (physical px) used to place the window.
    pub fn open<T>(
        target: &EventLoopWindowTarget<T>,
        anchor: Option<(PhysicalPosition<f64>, f64, f64)>,
    ) -> Option<Self> {
        let mut wb = WindowBuilder::new()
            .with_title("Pulse")
            .with_inner_size(LogicalSize::new(W, H))
            .with_decorations(false)
            .with_resizable(false)
            .with_always_on_top(true);
        if let Some((pos, iw, ih)) = anchor {
            // Placement assumes scale 1 for window size; close enough next to the tray.
            let x = pos.x + iw / 2.0 - f64::from(W) / 2.0;
            let y = if cfg!(target_os = "macos") { pos.y + ih } else { pos.y - f64::from(H) };
            wb = wb.with_position(PhysicalPosition::new(x.max(0.0), y.max(0.0)));
        }
        let window = Rc::new(wb.build(target).ok()?);
        let context = Context::new(window.clone()).ok()?;
        let surface = Surface::new(&context, window.clone()).ok()?;
        window.set_focus();
        Some(Self { window, surface })
    }

    pub fn request_redraw(&self) {
        self.window.request_redraw();
    }

    pub fn is(&self, id: tao::window::WindowId) -> bool {
        self.window.id() == id
    }

    pub fn draw(&mut self, mon: &Monitor, s: &Sample, bits: bool) {
        let size = self.window.inner_size();
        let (Some(w), Some(h)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
        else {
            return;
        };
        if self.surface.resize(w, h).is_err() {
            return;
        }
        let Ok(mut buf) = self.surface.buffer_mut() else {
            return;
        };
        let k = self.window.scale_factor().round().max(1.0) as i32;
        render(&mut buf, size.width as i32, size.height as i32, k, mon, s, bits);
        let _ = buf.present();
    }
}

/// Paint the whole popup into `buf` (`w`x`h` px, `k` = HiDPI scale).
fn render(buf: &mut [u32], w: i32, h: i32, k: i32, mon: &Monitor, s: &Sample, bits: bool) {
        let mut c = Canvas { buf, w, h };
        c.fill(0, 0, c.w, c.h, BG);

        let pad = 10 * k;
        let gw = c.w - 2 * pad;
        let gh = 50 * k;

        c.text(pad, 10 * k, &format!("DOWN {}", format::rate(s.down, bits)), FG, 2 * k);
        c.graph(pad, 32 * k, gw, gh, &mon.history_down, DOWN);
        c.text(pad, 92 * k, &format!("UP   {}", format::rate(s.up, bits)), FG, 2 * k);
        c.graph(pad, 114 * k, gw, gh, &mon.history_up, UP);

        c.text(
            pad,
            174 * k,
            &format!(
                "Total  D {}  U {}",
                bytes(mon.total_down),
                bytes(mon.total_up)
            ),
            DIM,
            k,
        );
        for (i, r) in s.ifaces.iter().take(6).enumerate() {
            let y = (192 + 12 * i as i32) * k;
            c.text(pad, y, &r.name, FG, k);
            c.text(
                pad + 90 * k,
                y,
                &format!("D {}  U {}", format::rate(r.down, bits), format::rate(r.up, bits)),
                DIM,
                k,
            );
        }
}

fn bytes(n: u64) -> String {
    let mut v = n as f64;
    let mut i = 0;
    let u = ["B", "KB", "MB", "GB", "TB"];
    while v >= 1000.0 && i < u.len() - 1 {
        v /= 1000.0;
        i += 1;
    }
    if i == 0 { format!("{n} B") } else { format!("{v:.1} {}", u[i]) }
}

struct Canvas<'a> {
    buf: &'a mut [u32],
    w: i32,
    h: i32,
}

impl Canvas<'_> {
    fn fill(&mut self, x: i32, y: i32, w: i32, h: i32, color: u32) {
        for yy in y.max(0)..(y + h).min(self.h) {
            let row = (yy * self.w) as usize;
            for xx in x.max(0)..(x + w).min(self.w) {
                self.buf[row + xx as usize] = color;
            }
        }
    }

    fn text(&mut self, x: i32, y: i32, s: &str, color: u32, scale: i32) {
        let mut cx = x;
        for ch in s.chars() {
            let idx = if (ch as usize) < 128 { ch as usize } else { b'?' as usize };
            let glyph = BASIC_LEGACY[idx];
            for (row, bits) in glyph.iter().enumerate() {
                for col in 0..8 {
                    if bits >> col & 1 == 1 {
                        self.fill(cx + col * scale, y + row as i32 * scale, scale, scale, color);
                    }
                }
            }
            cx += 8 * scale;
        }
    }

    /// Bar graph scaled to the max of the visible history.
    fn graph(&mut self, x: i32, y: i32, w: i32, h: i32, data: &VecDeque<f64>, color: u32) {
        self.fill(x, y, w, h, GRID);
        let max = data.iter().cloned().fold(1.0_f64, f64::max);
        let bar = (w / HISTORY as i32).max(1);
        let off = HISTORY - data.len();
        for (i, v) in data.iter().enumerate() {
            let bh = ((v / max) * f64::from(h)).round() as i32;
            let bx = x + (off + i) as i32 * bar;
            self.fill(bx, y + h - bh, bar - 1, bh, color);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Writes a PPM when `POPUP_PPM` is set, for eyeballing the layout.
    #[test]
    fn render_layout() {
        let mut mon = Monitor::new();
        for i in 0..40 {
            mon.history_down.push_back(f64::from(i * i) * 1000.0);
            mon.history_up.push_back(f64::from(i) * 5000.0);
        }
        mon.total_down = 123_456_789;
        let s = Sample {
            down: 1_560_000.0,
            up: 195_000.0,
            ifaces: vec![crate::stats::IfaceRate { name: "en0".into(), down: 1_560_000.0, up: 195_000.0 }],
            ..Sample::default()
        };
        let (w, h) = (W as i32, H as i32);
        let mut buf = vec![0u32; (w * h) as usize];
        render(&mut buf, w, h, 1, &mon, &s, false);
        assert!(buf.iter().any(|&p| p == DOWN) && buf.iter().any(|&p| p == UP));
        if let Ok(path) = std::env::var("POPUP_PPM") {
            let mut out = format!("P6\n{w} {h}\n255\n").into_bytes();
            for p in buf {
                out.extend([(p >> 16) as u8, (p >> 8) as u8, p as u8]);
            }
            std::fs::write(path, out).unwrap();
        }
    }
}
