use glib::translate::{FromGlib, IntoGlib};

use crate::{
    attr, builtin_font,
    color::Colorf,
    consts,
    font_metrics::{Coverage, FontAnalysis, RcFontAnalysis},
};

const MAX_RUN_LENGTH: usize = 100;

type FontStyle = usize;

const DRAW_NORMAL: FontStyle = 0;
const DRAW_BOLD: FontStyle = 1;
const DRAW_ITALIC: FontStyle = 2;
const DRAW_BOLD_ITALIC: FontStyle = 3;

trait AttrExt {
    fn style(&self) -> FontStyle;
}
impl AttrExt for attr::Attr {
    fn style(&self) -> FontStyle {
        let mut style = DRAW_NORMAL;
        if self.contains(attr::Attr::BOLD) {
            style |= DRAW_BOLD
        }
        if self.contains(attr::Attr::ITALIC) {
            style |= DRAW_ITALIC
        }
        style
    }
}
fn undercurl_rad(width: f64) -> f64 {
    width / 2. / std::f64::consts::SQRT_2
}

fn undercurl_arc_height(width: f64) -> f64 {
    undercurl_rad(width) * (1. - std::f64::consts::SQRT_2 / 2.)
}

pub fn undercurl_height(width: f64, line_width: f64) -> f64 {
    2. * undercurl_arc_height(width) + line_width
}

#[derive(Clone, Copy, Debug)]
pub struct Border {
    left: f64,
    right: f64,
    top: f64,
    bottom: f64,
}

impl Border {
    pub fn left(&self) -> f64 {
        self.left
    }
    pub fn right(&self) -> f64 {
        self.right
    }
    pub fn top(&self) -> f64 {
        self.top
    }
    pub fn bottom(&self) -> f64 {
        self.bottom
    }
}

impl Default for Border {
    fn default() -> Self {
        Border {
            left: 1.,
            right: 1.,
            top: 1.,
            bottom: 1.,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CharMetrics {
    cell_width: f64,
    cell_height: f64,
    char_ascent: f64,
    char_descent: f64,
    char_spacing: Border,
}

impl CharMetrics {
    pub fn cell_height(&self) -> f64 {
        self.cell_height
    }
    pub fn cell_width(&self) -> f64 {
        self.cell_width
    }
    pub fn char_ascent(&self) -> f64 {
        self.char_ascent
    }
    pub fn char_descent(&self) -> f64 {
        self.char_descent
    }
    pub fn char_spacing(&self) -> &Border {
        &self.char_spacing
    }
}
/// A request to draw a particular character spanning a given number of columns
/// at the given location.  Unlike most APIs, (x,y) specifies the top-left
/// corner of the cell into which the character will be drawn instead of the
/// left end of the baseline.
pub struct CharRequest {
    c: char,
    x: i16,
    y: i16,
    columns: i32,
    /// Char has RTL resolved directionality, mirror if mirrorable.
    mirror: bool,
    /// Add box drawing chars to the set of mirrorable characters.
    box_mirror: u8,
}

pub struct DrawingContext {
    cr: cairo::Context,
    font_analyses: [Option<RcFontAnalysis>; 4], // size = 4
    cell_width: f64,                            // 1.
    cell_height: f64,                           // 1.
    char_spacing: Border,                       // [1., 1., 1., 1.]

    /// Cache the undercurl's rendered look.
    undercurl_surface: Option<cairo::Surface>,
}

impl DrawingContext {
    pub fn set_cairo(&mut self, cr: cairo::Context) {
        self.cr = cr;
    }
    pub fn cairo(&self) -> &cairo::Context {
        &self.cr
    }
    pub fn cell_width(&self) -> f64 {
        self.cell_width
    }
    pub fn cell_height(&self) -> f64 {
        self.cell_height
    }

    pub fn clip(&self, rect: &cairo::Rectangle) {
        self.cr.save();
        self.cr
            .rectangle(rect.x(), rect.y(), rect.width(), rect.height());
        self.cr.clip();
    }
    pub fn unclip(&self) -> Result<(), cairo::Error> {
        self.cr.restore()
    }

    pub fn clear(
        &self,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        color: &Colorf,
    ) -> Result<(), cairo::Error> {
        self.cr.rectangle(x, y, width, height);
        self.cr.set_operator(cairo::Operator::Source);
        self.set_source_color_alpha(color);
        self.cr.fill()
    }
    pub fn clear_font_analyses(&mut self) {
        self.font_analyses[DRAW_NORMAL].take();
        self.font_analyses[DRAW_BOLD].take();
        self.font_analyses[DRAW_ITALIC].take();
        self.font_analyses[DRAW_BOLD_ITALIC].take();
    }
    pub fn set_text_font(
        &mut self,
        widget: &gtk::Widget,
        fontdesc: &pango::FontDescription,
        cell_width_scale: f64,
        cell_height_scale: f64,
    ) {
        //    PangoFontDescription *bolddesc   = nullptr;
        //PangoFontDescription *italicdesc = nullptr;
        //PangoFontDescription *bolditalicdesc = nullptr;
        // gint normal, bold, ratio;

        log::debug!("draw_set_text_font");

        self.clear_font_analyses();

        /* calculate bold font desc */
        let mut bolddesc = fontdesc.clone();
        if bolddesc.set_fields().contains(pango::FontMask::WEIGHT) {
            let weight = bolddesc.weight();
            let bold_weight = 1000.min(weight.into_glib() + consts::FONT_WEIGHT_BOLDENING);
            bolddesc.set_weight(unsafe { pango::Weight::from_glib(bold_weight) });
        } else {
            bolddesc.set_weight(pango::Weight::Bold);
        }

        /* calculate italic font desc */
        let mut italicdesc = fontdesc.clone();
        italicdesc.set_style(pango::Style::Italic);

        /* calculate bold italic font desc */
        let mut bolditalicdesc = bolddesc.clone();
        bolditalicdesc.set_style(pango::Style::Italic);

        self.font_analyses[DRAW_NORMAL].replace(FontAnalysis::create_for_widget(widget, fontdesc));
        self.font_analyses[DRAW_BOLD].replace(FontAnalysis::create_for_widget(widget, &bolddesc));
        self.font_analyses[DRAW_ITALIC]
            .replace(FontAnalysis::create_for_widget(widget, &italicdesc));
        self.font_analyses[DRAW_BOLD_ITALIC]
            .replace(FontAnalysis::create_for_widget(widget, &bolditalicdesc));

        /* Decide if we should keep this bold font face, per bug 54926:
         *  - reject bold font if it is not within 10% of normal font width
         */
        let mut normal = DRAW_NORMAL;
        let mut bold = normal | DRAW_BOLD;
        let mut ratio = self.font_analyses[bold].as_ref().unwrap().width() * 100.
            / self.font_analyses[normal].as_ref().unwrap().width();
        if (ratio - 100.).abs() > 10. {
            log::debug!("Rejecting bold font ({}%).", ratio);
            self.font_analyses[bold].replace(self.font_analyses[normal].as_ref().unwrap().clone());
        }
        normal = DRAW_ITALIC;
        bold = normal | DRAW_BOLD;
        ratio = self.font_analyses[bold].as_ref().unwrap().width() * 100.
            / self.font_analyses[normal].as_ref().unwrap().width();
        if (ratio - 100.).abs() > 10. {
            log::debug!("Rejecting italic bold font ({}%).", ratio);
            self.font_analyses[bold].replace(self.font_analyses[normal].as_ref().unwrap().clone());
        }

        /* Apply letter spacing and line spacing. */
        self.cell_width =
            self.font_analyses[DRAW_NORMAL].as_ref().unwrap().width() * cell_width_scale;
        self.char_spacing.left =
            (self.cell_width - self.font_analyses[DRAW_NORMAL].as_ref().unwrap().width()) / 2.;
        self.char_spacing.right =
            (self.cell_width - self.font_analyses[DRAW_NORMAL].as_ref().unwrap().width() + 1.) / 2.;
        self.cell_height =
            self.font_analyses[DRAW_NORMAL].as_ref().unwrap().height() * cell_height_scale;
        self.char_spacing.top =
            (self.cell_height - self.font_analyses[DRAW_NORMAL].as_ref().unwrap().height() + 1.)
                / 2.;
        self.char_spacing.bottom =
            (self.cell_height - self.font_analyses[DRAW_NORMAL].as_ref().unwrap().height()) / 2.;

        self.undercurl_surface.take();
    }
    pub fn char_metrics(&self) -> Option<CharMetrics> {
        if self.font_analyses[DRAW_NORMAL].is_none() {
            return None;
        }

        let font = self.font_analyses[DRAW_NORMAL].as_ref().unwrap();

        CharMetrics {
            cell_width: self.cell_width,
            cell_height: self.cell_height,
            char_ascent: font.ascent(),
            char_descent: font.height() - font.ascent(),
            char_spacing: self.char_spacing,
        }
        .into()

        //                    int* cell_width,
        //                    int* cell_height,
        //                    int* char_ascent,
        //                    int* char_descent,
        //                    GtkBorder* char_spacing);
    }

    /// left, right
    pub fn char_edges(&self, c: char, columns: i32, attr: attr::Attr) -> (f64, f64) {
        if !crate::builtin_font::is_builtin(c) {
            return (0., self.cell_width * f64::from(columns));
        }

        if self.font_analyses[DRAW_NORMAL].is_none() {
            return (0., 0.);
        }

        let w = self.font_analyses[attr.style()]
            .as_ref()
            .unwrap()
            .char_analysis(c)
            .width();
        let normal_width = self.font_analyses[DRAW_NORMAL]
            .as_ref()
            .map(|fa| fa.width() * columns as f64)
            .unwrap();
        let fits_width = self.cell_width * columns as f64;

        let l = if w <= normal_width {
            /* The regular case: The glyph is not wider than one (CJK: two) regular character(s).
             * Align to the left, after applying half (CJK: one) letter spacing. */
            self.char_spacing.left
                + if columns == 2 {
                    self.char_spacing.right
                } else {
                    0.
                }
        } else if w <= fits_width {
            /* Slightly wider glyph, but still fits in the cell (spacing included). This case can
             * only happen with nonzero letter spacing. Center the glyph in the cell(s). */
            (fits_width - w) / 2.
        } else {
            /* Even wider glyph: doesn't fit in the cell. Align at left and overflow on the right. */
            0.
        };

        // let left = l;
        // let right = l + w;
        (l, l + w)
    }

    fn draw_text_internal(&self, requests: &[CharRequest], attr: attr::Attr, color: &Colorf) {
        // gsize i;
        // cairo_scaled_font_t *last_scaled_font = nullptr;
        let mut last_scaled_font = None;
        let mut n_cr_glyphs = 0;
        let mut cr_glyphs = Vec::with_capacity(MAX_RUN_LENGTH);
        let fa = match &self.font_analyses[attr.style()] {
            Some(fa) => fa,
            None => return,
        };

        self.set_source_color_alpha(color);
        self.cr.set_operator(cairo::Operator::Over);

        for req in requests.iter() {
            let c = req.c;

            if req.mirror {
                vte_bidi_get_mirror_char(c, req.box_mirror, &c);
            }

            if builtin_font::is_builtin(c) {
                builtin_font::draw_builtin(self, c, color, req.x, req.y, fa.width(), req.columns);
                continue;
            }

            let ca = fa.char_analysis(c);
            // int x, y, ye;

            let (mut x, mut ye) = self.char_edges(c, req.columns, attr); // , x, ye /* unused */);
            x += req.x as f64;
            /* Bold/italic versions might have different ascents. In order to align their
             * baselines, we offset by the normal font's ascent here. (Bug 137.) */
            let y = req.y as f64
                + self.char_spacing.top
                + self.font_analyses[DRAW_NORMAL].as_ref().unwrap().ascent();

            match ca.coverage() {
                Coverage::Unknown => unreachable!(),
                Coverage::PangoLayoutLine(line) => {
                    self.cr.move_to(x, y);
                    match line {
                        Some(l) => {
                            pangocairo::show_layout_line(&self.cr, l);
                        }
                        None => {}
                    }
                }
                Coverage::PangoGlyphString(font, mut glyph_string) => {
                    self.cr.move_to(x, y);
                    pangocairo::show_glyph_string(&self.cr, font, &mut glyph_string);
                }
                Coverage::CairoGlyph(glyph_index, scaled_font) => {
                    if last_scaled_font
                        .as_ref()
                        .map(|lsf| !std::ptr::eq(lsf.to_raw_none(), scaled_font.to_raw_none()))
                        .unwrap_or(true)
                        || n_cr_glyphs == MAX_RUN_LENGTH
                    {
                        if n_cr_glyphs > 0 {
                            self.cr.set_scaled_font(last_scaled_font.as_ref().unwrap());
                            self.cr.show_glyphs(&cr_glyphs);
                            n_cr_glyphs = 0;
                        }
                        last_scaled_font.replace(*scaled_font);
                    }
                    let glyph = cairo::ffi::cairo_glyph_t {
                        index: *glyph_index as u64,
                        x,
                        y,
                    };

                    cr_glyphs.push(unsafe { std::mem::transmute(glyph) });
                    n_cr_glyphs += 1;
                }
            }
        }
        if n_cr_glyphs > 0 {
            self.cr.set_scaled_font(&last_scaled_font.unwrap());
            self.cr.show_glyphs(&cr_glyphs);
            n_cr_glyphs = 0;
        }
    }

    pub fn draw_text(&self, requests: &[CharRequest], attr: u32, color: &Colorf, alpha: f64) {
        unimplemented!()
    }
    pub fn draw_char(&self, request: &CharRequest, attr: u32, color: &Colorf, alpha: f64) -> bool {
        unimplemented!()
    }
    pub fn has_char(&self, c: char, attr: u32) -> bool {
        unimplemented!()
    }
    pub fn fill_rectangle(
        &self,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        color: &Colorf,
        alpha: f64,
    ) {
        unimplemented!()
    }
    pub fn draw_rectangle(
        &self,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        color: &Colorf,
        alpha: f64,
    ) {
        unimplemented!()
    }
    pub fn draw_line(
        &self,
        x: f64,
        y: f64,
        xp: f64,
        yp: f64,
        line_width: f64,
        color: &Colorf,
        alpha: f64,
    ) {
        unimplemented!()
    }

    pub fn draw_undercurl(
        &self,
        x: f64,
        y: f64,
        line_width: f64,
        count: i32,
        color: &Colorf,
        alpha: f64,
    ) {
        unimplemented!()
    }

    fn set_source_color_alpha(&self, color: &Colorf) {
        self.cr
            .set_source_rgba(color.red(), color.green(), color.blue(), color.alpha());
    }
    fn draw_graphic(
        &self,
        c: char,
        attr: u32,
        fg: &Colorf,
        x: f64,
        y: f64,
        font_width: f64,
        columns: usize,
        font_height: f64,
    ) {
        unimplemented!()
    }
}
