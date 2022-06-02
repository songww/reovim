use std::rc::Rc;

// use pangocairo::prelude::FontExt as PangoCairoFontExt;
use glib::translate::ToGlibPtr;
use glib::Cast;
use once_cell::sync::Lazy;
use pango::prelude::{FontExt, FontMapExt, FontsetExt};
use pangocairo::traits::FontExt as PangoCairoFontExt;
// use unicode_normalization::UnicodeNormalization;
use xi_unicode::{is_keycap_base, EmojiExt};

mod attributes;
mod layout;
mod text_cell;
mod text_line;

pub use text_cell::TextCell;
pub use text_line::TextLine;

use crate::vimview::HighlightDefinitions;

static BOX_DRAWING: &'static [u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/box-drawing.ttf"
));

pub trait IsSame {
    fn is_same(&self, other: &Self) -> bool;
}

impl IsSame for cairo::ScaledFont {
    fn is_same(&self, other: &Self) -> bool {
        self.to_glib_none().0 == other.to_glib_none().0
    }
}

pub struct Builtin {
    hb: harfbuzz::Font,
    // ft: freetype::face::Face<&'static [u8]>,
    ft: freetype::face::Face,
    // scaled_font: cairo::ScaledFont,
}

unsafe impl Send for Builtin {}
unsafe impl Sync for Builtin {}

pub fn builtin(ptem: f32) -> Builtin {
    static FONT: Lazy<Builtin> = Lazy::new(|| {
        let blob = harfbuzz::Blob::new_read_only(BOX_DRAWING);
        let mut face = harfbuzz::Face::new(&blob, 0);
        let hb = harfbuzz::Font::new(&mut face);
        let ft = freetype::Library::init().unwrap();
        let data: Vec<u8> = BOX_DRAWING.to_vec();
        let ftface = ft.new_memory_face(data, 0).unwrap();
        Builtin { hb, ft: ftface }
    });
    let ft = FONT.ft.clone();
    let mut hb = FONT.hb.clone();
    hb.set_ptem(ptem);
    ft.set_char_size((ptem * 64.) as isize, 0, 0, 0).unwrap();
    Builtin { hb, ft }
}

impl Builtin {
    pub fn has_char(c: char) -> bool {
        c >= '\u{2500}' && c <= '\u{257f}'
    }

    pub fn scaled_font(&self) -> anyhow::Result<cairo::ScaledFont> {
        static FT_FACE: cairo::UserDataKey<freetype::face::Face> = cairo::UserDataKey::new();
        // TODO: variations
        let face = cairo::FontFace::create_from_ft(&self.ft)?;
        face.set_user_data(&FT_FACE, Rc::new(self.ft.clone()))
            .unwrap();
        let upem = self.hb.ptem() as f64 / 72. * 96.;
        let (sx, sy) = (upem, upem);
        let ctm = cairo::Matrix::identity();
        let font_matrix = cairo::Matrix::new(sx, 0., 0., sy, 0., 0.);
        let mut options = cairo::FontOptions::new().unwrap();
        options.set_hint_style(cairo::HintStyle::None);
        options.set_hint_metrics(cairo::HintMetrics::Off);
        let scaled_font = cairo::ScaledFont::new(&face, &font_matrix, &ctm, &options)?;
        Ok(scaled_font)
    }

    fn hb(&self) -> harfbuzz::Font {
        self.hb.clone()
    }
}

trait FontExtManual {
    fn hb(&self) -> harfbuzz::Font;
}

impl FontExtManual for pango::Font {
    fn hb(&self) -> harfbuzz::Font {
        let hb = unsafe {
            let raw = pango::ffi::pango_font_get_hb_font(self.to_glib_none().0);
            let raw = harfbuzz::sys::hb_font_reference(raw as *mut _);
            harfbuzz::Font::from_raw(raw)
        };
        hb
    }
}

impl FontExtManual for pangocairo::Font {
    fn hb(&self) -> harfbuzz::Font {
        self.upcast_ref::<pango::Font>().hb()
    }
}

pub struct FontSet(Builtin, pango::Fontset);

impl FontSet {
    pub fn font(&self, wc: u32) -> Option<(harfbuzz::Font, cairo::ScaledFont)> {
        if Builtin::has_char(char::from_u32(wc).unwrap_or('a')) {
            log::info!(
                "using builtin font for char {}",
                char::from_u32(wc).unwrap()
            );
            self.0
                .scaled_font()
                .ok()
                .map(|scaled_font| (self.0.hb(), scaled_font))
        } else {
            self.1
                .font(wc)
                .inspect(|f| {
                    let c = char::from_u32(wc).unwrap();
                    if c != ' ' {
                        log::info!(
                            "using `{}` for char `{}` {:?}",
                            f.describe().unwrap(),
                            c,
                            f.metrics(None)
                        );
                    }
                })
                .and_then(|f| f.downcast().ok())
                .and_then(|f: pangocairo::Font| {
                    f.scaled_font().map(|scaled_font| (f.hb(), scaled_font))
                })
        }
    }

    pub fn metrics(&self) -> Option<pango::FontMetrics> {
        self.1.metrics()
    }
}

impl From<pango::Fontset> for FontSet {
    fn from(set: pango::Fontset) -> Self {
        let ptem = unsafe {
            let hbfont = pango::ffi::pango_font_get_hb_font(set.font(16).unwrap().to_glib_none().0);
            harfbuzz::sys::hb_font_get_ptem(hbfont as *mut _)
        };
        let builtin = builtin(ptem);
        FontSet(builtin, set)
    }
}

pub struct FontMap {
    regular: FontSet,
    bold: FontSet,
    italic: FontSet,
    bold_italic: FontSet,
    emoji: FontSet,
}

impl FontMap {
    pub fn new(
        regular: pango::FontDescription,
        bold: Option<pango::FontDescription>,
        italic: Option<pango::FontDescription>,
        bold_italic: Option<pango::FontDescription>,
    ) -> Self {
        let fontmap = pangocairo::FontMap::default().unwrap();
        let ctx = fontmap.create_context().unwrap();
        let language = pango::Language::from_string("en");
        let bold = bold.unwrap_or_else(|| {
            let mut desc = regular.clone();
            desc.set_weight(pango::Weight::Bold);
            desc
        });
        let italic = italic.unwrap_or_else(|| {
            let mut desc = regular.clone();
            desc.set_style(pango::Style::Italic);
            desc
        });
        let bold_italic = bold_italic.unwrap_or_else(|| {
            let mut desc = italic.clone();
            desc.set_weight(pango::Weight::Bold);
            desc
        });
        let emoji = {
            let mut emoji = regular.clone();
            emoji.set_family("emoji");
            emoji
        };
        let regular = fontmap.load_fontset(&ctx, &regular, &language).unwrap();
        let bold = fontmap.load_fontset(&ctx, &bold, &language).unwrap();
        let italic = fontmap.load_fontset(&ctx, &italic, &language).unwrap();
        let bold_italic = fontmap.load_fontset(&ctx, &bold_italic, &language).unwrap();
        let emoji = fontmap.load_fontset(&ctx, &emoji, &language).unwrap();

        FontMap {
            regular: FontSet::from(regular),
            bold: FontSet::from(bold),
            italic: FontSet::from(italic),
            bold_italic: FontSet::from(bold_italic),
            emoji: FontSet::from(emoji),
        }
    }

    pub fn regular(&self) -> &FontSet {
        &self.regular
    }

    pub fn italic(&self) -> &FontSet {
        &self.italic
    }

    pub fn bold(&self) -> &FontSet {
        &self.bold
    }

    pub fn bold_italic(&self) -> &FontSet {
        &self.bold_italic
    }

    pub fn emoji(&self) -> &FontSet {
        &self.emoji
    }

    pub fn metrics(&self) -> Option<pango::FontMetrics> {
        self.regular.metrics()
    }
}

pub type Nr = usize;

#[derive(Clone)]
pub struct Item<'a> {
    text: String,
    cells: Vec<&'a TextCell>,
    scaled_font: cairo::ScaledFont,
    hb_font: harfbuzz::Font,
    glyphs: Vec<cairo::Glyph>,
    clusters: Vec<cairo::TextCluster>,
}

impl<'a> Item<'a> {
    fn new(
        text: String,
        hb_font: harfbuzz::Font,
        scaled_font: cairo::ScaledFont,
        cell: &'a TextCell,
    ) -> Self {
        Item {
            text,
            hb_font,
            scaled_font,
            cells: vec![cell],
            glyphs: Vec::new(),
            clusters: Vec::new(),
        }
    }
    fn with_font(&self, hb_font: harfbuzz::Font, scaled_font: cairo::ScaledFont) -> Self {
        Item {
            text: self.text.clone(),
            hb_font,
            scaled_font,
            cells: self.cells.clone(),
            glyphs: Vec::new(),
            clusters: Vec::new(),
        }
    }

    fn with_cell(&self, cell: &'a TextCell) -> Self {
        Item {
            text: self.text.clone(),
            hb_font: self.hb_font.clone(),
            scaled_font: self.scaled_font.clone(),
            cells: vec![cell],
            glyphs: Vec::new(),
            clusters: Vec::new(),
        }
    }

    fn push_cell(&mut self, cell: &'a TextCell) {
        self.cells.push(cell);
    }

    fn push_str(&mut self, s: &str) {
        self.text.push_str(s);
    }
}

pub struct Context {
    hldefs: HighlightDefinitions,
    serial1: usize,
}

pub struct LayoutLine<'a, 'b> {
    text: String,
    items: Vec<Item<'a>>,
    hldefs: &'b HighlightDefinitions,
    metrics: pango::FontMetrics,
    // glyphs: Vec<cairo::Glyph>,
    // clusters: Vec<cairo::TextCluster>,
}

impl<'a, 'b> LayoutLine<'a, 'b> {
    pub fn with(
        fm: &FontMap,
        tl: &'a TextLine,
        hldefs: &'b HighlightDefinitions,
    ) -> LayoutLine<'a, 'b> {
        let mut buf = harfbuzz::Buffer::new();
        let (mut items, text) = fm.itemize(tl, hldefs);
        /* let (glyphs, clusters) =*/
        fm.shape(&mut buf, &text, &mut items);
        let metrics = fm.metrics().unwrap();
        LayoutLine {
            text,
            items,
            hldefs,
            metrics,
            // glyphs,
            // clusters,
        }
    }

    pub fn show(&self, cr: &cairo::Context) -> anyhow::Result<()> {
        // log::info!("show text glyphs");
        // println!("{} {:?}", self.glyphs.len(), &self.glyphs);
        // println!("{} {:?}", self.clusters.len(), &self.clusters);
        // assert!(self.glyphs.len() == self.clusters.len());
        let mut x = 0.;
        let height = self.metrics.height() as f64;
        let y = 0.;
        for item in self.items.iter() {
            let defaults = self.hldefs.defaults().unwrap();
            let hldef = item.cells[0].hldef.unwrap_or(HighlightDefinitions::DEFAULT);
            let hldef = self.hldefs.get(hldef).unwrap();
            let width =
                item.cells.len() as f64 * self.metrics.approximate_digit_width() as f64 / 16.;

            if let Some(background) = hldef.background() {
                cr.set_source_rgba(
                    background.red() as _,
                    background.green() as _,
                    background.blue() as _,
                    background.alpha() as _,
                );
                cr.save()?;
                cr.rectangle(x, y, width, height);
                cr.fill()?;
                cr.restore()?;
            }
            x += width as f64;
            let foreground = hldef.foreground(&defaults);
            cr.set_source_rgba(
                foreground.red() as _,
                foreground.green() as _,
                foreground.blue() as _,
                foreground.alpha() as _,
            );
            cr.set_scaled_font(&item.scaled_font);
            cr.show_text_glyphs(
                &item.text,
                &item.glyphs,
                &item.clusters,
                cairo::TextClusterFlags::None,
            )?;
            if hldef.strikethrough {
                // TODO:
            }
            if hldef.underline {
                // TODO
            }
            if hldef.undercurl {
                // TODO
            }
        }
        Ok(())
    }
}

impl FontMap {
    fn itemize<'a>(
        &self,
        tl: &'a TextLine,
        hldefs: &HighlightDefinitions,
    ) -> (Vec<Item<'a>>, String) {
        let mut items = Vec::new();
        let mut text = String::new();
        if tl.is_empty() {
            return (items, text);
        }
        fn _font(
            fm: &FontMap,
            hldefs: &HighlightDefinitions,
            cell: &TextCell,
            c: char,
        ) -> (harfbuzz::Font, cairo::ScaledFont) {
            let hldef = cell.hldef.unwrap_or(HighlightDefinitions::DEFAULT);
            let style = hldefs.get(hldef).unwrap();
            if !is_keycap_base(c) && c.is_emoji() {
                fm.emoji.font(c as u32).unwrap()
            } else if style.italic && style.bold {
                fm.bold_italic.font(c as u32).unwrap()
            } else if style.italic {
                fm.italic.font(c as u32).unwrap()
            } else if style.bold {
                fm.bold.font(c as u32).unwrap()
            } else {
                fm.regular.font(c as u32).unwrap()
            }
        }
        let mut iter = tl.iter();
        let cell = iter.next().unwrap();
        let (hb_font, scaled_font) = self.regular().font('a' as u32).unwrap();
        let item = Item::new(String::new(), hb_font, scaled_font, cell);
        items.push(item.clone());
        for (idx, c) in cell.text.chars().enumerate() {
            let (hb_font, scaled_font) = _font(&self, hldefs, cell, c);
            let last_mut = items.last_mut().unwrap();
            if idx == 0 {
                last_mut.hb_font = hb_font;
                last_mut.scaled_font = scaled_font;
            } else if !scaled_font.is_same(&last_mut.scaled_font) {
                panic!("{:?} with different font, {:?}", cell, tl);
            }
            last_mut.text.push(c);
        }
        text.push_str(&cell.text);
        for cell in iter {
            if cell.text.is_empty() {
                items.last_mut().unwrap().push_cell(cell);
                continue;
            }
            let item = item.with_cell(cell);

            let last = items.last().unwrap();

            let mut chars = cell.text.chars();
            let c = chars.next().unwrap();
            let (hb, scaled_font) = _font(&self, &hldefs, &cell, c);
            if !scaled_font.is_same(&last.scaled_font) {
                items.push(item.with_font(hb, scaled_font));
            }
            let last = items.last().unwrap();
            for c in chars {
                // let (_, scaled_font) = _font(&self, &hldefs, &cell, c);
                // if !scaled_font.is_same(&last.scaled_font) {
                //     panic!("{:?} with different font, {:?}", cell, tl);
                // }
            }
            let last = items.last_mut().unwrap();
            text.push_str(&cell.text);
            last.push_str(&cell.text);
            last.push_cell(&cell);
        }
        (items, text)
    }

    pub fn shape(&self, buf: &mut harfbuzz::Buffer, text: &str, items: &mut [Item]) {
        buf.clear_contents();
        buf.add_str(&text, 0, None);
        buf.guess_segment_properties();
        buf.clear_contents();

        // let mut glyphs = Vec::new();
        // let mut clusters = Vec::new();

        let mut x = 0.;
        let mut y = 0.;

        let mut start_at = 0;
        let iter = items.iter_mut().peekable();
        // TODO: shape only with preview and next item.
        for item in iter {
            let (glyphs_, clusters_) = self.shape_(
                &item.hb_font,
                buf,
                text,
                start_at,
                item.text.len(),
                &mut x,
                &mut y,
            );
            item.glyphs = glyphs_;
            item.clusters = clusters_;
            start_at += item.text.len();
        }

        // (glyphs, clusters)
    }

    fn shape_(
        &self,
        font: &harfbuzz::Font,
        buf: &mut harfbuzz::Buffer,
        text: &str,
        start_at: usize,
        end_at: usize,
        x: &mut f64,
        y: &mut f64,
    ) -> (Vec<cairo::Glyph>, Vec<cairo::TextCluster>) {
        if text.is_empty() {
            return (Vec::new(), Vec::new());
        }

        buf.clear_contents();
        buf.add_str(text, start_at, Some(end_at));
        buf.guess_segment_properties();
        buf.set_direction(harfbuzz::Direction::LTR);

        harfbuzz::shape(font, buf, &[]);

        let num_glyphs = buf.len();
        let glyph_infos = buf.glyph_infos();
        let glyph_positions = buf.glyph_positions();

        buf.clear_contents();

        let mut glyphs = Vec::with_capacity(num_glyphs + 1);

        let mut num_clusters = if num_glyphs > 0 { 1 } else { 0 };
        for i in 1..num_glyphs {
            unsafe {
                if glyph_infos.get_unchecked(i).cluster()
                    != glyph_infos.get_unchecked(i - 1).cluster()
                {
                    num_clusters += 1;
                }
            }
        }

        let mut clusters = vec![cairo::TextCluster::new(0, 0); num_clusters];

        let scale_bits = -6;

        /*
        log::info!("{}", text);
        log::info!(
            "text '{}', start_at {}, end_at {} x {} y {} glyph infos {:?}, glyph positions: {:?}",
            &text[start_at..start_at + end_at],
            start_at,
            end_at,
            x,
            y,
            &glyph_infos,
            &glyph_positions
        );
        */
        // let scale_bits = 0;
        for (glyph_info, position) in glyph_infos.iter().zip(glyph_positions.iter()) {
            let index = glyph_info.codepoint() as u64;
            let x_ = libm::scalbn(position.x_offset() as f64 + *x, scale_bits);
            let y_ = libm::scalbn(-position.y_offset() as f64 + *y, scale_bits);
            // println!("glyph {{ index: {index}, x: {x_}, y: {y_} }}", x_=x_ / 64.);
            glyphs.push(cairo::Glyph::new(index, x_ / 16., y_));

            *x += position.x_advance() as f64;
            *y += -position.y_advance() as f64;
        }

        // glyphs.push(cairo::Glyph::new(
        //     u64::MAX,
        //     libm::scalbn(x as f64, scale_bits),
        //     libm::scalbn(y as f64, scale_bits),
        // ));

        // unicode_segmentation;

        if num_clusters > 0 {
            let mut index = 0;
            let mut bytes = 0;
            let mut c = unsafe { clusters.get_unchecked_mut(0) };
            c.set_num_glyphs(c.num_glyphs() + 1);

            for i in 1..num_glyphs {
                let cluster1 = unsafe { glyph_infos.get_unchecked(i) }.cluster();
                let cluster2 = unsafe { glyph_infos.get_unchecked(i - 1) }.cluster();
                c = unsafe { clusters.get_unchecked_mut(index) };
                if cluster1 != cluster2 {
                    assert!(cluster1 > cluster2);
                    let num_bytes = (cluster1 - cluster2) as i32;
                    // println!("{} - {} = {}", cluster1, cluster2, num_bytes);
                    c.set_num_bytes(num_bytes);
                    bytes += num_bytes;
                    index += 1;
                }
                c = unsafe { clusters.get_unchecked_mut(index) };
                c.set_num_glyphs(c.num_glyphs() + 1);
            }
            c = unsafe { clusters.get_unchecked_mut(index) };
            c.set_num_bytes(end_at as i32 - bytes);
        }

        (glyphs, clusters)
    }
}
