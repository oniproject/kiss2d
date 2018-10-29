fn ipart(x: f64) -> f64 { x.floor() }
fn round(x: f64) -> f64 { ipart(x + 0.5) }
fn fpart(x: f64) -> f64 { x - ipart(x) }
fn rfpart(x: f64) -> f64 { 1.0 - fpart(x) }

pub fn clipped_aaline<F>(start: (isize, isize), end: (isize, isize), size: (isize, isize), plot: F)
    where F: FnMut(isize, isize, f64)
{
    clipped(start.0, start.1, end.0, end.1, size.0, size.1, |x1, y1, x2, y2| {
        aaline(x1, y1, x2, y2, plot)
    })
}

// Sutherland-Cohen
// see: https://dos.gamebub.com/cpp_algorithms.php#lineclip
pub fn clipped<F>(mut x1: isize, mut y1: isize, mut x2: isize, mut y2: isize, w: isize, h: isize, line: F)
    where F: FnOnce(isize, isize, isize, isize)
{
    macro abrl($x: expr, $y: expr, $w: expr, $h: expr) {
        $x < 0 || $x > $w || $y < 0 || $y > $h
    }

    #[doc(hidden)]
    macro intersection($x1: expr, $y1: expr, $x2: expr, $y2: expr, $w: expr, $h: expr) {{
        if $y1 > $h {
            $x1 += ($x2 - $x1) * ($h - $y1) / ($y2 - $y1);
            $y1 = $h;
        } else if $y1 < 0 {
            $x1 += ($x2 - $x1) * (0 - $y1) / ($y2 - $y1);
            $y1 = 0;
        }
        if $x1 > $w {
            $y1 += ($y2 - $y1) * ($w - $x1) / ($x2 - $x1);
            $x1 = $w;
        } else if $x1 < 0 {
            $y1 += ($y2 - $y1) * (0 - $x1) / ($x2 - $x1);
            $x1 = 0;
        }
    }}

    let p1 = abrl!(x1, y1, w, h);
    let p2 = abrl!(x2, y2, w, h);
    if !p1 && !p2 {
        line(x1, y1, x2, y2);
    } else if p1 && p2 {
        return;
    } else {
        // clipping
        intersection!(x1, y1, x2, y2, w, h);
        intersection!(x2, y2, x1, y1, w, h);
        let p1 = abrl!(x1, y1, w, h);
        let p2 = abrl!(x2, y2, w, h);
        if !p1 && !p2 {
            line(x1, y1, x2, y2);
        }
    }
}

pub fn aaline<F>(x1: isize, y1: isize, x2: isize, y2: isize, mut plot: F)
    where F: FnMut(isize, isize, f64)
{
    if x1.abs() > 90000 { return }
    if x2.abs() > 90000 { return }
    if y1.abs() > 90000 { return }
    if y2.abs() > 90000 { return }

    let (mut x1, mut y1) = (x1 as f64, y1 as f64);
    let (mut x2, mut y2) = (x2 as f64, y2 as f64);
    let dx = x2 - x1;
    let dy = y2 - y1;

    if dx.abs() > dy.abs() {
        if x2 < x1 {
            std::mem::swap(&mut x1, &mut x2);
            std::mem::swap(&mut y1, &mut y2);
        }

        let gradient = dy / dx;
        let xend = round(x1) as f64;
        let yend = y1 + gradient * (xend - x1);
        let xgap = rfpart(x1 + 0.5);

        let xpxl1 = xend as isize;
        let ypxl1 = ipart(yend) as isize;

        // Add the first endpoint
        plot(xpxl1, ypxl1, rfpart(yend) * xgap);
        plot(xpxl1, ypxl1 + 1, fpart(yend) * xgap);

        let mut intery = yend + gradient;

        let xend = round(x2) as f64;
        let yend = y2 + gradient * (xend - x2);
        let xgap = fpart(x2 + 0.5);

        let xpxl2 = xend as isize;
        let ypxl2 = ipart(yend) as isize;

        // Add the second endpoint
        plot(xpxl2, ypxl2, rfpart(yend) * xgap);
        plot(xpxl2, ypxl2 + 1, fpart(yend) * xgap);

        // Add all the points between the endpoints
        for x in (xpxl1 + 1)..=(xpxl2 - 1) {
            let y = ipart(intery) as isize;
            plot(x, y + 0, rfpart(intery));
            plot(x, y + 1, fpart(intery));
            intery += gradient;
        }
    } else {
        if y2 < y1 {
            std::mem::swap(&mut x1, &mut x2);
            std::mem::swap(&mut y1, &mut y2);
        }

        let gradient = dx / dy;
        let yend = round(y1) as f64;
        let xend = x1 + gradient * (yend - y1);
        let ygap = rfpart(y1 + 0.5);

        let ypxl1 = yend as isize;
        let xpxl1 = ipart(xend) as isize;

        // Add the first endpoint
        plot(xpxl1, ypxl1, rfpart(xend) * ygap);
        plot(xpxl1, ypxl1 + 1, fpart(xend) * ygap);

        let mut interx = xend + gradient;

        let yend = round(y2) as f64;
        let xend = x2 + gradient * (yend - y2);
        let ygap = fpart(y2 + 0.5);

        let ypxl2 = yend as isize;
        let xpxl2 = ipart(xend) as isize;

        // Add the second endpoint
        plot(xpxl2, ypxl2, rfpart(xend) * ygap);
        plot(xpxl2, ypxl2 + 1, fpart(xend) * ygap);

        // Add all the points between the endpoints
        for y in (ypxl1 + 1)..=(ypxl2 - 1) {
            let x = ipart(interx) as isize;
            plot(x + 0, y, rfpart(interx));
            plot(x + 1, y, fpart(interx));
            interx += gradient;
        }
    }
}
