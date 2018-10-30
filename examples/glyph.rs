use kiss2d::{Canvas, Key, meter::Meter};
use kiss2d::clrs::*;

static TITLE: &str = "Glyph Test - Press ESC to exit";

const GLYPH_W: usize = 893;
const GLYPH_H: usize = 1122;
const WIDTH: usize = GLYPH_W;
const HEIGHT: usize = GLYPH_H;

enum C {
    MoveTo(f32, f32),
    LineTo(f32, f32),
    QuadTo(f32, f32, f32, f32),
}

// Is the 'a' glyph from the Roboto Regular font, translated so that its top left corner is (0, 0).
static GLYPH_DATA: &[C] = &[
    C::MoveTo(699., 1102.),
    C::QuadTo(683., 1070., 673.,  988.),
    C::QuadTo(544., 1122., 365., 1122.),
    C::QuadTo(205., 1122., 102.5,1031.5),
    C::QuadTo(  0.,  941.,   0. , 802.),
    C::QuadTo(  0.,  633., 128.5, 539.5),
    C::QuadTo(257.,  446., 490.,  446.),
    C::LineTo(670.,  446.),
    C::LineTo(670.,  361.),
    C::QuadTo(670.,  264., 612., 206.5),
    C::QuadTo(554.,  149., 441., 149.),
    C::QuadTo(342.,  149., 275., 199.),
    C::QuadTo(208.,  249., 208., 320.),
    C::LineTo( 22.,  320.),
    C::QuadTo( 22.,  239.,  79.5,163.5),
    C::QuadTo(137.,   88., 235.5, 44.),
    C::QuadTo(334.,    0., 452.,   0.),
    C::QuadTo(639.,    0., 745.,  93.5),
    C::QuadTo(851.,  187., 855., 351.),
    C::LineTo(855.,  849.),
    C::QuadTo(855.,  998., 893.,1086.),
    C::LineTo(893., 1102.),
    C::LineTo(699., 1102.),
    C::MoveTo(392.,  961.),
    C::QuadTo(479.,  961., 557., 916.),
    C::QuadTo(635.,  871., 670., 799.),
    C::LineTo(670.,  577.),
    C::LineTo(525.,  577.),
    C::QuadTo(185.,  577., 185., 776.),
    C::QuadTo(185.,  863., 243., 912.),
    C::QuadTo(301.,  961., 392., 961.),
];

fn main() -> minifb::Result<()> {
    let mut canvas = Canvas::new(TITLE, WIDTH, HEIGHT)?;
    let mut rs = kiss2d::vg::Rasterizer::new(GLYPH_W, GLYPH_H);
    let mut meter = Meter::new();
    while canvas.is_open() && !canvas.is_keydown(Key::Escape) {
        //canvas.clear();
        canvas.fill(NAVY);
        rs.clear();

        for c in GLYPH_DATA {
            match *c {
                C::MoveTo(px, py)         => rs.move_to(px, py),
                C::LineTo(px, py)         => rs.line_to(px, py),
                C::QuadTo(px, py, qx, qy) => rs.quad_to(px, py, qx, qy),
            }
        }

        let mut dst = canvas.image_mut();
        let r = dst.rect;
        rs.rgba_uniform_over(&mut dst, r, [0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF]);

        meter.render(&mut canvas, 0, 0);
        canvas.redraw()?;
    }

    Ok(())
}
