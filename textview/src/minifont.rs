fn is_box_char(c: char) -> bool {
    // Box Drawing & Block Elements
    (c >= 0x2500 as char && c <= 0x259f as char)
        || (c >= 0x25e2 as char && c <= 0x25e5 as char)
        || (c >= 0x1fb00 as char && c <= 0x1fbaf as char)
}

/// pixman data must have stride 1 mod 4
static mut HATCHING_PATTERN_LR_DATA: [u8; 16] = [
    0xff, 0x00, 0x00, 0x00, //
    0x00, 0xff, 0x00, 0x00, //
    0x00, 0x00, 0xff, 0x00, //
    0x00, 0x00, 0x00, 0xff, //
];
static mut HATCHING_PATTERN_RL_DATA: [u8; 16] = [
    0x00, 0x00, 0x00, 0xff, //
    0x00, 0x00, 0xff, 0x00, //
    0x00, 0xff, 0x00, 0x00, //
    0xff, 0x00, 0x00, 0x00, //
];
static mut CHECKERBOARD_PATTERN_DATA: [u8; 16] = [
    0xff, 0xff, 0x00, 0x00, //
    0xff, 0xff, 0x00, 0x00, //
    0x00, 0x00, 0xff, 0xff, //
    0x00, 0x00, 0xff, 0xff, //
];
static mut CHECKERBOARD_REVERSE_PATTERN_DATA: [u8; 16] = [
    0x00, 0x00, 0xff, 0xff, //
    0x00, 0x00, 0xff, 0xff, //
    0xff, 0xff, 0x00, 0x00, //
    0xff, 0xff, 0x00, 0x00, //
];

macro_rules! define_static_pattern_func {
    ($name:ident, $data:expr, $width:literal, $height:literal, $stride:literal) => {
        fn $name() -> &'static cairo::Pattern {
            static PATTERN: once_cell::sync::OnceCell<fragile::Fragile<cairo::SurfacePattern>> =
                once_cell::sync::OnceCell::new();
            PATTERN
                .get_or_init(|| {
                    let surface = cairo::ImageSurface::create_for_data(
                        unsafe { &mut $data },
                        cairo::Format::A8,
                        $width,
                        $height,
                        $stride,
                    )
                    .unwrap();
                    let pattern = cairo::SurfacePattern::create(&surface);
                    pattern.set_extend(cairo::Extend::Repeat);
                    pattern.set_filter(cairo::Filter::Nearest);
                    fragile::Fragile::new(pattern)
                })
                .get()
        }
    };
}

define_static_pattern_func!(
    create_hatching_pattern_lr,
    HATCHING_PATTERN_LR_DATA,
    4,
    4,
    4
);
define_static_pattern_func!(
    create_hatching_pattern_rl,
    HATCHING_PATTERN_RL_DATA,
    4,
    4,
    4
);
define_static_pattern_func!(
    create_checkerboard_pattern,
    CHECKERBOARD_PATTERN_DATA,
    4,
    4,
    4
);
define_static_pattern_func!(
    create_checkerboard_reverse_pattern,
    CHECKERBOARD_REVERSE_PATTERN_DATA,
    4,
    4,
    4
);

fn rectangle(
    cr: &cairo::Context,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    xdenom: i32,
    ydenom: i32,
    xb1: i32,
    yb1: i32,
    xb2: i32,
    yb2: i32,
) -> Result<(), cairo::Error> {
    let x1 = (w) * (xb1 as f64) / (xdenom as f64);
    let y1 = (h) * (yb1 as f64) / (ydenom as f64);
    let x2 = (w) * (xb2 as f64) / (xdenom as f64);
    let y2 = (h) * (yb2 as f64) / (ydenom as f64);
    cr.rectangle((x) + x1, (y) + y1, (x2 - x1).max(1.), (y2 - y1).max(1.));
    cr.fill()?;
    Ok(())
}

fn polygon(
    cr: &cairo::Context,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    xdenom: i32,
    ydenom: i32,
    cc: &[i8],
) -> Result<(), cairo::Error> {
    let mut x1 = (w) * (cc[0] as f64) / (xdenom as f64);
    let mut y1 = (h) * (cc[1] as f64) / (ydenom as f64);
    cr.move_to((x) + x1, (y) + y1);
    let mut i: usize = 2;
    while cc[i] != -1 {
        x1 = (w) * (cc[i] as f64) / (xdenom as f64);
        y1 = (h) * (cc[i + 1] as f64) / (ydenom as f64);
        cr.line_to((x) + x1, (y) + y1);
        i += 2;
    }
    cr.fill()?;
    Ok(())
}

fn pattern(
    cr: &cairo::Context,
    pattern: &cairo::Pattern,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), cairo::Error> {
    cr.push_group();
    cr.rectangle(x, y, width, height);
    cr.fill()?;
    cr.pop_group_to_source()?;
    cr.mask(pattern)?;
    Ok(())
}

use crate::box_drawing::DRAW_BOX_DRAWING_BITMAPS;

struct DrawingContext {}
impl DrawingContext {
    fn cairo(&self) -> &cairo::Context {
        todo!()
    }
    fn cell_width(&self) -> i32 {
        todo!()
    }
    fn cell_height(&self) -> i32 {
        todo!()
    }
}
struct RGB;

pub fn draw_box_char(
    context: &DrawingContext,
    c: char,
    attr: u32,
    fg: &RGB,
    x: i32,
    y: i32,
    font_width: i32,
    columns: i32,
    font_height: i32,
) -> Result<(), cairo::Error> {
    let mut width: i32;
    let mut height: i32;
    let mut xcenter: i32;
    let mut xright: i32;
    let mut ycenter: i32;
    let mut ybottom: i32;
    let mut upper_half: i32;
    let mut left_half: i32;
    let mut light_line_width: i32;
    let mut heavy_line_width: i32;
    let mut adjust: f32;
    let cr = context.cairo();

    cr.save()?;

    let c = c as u32;

    width = context.cell_width() * columns;
    height = context.cell_height();
    upper_half = height / 2;
    left_half = width / 2;

    /* Exclude the spacing for line width computation. */
    light_line_width = font_width / 5;
    light_line_width = light_line_width.max(1);

    if c >= 0x2550 && c <= 0x256c {
        heavy_line_width = 3 * light_line_width;
    } else {
        heavy_line_width = light_line_width + 2;
    }

    xcenter = x + left_half;
    ycenter = y + upper_half;
    xright = x + width;
    ybottom = y + height;

    /* Box Drawing */
    if c == 0x1fbaf {
        /* box drawings light horizontal with vertical stroke */
        rectangle(
            cr,
            (x + left_half - light_line_width / 2) as f64,
            y as f64,
            light_line_width as f64,
            height as f64,
            1,
            3,
            0,
            1,
            1,
            2,
        );
        c = 0x2500;
    }
    match c {
        0x2500 | /* box drawings light horizontal */
        0x2501 | /* box drawings heavy horizontal */
        0x2502 | /* box drawings light vertical */
        0x2503 | /* box drawings heavy vertical */
        0x250c | /* box drawings light down and right */
        0x250d | /* box drawings down light and right heavy */
        0x250e | /* box drawings down heavy and right light */
        0x250f | /* box drawings heavy down and right */
        0x2510 | /* box drawings light down and left */
        0x2511 | /* box drawings down light and left heavy */
        0x2512 | /* box drawings down heavy and left light */
        0x2513 | /* box drawings heavy down and left */
        0x2514 | /* box drawings light up and right */
        0x2515 | /* box drawings up light and right heavy */
        0x2516 | /* box drawings up heavy and right light */
        0x2517 | /* box drawings heavy up and right */
        0x2518 | /* box drawings light up and left */
        0x2519 | /* box drawings up light and left heavy */
        0x251a | /* box drawings up heavy and left light */
        0x251b | /* box drawings heavy up and left */
        0x251c | /* box drawings light vertical and right */
        0x251d | /* box drawings vertical light and right heavy */
        0x251e | /* box drawings up heavy and right down light */
        0x251f | /* box drawings down heavy and right up light */
        0x2520 | /* box drawings vertical heavy and right light */
        0x2521 | /* box drawings down light and right up heavy */
        0x2522 | /* box drawings up light and right down heavy */
        0x2523 | /* box drawings heavy vertical and right */
        0x2524 | /* box drawings light vertical and left */
        0x2525 | /* box drawings vertical light and left heavy */
        0x2526 | /* box drawings up heavy and left down light */
        0x2527 | /* box drawings down heavy and left up light */
        0x2528 | /* box drawings vertical heavy and left light */
        0x2529 | /* box drawings down light and left up heavy */
        0x252a | /* box drawings up light and left down heavy */
        0x252b | /* box drawings heavy vertical and left */
        0x252c | /* box drawings light down and horizontal */
        0x252d | /* box drawings left heavy and right down light */
        0x252e | /* box drawings right heavy and left down light */
        0x252f | /* box drawings down light and horizontal heavy */
        0x2530 | /* box drawings down heavy and horizontal light */
        0x2531 | /* box drawings right light and left down heavy */
        0x2532 | /* box drawings left light and right down heavy */
        0x2533 | /* box drawings heavy down and horizontal */
        0x2534 | /* box drawings light up and horizontal */
        0x2535 | /* box drawings left heavy and right up light */
        0x2536 | /* box drawings right heavy and left up light */
        0x2537 | /* box drawings up light and horizontal heavy */
        0x2538 | /* box drawings up heavy and horizontal light */
        0x2539 | /* box drawings right light and left up heavy */
        0x253a | /* box drawings left light and right up heavy */
        0x253b | /* box drawings heavy up and horizontal */
        0x253c | /* box drawings light vertical and horizontal */
        0x253d | /* box drawings left heavy and right vertical light */
        0x253e | /* box drawings right heavy and left vertical light */
        0x253f | /* box drawings vertical light and horizontal heavy */
        0x2540 | /* box drawings up heavy and down horizontal light */
        0x2541 | /* box drawings down heavy and up horizontal light */
        0x2542 | /* box drawings vertical heavy and horizontal light */
        0x2543 | /* box drawings left up heavy and right down light */
        0x2544 | /* box drawings right up heavy and left down light */
        0x2545 | /* box drawings left down heavy and right up light */
        0x2546 | /* box drawings right down heavy and left up light */
        0x2547 | /* box drawings down light and up horizontal heavy */
        0x2548 | /* box drawings up light and down horizontal heavy */
        0x2549 | /* box drawings right light and left vertical heavy */
        0x254a | /* box drawings left light and right vertical heavy */
        0x254b | /* box drawings heavy vertical and horizontal */
        0x2550 | /* box drawings double horizontal */
        0x2551 | /* box drawings double vertical */
        0x2552 | /* box drawings down single and right double */
        0x2553 | /* box drawings down double and right single */
        0x2554 | /* box drawings double down and right */
        0x2555 | /* box drawings down single and left double */
        0x2556 | /* box drawings down double and left single */
        0x2557 | /* box drawings double down and left */
        0x2558 | /* box drawings up single and right double */
        0x2559 | /* box drawings up double and right single */
        0x255a | /* box drawings double up and right */
        0x255b | /* box drawings up single and left double */
        0x255c | /* box drawings up double and left single */
        0x255d | /* box drawings double up and left */
        0x255e | /* box drawings vertical single and right double */
        0x255f | /* box drawings vertical double and right single */
        0x2560 | /* box drawings double vertical and right */
        0x2561 | /* box drawings vertical single and left double */
        0x2562 | /* box drawings vertical double and left single */
        0x2563 | /* box drawings double vertical and left */
        0x2564 | /* box drawings down single and horizontal double */
        0x2565 | /* box drawings down double and horizontal single */
        0x2566 | /* box drawings double down and horizontal */
        0x2567 | /* box drawings up single and horizontal double */
        0x2568 | /* box drawings up double and horizontal single */
        0x2569 | /* box drawings double up and horizontal */
        0x256a | /* box drawings vertical single and horizontal double */
        0x256b | /* box drawings vertical double and horizontal single */
        0x256c | /* box drawings double vertical and horizontal */
        0x2574 | /* box drawings light left */
        0x2575 | /* box drawings light up */
        0x2576 | /* box drawings light right */
        0x2577 | /* box drawings light down */
        0x2578 | /* box drawings heavy left */
        0x2579 | /* box drawings heavy up */
        0x257a | /* box drawings heavy right */
        0x257b | /* box drawings heavy down */
        0x257c | /* box drawings light left and heavy right */
        0x257d | /* box drawings light up and heavy down */
        0x257e | /* box drawings heavy left and light right */
        0x257f => {
            /* box drawings heavy up and light down */

            let mut bitmap: u32 = DRAW_BOX_DRAWING_BITMAPS[c as usize - 0x2500];
            let xboundaries: [i32; 6] = [
                0,
                left_half - heavy_line_width / 2,
                left_half - light_line_width / 2,
                left_half - light_line_width / 2 + light_line_width,
                left_half - heavy_line_width / 2 + heavy_line_width,
                width,
            ];
            let yboundaries: [i32; 6] = [
                0,
                upper_half - heavy_line_width / 2,
                upper_half - light_line_width / 2,
                upper_half - light_line_width / 2 + light_line_width,
                upper_half - heavy_line_width / 2 + heavy_line_width,
                height,
            ];
            cr.set_line_width(0.);
            for yi in (0..5).rev() {
                for xi in (0..5).rev() {
                    if bitmap >= 1 {
                        cr.rectangle(
                            (x + xboundaries[xi]) as f64,
                            (y + yboundaries[yi]) as f64,
                            (xboundaries[xi + 1] - xboundaries[xi]) as f64,
                            (yboundaries[yi + 1] - yboundaries[yi]) as f64,
                        );
                        cr.fill()?;
                    }
                    bitmap >>= 1;
                }
            }
        }

        0x2504 | /* box drawings light triple dash horizontal */
        0x2505 | /* box drawings heavy triple dash horizontal */
        0x2506 | /* box drawings light triple dash vertical */
        0x2507 | /* box drawings heavy triple dash vertical */
        0x2508 | /* box drawings light quadruple dash horizontal */
        0x2509 | /* box drawings heavy quadruple dash horizontal */
        0x250a | /* box drawings light quadruple dash vertical */
        0x250b | /* box drawings heavy quadruple dash vertical */
        0x254c | /* box drawings light double dash horizontal */
        0x254d | /* box drawings heavy double dash horizontal */
        0x254e | /* box drawings light double dash vertical */
        0x254f => {
            /* box drawings heavy double dash vertical */

            let v: u32 = c as u32 - 0x2500;
            // int size, line_width;

            let size = if v >= 2 { height } else { width };

            match v >> 2 {
                1 => {
                    // triple dash */
                    let segment: f64 = size / 8.;
                    let dashes: [f64; 2] = [segment * 2., segment];
                    cr.set_dash(&dashes, G_N_ELEMENTS(dashes), 0.);
                }
                2 => {
                    // quadruple dash

                    let segment = size / 11.;
                    let dashes: [f64; 2] = [segment * 2., segment];
                    cr.set_dash(&dashes, G_N_ELEMENTS(dashes), 0.);
                }
                19 => {
                    // double dash

                    let segment = size / 5.;
                    let dashes: [f64; 2] = [segment * 2., segment];
                    cr.set_dash(&dashes, G_N_ELEMENTS(dashes), 0.);
                }
            }

            let line_width = if v >= 1 {
                heavy_line_width
            } else {
                light_line_width
            };
            adjust = if line_width >= 1 { 0.5 } else { 0. };

            cr.set_line_width(line_width);
            cr.set_line_cap(cairo::LineCap::Butt);
            if v >= 2 {
                cr.move_to(xcenter + adjust, y);
                cr.line_to(xcenter + adjust, y + height);
            } else {
                cr.move_to(x, ycenter + adjust);
                cr.line_to(x + width, ycenter + adjust);
            }
            cr.stroke();
        }

        0x256d | /* box drawings light arc down and right */
        0x256e | /* box drawings light arc down and left */
        0x256f | /* box drawings light arc up and left */
        0x2570 => {
            /* box drawings light arc up and right */

            let v = c as usize - 0x256d;
            // int line_width;
            // int radius;

            cr.set_line_cap(cairo::LineCap::Butt);

            let line_width = light_line_width;
            adjust = if line_width >= 1 { 0.5 } else { 0. };
            cr.set_line_width(line_width);

            let radius = (font_width + 2) / 3;
            let radius = radius.max(heavy_line_width);

            if v >= 2 {
                cr.move_to(xcenter + adjust, y);
                cr.line_to(xcenter + adjust, ycenter - radius + 2 * adjust);
            } else {
                cr.move_to(xcenter + adjust, ybottom);
                cr.line_to(xcenter + adjust, ycenter + radius);
            }
            cr.stroke();

            cr.arc(
                if v == 1 || v == 2 {
                    xcenter - radius + 2 * adjust
                } else {
                    xcenter + radius
                },
                if v >= 2 {
                    ycenter - radius + 2 * adjust
                } else {
                    ycenter + radius
                },
                radius - adjust,
                (v + 2) * M_PI / 2.0,
                (v + 3) * M_PI / 2.0,
            );
            cr.stroke();

            if v == 1 || v == 2 {
                cr.move_to(xcenter - radius + 2 * adjust, ycenter + adjust);
                cr.line_to(x, ycenter + adjust);
            } else {
                cr.move_to(xcenter + radius, ycenter + adjust);
                cr.line_to(xright, ycenter + adjust);
            }

            cr.stroke();
        }

        0x2571 | /* box drawings light diagonal upper right to lower left */
        0x2572 | /* box drawings light diagonal upper left to lower right */
        0x2573 => {
            /* box drawings light diagonal cross */

            let dx = (light_line_width + 1) / 2;
            cr.rectangle(x - dx, y, width + 2 * dx, height);
            cr.clip();
            cr.set_line_cap(CAIRO_LINE_CAP_SQUARE);
            cr.set_line_width(light_line_width);
            if (c != 0x2571) {
                cr.move_to(x, y);
                cr.line_to(xright, ybottom);
                cr.stroke();
            }
            if (c != 0x2572) {
                cr.move_to(xright, y);
                cr.line_to(x, ybottom);
                cr.stroke();
            }
        }

        /* Block Elements */
        0x2580 => {
            /* upper half block */
            rectangle(cr, x, y, width, height, 1, 2, 0, 0, 1, 1);
        }
        0x2581 | /* lower one eighth block */
        0x2582 | /* lower one quarter block */
        0x2583 | /* lower three eighths block */
        0x2584 | /* lower half block */
        0x2585 | /* lower five eighths block */
        0x2586 | /* lower three quarters block */
        0x2587 => {
            /* lower seven eighths block */

            let v = 0x2588 - c;
            rectangle(cr, x, y, width, height, 1, 8, 0, v, 1, 8);
        }

        0x2588 | /* full block */
        0x2589 | /* left seven eighths block */
        0x258a | /* left three quarters block */
        0x258b | /* left five eighths block */
        0x258c | /* left half block */
        0x258d | /* left three eighths block */
        0x258e | /* left one quarter block */
        0x258f => {
            /* left one eighth block */

            let v = 0x2590 - c;
            rectangle(cr, x, y, width, height, 8, 1, 0, 0, v, 1);
        }

        0x2590 => {
            /* right half block */
            rectangle(cr, x, y, width, height, 2, 1, 1, 0, 2, 1);
        }
        0x2591 | /* light shade */
        0x2592 | /* medium shade */
        0x2593 => {
            /* dark shade */
            cr.set_source_rgba(
                fg.red() / 65535.,
                fg.green() / 65535.,
                fg.blue() / 65535.,
                (c - 0x2590) / 4.,
            );
            cr.rectangle(x, y, width, height);
            cr.fill();
        }
        0x2594 =>
        /* upper one eighth block */
        {
            rectangle(cr, x, y, width, height, 1, 8, 0, 0, 1, 1);
        }

        0x2595 => {
            /* right one eighth block */

            rectangle(cr, x, y, width, height, 8, 1, 7, 0, 8, 1);
        }

        0x2596 => {
            /* quadrant lower left */
            rectangle(cr, x, y, width, height, 2, 2, 0, 1, 1, 2);
        }
        0x2597 => {
            /* quadrant lower right */
            rectangle(cr, x, y, width, height, 2, 2, 1, 1, 2, 2);
        }

        0x2598 => {
            /* quadrant upper left */
            rectangle(cr, x, y, width, height, 2, 2, 0, 0, 1, 1);
        }

        0x2599 => {
            /* quadrant upper left and lower left and lower right */
            rectangle(cr, x, y, width, height, 2, 2, 0, 0, 1, 1);
            rectangle(cr, x, y, width, height, 2, 2, 0, 1, 2, 2);
        }
        0x259a => {
            /* quadrant upper left and lower right */
            rectangle(cr, x, y, width, height, 2, 2, 0, 0, 1, 1);
            rectangle(cr, x, y, width, height, 2, 2, 1, 1, 2, 2);
        }
        0x259b => {
            /* quadrant upper left and upper right and lower left */
            rectangle(cr, x, y, width, height, 2, 2, 0, 0, 2, 1);
            rectangle(cr, x, y, width, height, 2, 2, 0, 1, 1, 2);
        }
        0x259c => {
            /* quadrant upper left and upper right and lower right */
            rectangle(cr, x, y, width, height, 2, 2, 0, 0, 2, 1);
            rectangle(cr, x, y, width, height, 2, 2, 1, 1, 2, 2);
        }
        0x259d => {
            /* quadrant upper right */
            rectangle(cr, x, y, width, height, 2, 2, 1, 0, 2, 1);
        }
        0x259e => {
            /* quadrant upper right and lower left */
            rectangle(cr, x, y, width, height, 2, 2, 1, 0, 2, 1);
            rectangle(cr, x, y, width, height, 2, 2, 0, 1, 1, 2);
        }
        0x259f => {
            /* quadrant upper right and lower left and lower right */
            rectangle(cr, x, y, width, height, 2, 2, 1, 0, 2, 1);
            rectangle(cr, x, y, width, height, 2, 2, 0, 1, 2, 2);
        }
        0x25e2 => {
            /* black lower right triangle */
            let coords: &[i8] = &[0, 1, 1, 0, 1, 1, -1];
            polygon(cr, x, y, width, height, 1, 1, coords);
        }

        0x25e3 => {
            /* black lower left triangle */
            let coords: &[i8] = &[0, 0, 1, 1, 0, 1, -1];
            polygon(cr, x, y, width, height, 1, 1, coords);
        }

        0x25e4 => {
            /* black upper left triangle */
            let coords: &[i8] = &[0, 0, 1, 0, 0, 1, -1];
            polygon(cr, x, y, width, height, 1, 1, coords);
        }

        0x25e5 => {
            /* black upper right triangle */
            let coords: &[i8] = &[0, 0, 1, 0, 1, 1, -1];
            polygon(cr, x, y, width, height, 1, 1, coords);
        }

        0x1fb00 |
        0x1fb01 |
        0x1fb02 |
        0x1fb03 |
        0x1fb04 |
        0x1fb05 |
        0x1fb06 |
        0x1fb07 |
        0x1fb08 |
        0x1fb09 |
        0x1fb0a |
        0x1fb0b |
        0x1fb0c |
        0x1fb0d |
        0x1fb0e |
        0x1fb0f |
        0x1fb10 |
        0x1fb11 |
        0x1fb12 |
        0x1fb13 |
        0x1fb14 |
        0x1fb15 |
        0x1fb16 |
        0x1fb17 |
        0x1fb18 |
        0x1fb19 |
        0x1fb1a |
        0x1fb1b |
        0x1fb1c |
        0x1fb1d |
        0x1fb1e |
        0x1fb1f |
        0x1fb20 |
        0x1fb21 |
        0x1fb22 |
        0x1fb23 |
        0x1fb24 |
        0x1fb25 |
        0x1fb26 |
        0x1fb27 |
        0x1fb28 |
        0x1fb29 |
        0x1fb2a |
        0x1fb2b |
        0x1fb2c |
        0x1fb2d |
        0x1fb2e |
        0x1fb2f |
        0x1fb30 |
        0x1fb31 |
        0x1fb32 |
        0x1fb33 |
        0x1fb34 |
        0x1fb35 |
        0x1fb36 |
        0x1fb37 |
        0x1fb38 |
        0x1fb39 |
        0x1fb3a |
        0x1fb3b => {
            let mut bitmap: u32 = c - 0x1fb00 + 1;
            if bitmap >= 0x15 {
                bitmap += 1
            };
            if bitmap >= 0x2a {
                bitmap += 1
            };
            // int xi, yi;
            cr.set_line_width(0.);
            // for (yi = 0; yi <= 2; yi++) {
            for yi in 0..=2 {
                // for (xi = 0; xi <= 1; xi++) {
                for xi in 0..=1 {
                    if bitmap >= 1 {
                        rectangle(cr, x, y, width, height, 2, 3, xi, yi, xi + 1, yi + 1);
                    }
                    bitmap >>= 1;
                }
            }
        }

        0x1fb3c |
        0x1fb3d |
        0x1fb3e |
        0x1fb3f |
        0x1fb40 |
        0x1fb41 |
        0x1fb42 |
        0x1fb43 |
        0x1fb44 |
        0x1fb45 |
        0x1fb46 |
        0x1fb47 |
        0x1fb48 |
        0x1fb49 |
        0x1fb4a |
        0x1fb4b |
        0x1fb4c |
        0x1fb4d |
        0x1fb4e |
        0x1fb4f |
        0x1fb50 |
        0x1fb51 |
        0x1fb52 |
        0x1fb53 |
        0x1fb54 |
        0x1fb55 |
        0x1fb56 |
        0x1fb57 |
        0x1fb58 |
        0x1fb59 |
        0x1fb5a |
        0x1fb5b |
        0x1fb5c |
        0x1fb5d |
        0x1fb5e |
        0x1fb5f |
        0x1fb60 |
        0x1fb61 |
        0x1fb62 |
        0x1fb63 |
        0x1fb64 |
        0x1fb65 |
        0x1fb66 |
        0x1fb67 => {
            let v = c - 0x1fb3c;
            let coords: &[&[i8]] = [
                &[0, 2, 1, 3, 0, 3, -1],             /* 3c */
                &[0, 2, 2, 3, 0, 3, -1],             /* 3d */
                &[0, 1, 1, 3, 0, 3, -1],             /* 3e */
                &[0, 1, 2, 3, 0, 3, -1],             /* 3f */
                &[0, 0, 1, 3, 0, 3, -1],             /* 40 */
                &[0, 1, 1, 0, 2, 0, 2, 3, 0, 3, -1], /* 41 */
                &[0, 1, 2, 0, 2, 3, 0, 3, -1],       /* 42 */
                &[0, 2, 1, 0, 2, 0, 2, 3, 0, 3, -1], /* 43 */
                &[0, 2, 2, 0, 2, 3, 0, 3, -1],       /* 44 */
                &[0, 3, 1, 0, 2, 0, 2, 3, -1],       /* 45 */
                &[0, 2, 2, 1, 2, 3, 0, 3, -1],       /* 46 */
                &[1, 3, 2, 2, 2, 3, -1],             /* 47 */
                &[0, 3, 2, 2, 2, 3, -1],             /* 48 */
                &[1, 3, 2, 1, 2, 3, -1],             /* 49 */
                &[0, 3, 2, 1, 2, 3, -1],             /* 4a */
                &[1, 3, 2, 0, 2, 3, -1],             /* 4b */
                &[0, 0, 1, 0, 2, 1, 2, 3, 0, 3, -1], /* 4c */
                &[0, 0, 2, 1, 2, 3, 0, 3, -1],       /* 4d */
                &[0, 0, 1, 0, 2, 2, 2, 3, 0, 3, -1], /* 4e */
                &[0, 0, 2, 2, 2, 3, 0, 3, -1],       /* 4f */
                &[0, 0, 1, 0, 2, 3, 0, 3, -1],       /* 50 */
                &[0, 1, 2, 2, 2, 3, 0, 3, -1],       /* 51 */
                &[0, 0, 2, 0, 2, 3, 1, 3, 0, 2, -1], /* 52 */
                &[0, 0, 2, 0, 2, 3, 0, 2, -1],       /* 53 */
                &[0, 0, 2, 0, 2, 3, 1, 3, 0, 1, -1], /* 54 */
                &[0, 0, 2, 0, 2, 3, 0, 1, -1],       /* 55 */
                &[0, 0, 2, 0, 2, 3, 1, 3, -1],       /* 56 */
                &[0, 0, 1, 0, 0, 1, -1],             /* 57 */
                &[0, 0, 2, 0, 0, 1, -1],             /* 58 */
                &[0, 0, 1, 0, 0, 2, -1],             /* 59 */
                &[0, 0, 2, 0, 0, 2, -1],             /* 5a */
                &[0, 0, 1, 0, 0, 3, -1],             /* 5b */
                &[0, 0, 2, 0, 2, 1, 0, 2, -1],       /* 5c */
                &[0, 0, 2, 0, 2, 2, 1, 3, 0, 3, -1], /* 5d */
                &[0, 0, 2, 0, 2, 2, 0, 3, -1],       /* 5e */
                &[0, 0, 2, 0, 2, 1, 1, 3, 0, 3, -1], /* 5f */
                &[0, 0, 2, 0, 2, 1, 0, 3, -1],       /* 60 */
                &[0, 0, 2, 0, 1, 3, 0, 3, -1],       /* 61 */
                &[1, 0, 2, 0, 2, 1, -1],             /* 62 */
                &[0, 0, 2, 0, 2, 1, -1],             /* 63 */
                &[1, 0, 2, 0, 2, 2, -1],             /* 64 */
                &[0, 0, 2, 0, 2, 2, -1],             /* 65 */
                &[1, 0, 2, 0, 2, 3, -1],             /* 66 */
                &[0, 0, 2, 0, 2, 2, 0, 1, -1],       /* 67 */
            ];
            polygon(cr, x, y, width, height, 2, 3, coords[v]);
        }

        0x1fb68 => {}
        0x1fb69 => {}
        0x1fb6a => {}
        0x1fb6b => {}
        0x1fb6c => {}
        0x1fb6d => {}
        0x1fb6e => {}
        0x1fb6f => {
            let v = c - 0x1fb68;
            let coords: [[i8; 11]; 8] = [
                [0, 0, 2, 0, 2, 2, 0, 2, 1, 1, -1], /* 68 */
                [0, 0, 1, 1, 2, 0, 2, 2, 0, 2, -1], /* 69 */
                [0, 0, 2, 0, 1, 1, 2, 2, 0, 2, -1], /* 6a */
                [0, 0, 2, 0, 2, 2, 1, 1, 0, 2, -1], /* 6b */
                [0, 0, 1, 1, 0, 2, -1],             /* 6c */
                [0, 0, 2, 0, 1, 1, -1],             /* 6d */
                [1, 1, 2, 0, 2, 2, -1],             /* 6e */
                [1, 1, 2, 2, 0, 2, -1],             /* 6f */
            ];
            polygon(cr, x, y, width, height, 2, 2, coords[v]);
        }

        0x1fb70 => {}
        0x1fb71 => {}
        0x1fb72 => {}
        0x1fb73 => {}
        0x1fb74 => {}
        0x1fb75 => {
            let v = c - 0x1fb70 + 1;
            rectangle(cr, x, y, width, height, 8, 1, v, 0, v + 1, 1);
        }

        0x1fb76 => {}
        0x1fb77 => {}
        0x1fb78 => {}
        0x1fb79 => {}
        0x1fb7a => {}
        0x1fb7b => {
            let v = c - 0x1fb76 + 1;
            rectangle(cr, x, y, width, height, 1, 8, 0, v, 1, v + 1);
        }

        0x1fb7c => {
            rectangle(cr, x, y, width, height, 1, 8, 0, 7, 1, 8);
            rectangle(cr, x, y, width, height, 8, 1, 0, 0, 1, 1);
        }
        0x1fb7d => {
            rectangle(cr, x, y, width, height, 1, 8, 0, 0, 1, 1);
            rectangle(cr, x, y, width, height, 8, 1, 0, 0, 1, 1);
        }
        0x1fb7e => {
            rectangle(cr, x, y, width, height, 1, 8, 0, 0, 1, 1);
            rectangle(cr, x, y, width, height, 8, 1, 7, 0, 8, 1);
        }
        0x1fb7f => {
            rectangle(cr, x, y, width, height, 1, 8, 0, 7, 1, 8);
            rectangle(cr, x, y, width, height, 8, 1, 7, 0, 8, 1);
        }
        0x1fb80 => {
            rectangle(cr, x, y, width, height, 1, 8, 0, 0, 1, 1);
            rectangle(cr, x, y, width, height, 1, 8, 0, 7, 1, 8);
        }
        0x1fb81 => {
            rectangle(cr, x, y, width, height, 1, 8, 0, 0, 1, 1);
            rectangle(cr, x, y, width, height, 1, 8, 0, 2, 1, 3);
            rectangle(cr, x, y, width, height, 1, 8, 0, 4, 1, 5);
            rectangle(cr, x, y, width, height, 1, 8, 0, 7, 1, 8);
        }
        0x1fb82 => {}
        0x1fb83 => {}
        0x1fb84 => {}
        0x1fb85 => {}
        0x1fb86 => {
            let mut v = c - 0x1fb82 + 2;
            if v >= 4 {
                v += 1
            };
            rectangle(cr, x, y, width, height, 1, 8, 0, 0, 1, v);
        }

        0x1fb87 => {}
        0x1fb88 => {}
        0x1fb89 => {}
        0x1fb8a => {}
        0x1fb8b => {
            let v = c - 0x1fb87 + 2;
            if v >= 4 {
                v += 1
            };
            rectangle(cr, x, y, width, height, 8, 1, 8 - v, 0, 8, 1);
        }

        0x1fb8c => {
            cr.set_source_rgba(
                fg.red() / 65535.,
                fg.green() / 65535.,
                fg.blue() / 65535.,
                0.5,
            );
            rectangle(cr, x, y, width, height, 2, 1, 0, 0, 1, 1);
        }
        0x1fb8d => {
            cr.set_source_rgba(
                fg.red() / 65535.,
                fg.green() / 65535.,
                fg.blue() / 65535.,
                0.5,
            );
            rectangle(cr, x, y, width, height, 2, 1, 1, 0, 2, 1);
        }
        0x1fb8e => {
            cr.set_source_rgba(
                fg.red() / 65535.,
                fg.green() / 65535.,
                fg.blue() / 65535.,
                0.5,
            );
            rectangle(cr, x, y, width, height, 1, 2, 0, 0, 1, 1);
        }
        0x1fb8f => {
            cr.set_source_rgba(
                fg.red() / 65535.,
                fg.green() / 65535.,
                fg.blue() / 65535.,
                0.5,
            );
            rectangle(cr, x, y, width, height, 1, 2, 0, 1, 1, 2);
        }
        0x1fb90 => {
            cr.set_source_rgba(fg.red() / 65535., fg.green() / 65535., fg.blue() / 65535., 0.5);
            rectangle(cr, x, y, width, height, 1, 1, 0, 0, 1, 1);
        }
        0x1fb91 => {
            rectangle(cr, x, y, width, height, 1, 2, 0, 0, 1, 1);
            cr.set_source_rgba(fg.red() / 65535., fg.green() / 65535., fg.blue() / 65535., 0.5);
            rectangle(cr, x, y, width, height, 1, 2, 0, 1, 1, 2);
        }
        0x1fb92 => {
            rectangle(cr, x, y, width, height, 1, 2, 0, 1, 1, 2);
            cr.set_source_rgba(fg.red() / 65535., fg.green() / 65535., fg.blue() / 65535., 0.5);
            rectangle(cr, x, y, width, height, 1, 2, 0, 0, 1, 1);
        }

        0x1fb93 => {
            //#if 0
            //                /* codepoint not assigned */
            //                rectangle(cr, x, y, width, height, 2, 1,  0, 0,  1, 1);
            //                cairo_set_source_rgba (cr,
            //                                       fg.red / 65535.,
            //                                       fg.green / 65535.,
            //                                       fg.blue / 65535.,
            //                                       0.5);
            //                rectangle(cr, x, y, width, height, 2, 1,  1, 0,  2, 1);
            //#endif
        }
        0x1fb94 => {
            rectangle(cr, x, y, width, height, 2, 1, 1, 0, 2, 1);
            cr.set_source_rgba(fg.red() / 65535., fg.green() / 65535., fg.blue() / 65535., 0.5);
            rectangle(cr, x, y, width, height, 2, 1, 0, 0, 1, 1);
        }

        0x1fb95 => {
            pattern(cr, create_checkerboard_pattern(), x, y, width, height);
        }

        0x1fb96 => {
            pattern(
                cr,
                create_checkerboard_reverse_pattern(),
                x,
                y,
                width,
                height,
            );
        }

        0x1fb97 => {
            rectangle(cr, x, y, width, height, 1, 4, 0, 1, 1, 2);
            rectangle(cr, x, y, width, height, 1, 4, 0, 3, 1, 4);
        }

        0x1fb98 => {
            pattern(cr, create_hatching_pattern_lr(), x, y, width, height);
        }

        0x1fb99 => {
            pattern(cr, create_hatching_pattern_rl(), x, y, width, height);
        }

        0x1fb9a => {
            let coords: &[i8] = &[0, 0, 1, 0, 0, 1, 1, 1, -1];
            polygon(cr, x, y, width, height, 1, 1, &coords);
        }

        0x1fb9b => {
            let coords: &[i8] = &[0, 0, 1, 1, 1, 0, 0, 1, -1];
            polygon(cr, x, y, width, height, 1, 1, &coords);
        }

        0x1fb9c => {
            let coords: &[i8] = &[0, 0, 1, 0, 0, 1, -1];
            cr.set_source_rgba(fg.red / 65535., fg.green / 65535., fg.blue / 65535., 0.5);
            polygon(cr, x, y, width, height, 1, 1, &coords);
        }

        0x1fb9d => {
            let coords: &[i8] = &[0, 0, 1, 0, 1, 1, -1];
            cr.set_source_rgba(fg.red / 65535., fg.green / 65535., fg.blue / 65535., 0.5);
            polygon(cr, x, y, width, height, 1, 1, &coords);
        }

        0x1fb9e => {
            let coords: &[i8] = &[0, 1, 1, 0, 1, 1, -1];
            cr.set_source_rgba(fg.red / 65535., fg.green / 65535., fg.blue / 65535., 0.5);
            polygon(cr, x, y, width, height, 1, 1, &coords);
        }

        0x1fb9f => {
            let coords: &[i8] = &[0, 0, 1, 1, 0, 1, -1];
            cr.set_source_rgba(fg.red / 65535., fg.green / 65535., fg.blue / 65535., 0.5);
            polygon(cr, x, y, width, height, 1, 1, &coords);
        }

        0x1fba0 => {}
        0x1fba1 => {}
        0x1fba2 => {}
        0x1fba3 => {}
        0x1fba4 => {}
        0x1fba5 => {}
        0x1fba6 => {}
        0x1fba7 => {}
        0x1fba8 => {}
        0x1fba9 => {}
        0x1fbaa => {}
        0x1fbab => {}
        0x1fbac => {}
        0x1fbad => {}
        0x1fbae => {
            let v = c - 0x1fba0;
            let map: [u8; 15] = [
                0b0001, 0b0010, 0b0100, 0b1000, 0b0101, 0b1010, 0b1100, 0b0011, 0b1001, 0b0110,
                0b1110, 0b1101, 0b1011, 0b0111, 0b1111,
            ];
            cr.set_line_cap(cairo::LineCap::Butt);
            cr.set_line_cap(cairo::LineCap::Butt);
            cr.set_line_width(light_line_width);
            adjust = if light_line_width >= 1 { 0.5 } else { 0. };
            let dx: f64 = light_line_width / 2.;
            let dy: f64 = light_line_width / 2.;
            if map[v as usize] >= 1 {
                /* upper left */
                cr.move_to(x, ycenter + adjust);
                cr.line_to(x + dx, ycenter + adjust);
                cr.line_to(xcenter + adjust, y + dy);
                cr.line_to(xcenter + adjust, y);
                cr.stroke();
            }
            if map[v as usize] >= 2 {
                /* upper right */
                cr.move_to(xright, ycenter + adjust);
                cr.line_to(xright - dx, ycenter + adjust);
                cr.line_to(xcenter + adjust, y + dy);
                cr.line_to(xcenter + adjust, y);
                cr.stroke();
            }
            if map[v as usize] >= 4 {
                /* lower left */
                cr.move_to(x, ycenter + adjust);
                cr.line_to(x + dx, ycenter + adjust);
                cr.line_to(xcenter + adjust, ybottom - dy);
                cr.line_to(xcenter + adjust, ybottom);
                cr.stroke();
            }
            if map[v as usize] >= 8 {
                /* lower right */
                cr.move_to(xright, ycenter + adjust);
                cr.line_to(xright - dx, ycenter + adjust);
                cr.line_to(xcenter + adjust, ybottom - dy);
                cr.line_to(xcenter + adjust, ybottom);
                cr.stroke();
            }
        }

        _ => {
            unreachable!();
        }
    }

    cr.restore();
    Ok(())
}
