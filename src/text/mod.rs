use std::rc::Rc;

// use pangocairo::prelude::FontExt as PangoCairoFontExt;
use glib::translate::ToGlibPtr;
use glib::Cast;
use once_cell::sync::Lazy;
use pango::prelude::{FontExt, FontMapExt, FontsetExt};
use pangocairo::traits::FontExt as PangoCairoFontExt;

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
    static Font: Lazy<Builtin> = Lazy::new(|| {
        let blob = harfbuzz::Blob::new_read_only(BOX_DRAWING);
        let mut face = harfbuzz::Face::new(&blob, 0);
        let hb = harfbuzz::Font::new(&mut face);
        let ft = freetype::Library::init().unwrap();
        let data: Vec<u8> = BOX_DRAWING.to_vec();
        let ftface = ft.new_memory_face(data, 0).unwrap();
        Builtin { hb, ft: ftface }
    });
    let ft = Font.ft.clone();
    let mut hb = Font.hb.clone();
    hb.set_ptem(ptem);
    ft.set_char_size((ptem * 64.) as isize, 0, 0, 0);
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
        face.set_user_data(&FT_FACE, Rc::new(self.ft.clone()));
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
}

pub struct FontSet(Builtin, pango::Fontset);

impl FontSet {
    pub fn font(&self, wc: u32) -> Option<cairo::ScaledFont> {
        if Builtin::has_char(char::from_u32(wc).unwrap_or('a')) {
            self.0.scaled_font().ok()
        } else {
            self.1
                .font(wc)
                .and_then(|f| f.downcast().ok())
                .and_then(|f: pangocairo::Font| f.scaled_font())
        }
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

    buf: harfbuzz::Buffer,
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
        let regular = fontmap.load_fontset(&ctx, &regular, &language).unwrap();
        let bold = fontmap.load_fontset(&ctx, &bold, &language).unwrap();
        let italic = fontmap.load_fontset(&ctx, &italic, &language).unwrap();
        let bold_italic = fontmap.load_fontset(&ctx, &bold_italic, &language).unwrap();

        let buf = harfbuzz::Buffer::new();

        FontMap {
            regular: FontSet::from(regular),
            bold: FontSet::from(bold),
            italic: FontSet::from(italic),
            bold_italic: FontSet::from(bold_italic),

            buf,
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
}

pub type Nr = usize;

#[derive(Debug, Clone)]
pub struct Item<'a> {
    text: String,
    font: cairo::ScaledFont,
    cells: Vec<&'a TextCell>,
}

impl<'a> Item<'a> {
    fn new(text: String, font: cairo::ScaledFont, cell: &'a TextCell) -> Self {
        Item {
            text,
            font,
            cells: vec![cell],
        }
    }
    fn with_font(&self, font: cairo::ScaledFont) -> Self {
        Item {
            text: self.text.clone(),
            font,
            cells: self.cells.clone(),
        }
    }

    fn with_cell(&self, cell: &'a TextCell) -> Self {
        Item {
            text: self.text.clone(),
            font: self.font.clone(),
            cells: vec![cell],
        }
    }

    fn push_cell(&mut self, cell: &'a TextCell) {
        self.cells.push(cell);
    }

    fn push_str(&mut self, s: &str) {
        self.text.push_str(s);
    }
}

pub struct LayoutLine<'a> {
    text: String,
    items: Vec<Item<'a>>,
    glyphs: Vec<cairo::Glyph>,
    clusters: Vec<cairo::TextCluster>,
}

impl<'a> LayoutLine<'a> {
    pub fn with(
        fm: &mut FontMap,
        tl: &'a TextLine,
        hldefs: &HighlightDefinitions,
    ) -> LayoutLine<'a> {
        let (items, text) = fm.itemize(tl, hldefs);
        let (glyphs, clusters) = fm.shape(&text, &items);
        LayoutLine {
            text,
            items,
            glyphs,
            clusters,
        }
    }

    pub fn render(&self, cr: &cairo::Context) -> anyhow::Result<()> {
        cr.show_text_glyphs(
            &self.text,
            &self.glyphs,
            &self.clusters,
            cairo::TextClusterFlags::None,
        )?;
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
        fn scaled_font(
            fm: &FontMap,
            hldefs: &HighlightDefinitions,
            cell: &TextCell,
            c: char,
        ) -> cairo::ScaledFont {
            let hldef = cell.hldef.unwrap_or(HighlightDefinitions::DEFAULT);
            let style = hldefs.get(hldef).unwrap();
            if style.italic && style.bold {
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
        let item = Item::new(
            String::new(),
            self.regular().font('a' as u32).unwrap(),
            cell,
        );
        items.push(item.clone());
        for (idx, c) in cell.text.chars().enumerate() {
            let font = scaled_font(&self, hldefs, cell, c);
            if idx == 0 {
                items.last_mut().unwrap().font = font;
            } else if !font.is_same(&items.last().unwrap().font) {
                panic!("{:?} with different font, {:?}", cell, tl);
            }
            items.last_mut().unwrap().text.push(c);
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
            let font = scaled_font(&self, &hldefs, &cell, c);
            if !font.is_same(&last.font) {
                items.push(item.with_font(font));
            }
            let last = items.last().unwrap();
            for c in chars {
                let font = scaled_font(&self, &hldefs, &cell, c);
                if !font.is_same(&last.font) {
                    panic!("{:?} with different font, {:?}", cell, tl);
                }
            }
            let last = items.last_mut().unwrap();
            text.push_str(&cell.text);
            last.push_str(&cell.text);
            last.push_cell(&cell);
        }
        (items, text)
    }

    pub fn shape(
        &mut self,
        text: &str,
        items: &[Item],
    ) -> (Vec<cairo::Glyph>, Vec<cairo::TextCluster>) {
        self.buf.clear_contents();
        self.buf.add_str(&text, 0, None);
        self.buf.guess_segment_properties();
        self.buf.clear_contents();

        let mut glyphs = Vec::new();
        let mut clusters = Vec::new();

        let mut x = 0.;
        let mut y = 0.;

        let mut start_at = 0;
        let iter = items.iter().peekable();
        for item in iter {
            let (glyphs_, clusters_) =
                self.shape_(text, start_at, start_at + item.text.len(), &mut x, &mut y);
            glyphs.extend(glyphs_);
            clusters.extend(clusters_);
            start_at += item.text.len();
        }

        (glyphs, clusters)
    }

    fn shape_(
        &mut self,
        text: &str,
        start_at: usize,
        end_at: usize,
        x: &mut f64,
        y: &mut f64,
    ) -> (Vec<cairo::Glyph>, Vec<cairo::TextCluster>) {
        if text.is_empty() {
            return (Vec::new(), Vec::new());
        }

        self.buf.clear_contents();
        self.buf.add_str(text, start_at, Some(end_at));
        self.buf.guess_segment_properties();
        self.buf.set_direction(harfbuzz::Direction::LTR);

        let num_glyphs = self.buf.len();
        let glyph_infos = self.buf.glyph_infos();
        let glyph_positions = self.buf.glyph_positions();

        self.buf.clear_contents();

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

        let position = &glyph_positions[0];
        for glyph_info in glyph_infos.iter() {
            let index = glyph_info.codepoint() as u64;
            let x_ = libm::scalbn(position.x_offset() as f64 + *x, scale_bits);
            let y_ = libm::scalbn(-position.x_offset() as f64 + *y, scale_bits);
            glyphs.push(cairo::Glyph::new(index, x_, y_));

            *x += position.x_advance() as f64;
            *y += -position.y_advance() as f64;
        }

        // glyphs.push(cairo::Glyph::new(
        //     u64::MAX,
        //     libm::scalbn(x as f64, scale_bits),
        //     libm::scalbn(y as f64, scale_bits),
        // ));

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
                    c.set_num_bytes(num_bytes);
                    bytes += num_bytes;
                    index += 1;
                }
                c.set_num_glyphs(c.num_glyphs() + 1);
            }
            c = unsafe { clusters.get_unchecked_mut(index) };
            c.set_num_bytes(text.len() as i32 - bytes);
        }

        (glyphs, clusters)
    }
}
