fn ipart(x: f64) -> f64 { x.floor() }
fn round(x: f64) -> f64 { ipart(x + 0.5) }
fn fpart(x: f64) -> f64 { x - ipart(x) }
fn rfpart(x: f64) -> f64 { 1.0 - fpart(x) }

pub fn aaline<F>(mut x1: f64, mut y1: f64, mut x2: f64, mut y2: f64, mut plot: F)
    where F: FnMut(isize, isize, f64)
{
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
            plot(x    , y, rfpart(interx));
            plot(x + 1, y, fpart(interx));
            interx += gradient;
        }
    }
}
