use std::rc::Rc;
use std::str::FromStr;

use glib::translate::ToGlibPtr;
use glib::Cast;
use once_cell::sync::Lazy;
use pango::prelude::{FontExt, FontMapExt, FontsetExt};
use pangocairo::traits::FontExt as PangoCairoFontExt;
use xi_unicode::{is_keycap_base, EmojiExt};

mod attributes;
mod layout;
mod text_cell;
mod text_line;

use crate::metrics::Metrics;

pub use text_cell::TextCell;
pub use text_line::TextLine;

use crate::vimview::HighlightDefinitions;

static BOX_DRAWING: &[u8] = include_bytes!(concat!(
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

const PANGO_SCALE: f64 = pango::SCALE as f64;

#[derive(Clone)]
pub struct Builtin {
    hb: harfbuzz::FontMut,
    ft: freetype::face::Face,
    scaled_font: Option<cairo::ScaledFont>,
}

unsafe impl Send for Builtin {}
unsafe impl Sync for Builtin {}

pub fn builtin(ptem: f32, x_scale: i32, y_scale: i32) -> Builtin {
    static FONT: Lazy<Builtin> = Lazy::new(|| {
        let blob = harfbuzz::Blob::new_read_only(BOX_DRAWING);
        let mut face = harfbuzz::Face::new(&blob, 0);
        let hb = harfbuzz::Font::new(&mut face);
        let ft = freetype::Library::init().unwrap();
        let data: Vec<u8> = BOX_DRAWING.to_vec();
        let ftface = ft.new_memory_face(data, 0).unwrap();
        Builtin {
            hb,
            ft: ftface,
            scaled_font: None,
        }
    });
    let ft = FONT.ft.clone();
    let mut hb = FONT.hb.clone();
    log::info!("ptem {}", ptem);
    hb.set_ptem(ptem);
    hb.set_scale(x_scale, y_scale);
    ft.set_pixel_sizes(0, to_freetype_26_6(ptem as f64) as u32)
        .unwrap();
    let scaled_font = scaled_font(&ft, &hb);

    Builtin {
        hb,
        ft,
        scaled_font: scaled_font.into(),
    }
}

fn scaled_font(ft: &freetype::Face, hb: &harfbuzz::Font) -> cairo::ScaledFont {
    static FT_FACE: cairo::UserDataKey<freetype::face::Face> = cairo::UserDataKey::new();
    // TODO: variations
    let face = cairo::FontFace::create_from_ft(ft).unwrap();
    face.set_user_data(&FT_FACE, Rc::new(ft.clone())).unwrap();
    let upem = hb.ptem() as f64 / 72. * 96.;
    let (sx, sy) = (upem, upem);
    let ctm = cairo::Matrix::identity();
    let font_matrix = cairo::Matrix::new(sx, 0., 0., sy, 0., 0.);
    let mut options = cairo::FontOptions::new().unwrap();
    options.set_hint_style(cairo::HintStyle::None);
    options.set_hint_metrics(cairo::HintMetrics::Off);
    cairo::ScaledFont::new(&face, &font_matrix, &ctm, &options).unwrap()
}

impl Builtin {
    pub fn has_char(c: char) -> bool {
        ('\u{2500}'..='\u{257f}').contains(&c)
    }
    fn scaled_font(&self) -> Option<cairo::ScaledFont> {
        self.scaled_font.clone()
    }
}

impl FontExtManual for Builtin {
    fn hb(&self) -> harfbuzz::Font {
        self.hb.clone().into_immutable()
    }
}

trait FontExtManual {
    fn hb(&self) -> harfbuzz::Font;
}

impl FontExtManual for pangocairo::Font {
    fn hb(&self) -> harfbuzz::Font {
        let font = self.upcast_ref::<pango::Font>();
        let hb = unsafe {
            let raw = pango::ffi::pango_font_get_hb_font(font.to_glib_none().0);
            let raw = harfbuzz::sys::hb_font_reference(raw as *mut _);
            harfbuzz::FontMut::from_raw(raw)
        };
        hb.into_immutable()
    }
}

#[inline]
fn to_freetype_26_6(f: f64) -> isize {
    ((1i32 << 6) as f64 * f).round() as isize
}

/*
trait HBFontExt {
    fn scaled_font(&mut self) -> Rc<cairo::ScaledFont>;
}

impl HBFontExt for harfbuzz::Font {
    fn scaled_font(&mut self) -> Rc<cairo::ScaledFont> {
        static HBFONT_KEY: cairo::UserDataKey<hb::Font> = cairo::UserDataKey::new();
        static mut SCALED_FONT_KEY: harfbuzz::UserDataKey<cairo::ScaledFont> =
            harfbuzz::UserDataKey::new();
        if let Some(scaled_font) = self.user_data(unsafe { &mut SCALED_FONT_KEY }) {
            // log::info!("strong count {}", Rc::strong_count(&scaled_font));
            // log::info!("weak count {}", Rc::weak_count(&scaled_font));
            return scaled_font;
        }
        log::info!("----------------------------------------");
        // static FT_FACE: cairo::UserDataKey<freetype::face::Face> = cairo::UserDataKey::new();
        let face = self.face();
        let ptem = self.ptem();
        let upem = face.upem();
        let (x_scale, y_scale) = self.scale();
        log::info!("x-scale {x_scale} y-scale {y_scale}");
        let (x_ppem, y_ppem) = self.ppem();
        log::info!("x-ppem {x_ppem} y-ppem {y_ppem}");
        log::info!("upem {upem} ptem {ptem}");
        let upem = ptem as f64 / 72. * 96.;
        log::info!("upem {upem}");
        let face_index = face.index();
        log::info!("face index {}", face_index);
        let blob = face.reference_blob();
        log::info!("blob {:p}", blob.as_raw());
        let data = blob.data();
        log::info!("data {}", data.len());
        let libary = freetype::Library::init().unwrap();
        let face = libary
            .new_memory_face(data.to_vec(), face_index as _)
            .unwrap();
        log::info!("char size {}", to_freetype_26_6(upem));
        face.set_char_size(to_freetype_26_6(upem), 0, 0, 0).unwrap();
        let face = cairo::FontFace::create_from_ft(&face).unwrap();
        // TODO: variations
        // let (sx, sy) = (upem, upem);
        let ctm = cairo::Matrix::identity();
        let font_matrix = cairo::Matrix::new(upem, 0., 0., upem, 0., 0.);
        let mut options = cairo::FontOptions::new().unwrap();
        options.set_hint_style(cairo::HintStyle::None);
        options.set_hint_metrics(cairo::HintMetrics::Off);
        let scaled_font = cairo::ScaledFont::new(&face, &font_matrix, &ctm, &options).unwrap();
        log::info!("scaled font created.");
        let scaled_font = std::rc::Rc::new(scaled_font);
        self.set_user_data(unsafe { &mut SCALED_FONT_KEY }, scaled_font.clone(), true)
            .unwrap();
        scaled_font
            .set_user_data(&HBFONT_KEY, Rc::new(self.clone()))
            .unwrap();
        scaled_font
    }
}
*/

#[derive(Clone)]
pub enum Font {
    Builtin(Builtin),
    Pango(pangocairo::Font),
}

impl Font {
    fn hb(&self) -> harfbuzz::Font {
        match self {
            Font::Builtin(builtin) => builtin.hb(),
            Font::Pango(pango) => pango.hb(),
        }
    }

    fn scaled_font(&self) -> Option<cairo::ScaledFont> {
        match self {
            Font::Builtin(builtin) => builtin.scaled_font(),
            Font::Pango(pango) => pango.scaled_font(),
        }
    }

    fn desc(&self) -> pango::FontDescription {
        match self {
            Font::Builtin(_) => pango::FontDescription::from_string("builtin box drawing"),
            Font::Pango(pango) => pango.describe().unwrap(),
        }
    }
}

impl From<Builtin> for Font {
    fn from(builtin: Builtin) -> Font {
        Font::Builtin(builtin)
    }
}

impl From<pangocairo::Font> for Font {
    fn from(pango: pangocairo::Font) -> Self {
        Font::Pango(pango)
    }
}

pub struct FontSet(Builtin, pango::Fontset);

impl FontSet {
    pub fn font(&self, text: &str) -> Option<Font> {
        let is_builtin = text.chars().all(Builtin::has_char);
        if is_builtin {
            log::info!("-> using builtin font for '{}'", text);
            Some(self.0.clone().into())
        } else {
            let mut font = None;
            self.1.foreach(|_fs, f| {
                if text.chars().all(|c| f.has_char(c)) {
                    font = f.clone().into();
                    true
                } else {
                    false
                }
            });
            font.inspect(|f| {
                log::info!(
                    "-> using `{}` for '{}'",
                    f.describe().unwrap(),
                    text,
                    // inspect(&f.metrics(None).unwrap())
                );
            })
            .and_then(|f| f.downcast::<pangocairo::Font>().ok())
            .map(Into::into)
        }
    }

    pub fn metrics(&self) -> Option<pango::FontMetrics> {
        self.1.metrics()
    }
}

impl From<pango::Fontset> for FontSet {
    fn from(set: pango::Fontset) -> Self {
        let font = set.font(65).unwrap();
        log::info!("------------------");
        log::info!("-> font {}", font.describe().unwrap());
        let scaled_font = font
            .downcast_ref::<pangocairo::Font>()
            .unwrap()
            .scaled_font()
            .unwrap();
        log::info!("------------------");
        let mut x_scale = 0;
        let mut y_scale = 0;
        let ptem = unsafe {
            let hbfont = pango::ffi::pango_font_get_hb_font(font.to_glib_none().0);
            harfbuzz::sys::hb_font_get_scale(hbfont as *mut _, &mut x_scale, &mut y_scale);
            log::info!("font scale ({}, {})", x_scale, y_scale);
            let (mut x_ppem, mut y_ppem) = (0, 0);
            harfbuzz::sys::hb_font_get_ppem(hbfont as *mut _, &mut x_ppem, &mut y_ppem);
            log::info!("font ppem ({}, {})", x_ppem, y_ppem);
            let hbface = harfbuzz::sys::hb_font_get_face(hbfont as *mut _);
            let upem = harfbuzz::sys::hb_face_get_upem(hbface);
            log::info!("font upem {}", upem);
            harfbuzz::sys::hb_font_get_ptem(hbfont as *mut _)
        };
        log::info!("ptem {}", ptem);
        log::info!("scaled font extents: {:?}", scaled_font.extents());
        log::info!("scaled font ctm: {:?}", scaled_font.ctm());
        log::info!("scaled font font matrix: {:?}", scaled_font.font_matrix());
        log::info!("scaled font scale matrix: {:?}", scaled_font.scale_matrix());
        let builtin = builtin(ptem, x_scale, y_scale);
        let scaled_font = builtin.scaled_font().unwrap();
        log::info!("builtin font scale {:?}", builtin.hb().scale());
        log::info!("builtin scaled font extents: {:?}", scaled_font.extents());
        log::info!("builtin scaled font ctm: {:?}", scaled_font.ctm());
        log::info!(
            "builtin scaled font font matrix: {:?}",
            scaled_font.font_matrix()
        );
        log::info!(
            "builtin scaled font scale matrix: {:?}",
            scaled_font.scale_matrix()
        );
        FontSet(builtin, set)
    }
}

fn inspect(metrics: &pango::FontMetrics) -> String {
    let metrics = unsafe { *(metrics.to_glib_none().0) };
    format!(
        "PangoFontMetrics {{
    ref_count: {},
    ascent: {},
    descent: {},
    height: {},
    approximate_char_width: {},
    approximate_digit_width: {},
    underline_position: {},
    underline_thickness: {},
    strikethrough_position: {},
    strikethrough_thickness: {},
}}",
        metrics.ref_count,
        metrics.ascent as f64 / 1024.,
        metrics.descent as f64 / 1024.,
        metrics.height as f64 / 1024.,
        metrics.approximate_char_width as f64 / 1024.,
        metrics.approximate_digit_width as f64 / 1024.,
        metrics.underline_position as f64 / 1024.,
        metrics.underline_thickness as f64 / 1024.,
        metrics.strikethrough_position as f64 / 1024.,
        metrics.strikethrough_thickness as f64 / 1024.
    )
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

        log::debug!("regular {}", inspect(&regular.metrics().unwrap()));
        let font = regular.font(65 /* A */).unwrap();
        let regular_desc = font.describe().unwrap();
        log::debug!("regular size {}", regular_desc.size());
        log::debug!("regular weight {}", regular_desc.weight());
        log::debug!("regular stretch {}", regular_desc.stretch());
        log::debug!("regular variant {}", regular_desc.variant());
        log::debug!(
            "regular variations {}",
            regular_desc.variations().unwrap_or_else(|| "".into())
        );
        log::debug!("regular style {}", regular_desc.style());
        log::debug!("regular filename {}", regular_desc.to_filename().unwrap());
        log::debug!("bold {}", inspect(&bold.metrics().unwrap()));
        log::debug!("italic {}", inspect(&italic.metrics().unwrap()));
        log::debug!("bold italic {}", inspect(&bold_italic.metrics().unwrap()));
        log::debug!("emoji {}", inspect(&emoji.metrics().unwrap()));

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
    font: Font,
    cells: Vec<&'a TextCell>,
    glyphs: Vec<cairo::Glyph>,
    clusters: Vec<cairo::TextCluster>,
}

impl<'a> Item<'a> {
    fn new(text: String, font: Font, cell: &'a TextCell) -> Self {
        Item {
            text,
            font,
            cells: vec![cell],
            glyphs: Vec::new(),
            clusters: Vec::new(),
        }
    }
    fn with_font(&self, font: Font) -> Self {
        Item {
            text: self.text.clone(),
            font,
            cells: self.cells.clone(),
            glyphs: Vec::new(),
            clusters: Vec::new(),
        }
    }

    fn with_cell(&self, cell: &'a TextCell) -> Self {
        Item {
            text: self.text.clone(),
            font: self.font.clone(),
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
    // text: String,
    items: Vec<Item<'a>>,
    hldefs: &'b HighlightDefinitions,
    metrics: Metrics,
}

impl<'a, 'b> LayoutLine<'a, 'b> {
    pub fn with(
        fm: &FontMap,
        tl: &'a TextLine,
        hldefs: &'b HighlightDefinitions,
        metrics: Metrics,
    ) -> LayoutLine<'a, 'b> {
        let mut buf = harfbuzz::Buffer::new();
        let (mut items, text) = fm.itemize(tl, hldefs);
        /* let (glyphs, clusters) =*/
        fm.shape(&mut buf, &text, &mut items);
        LayoutLine {
            // text,
            items,
            hldefs,
            metrics,
            // glyphs,
            // clusters,
        }
    }

    pub fn show(&self, cr: &cairo::Context) -> anyhow::Result<()> {
        cr.save()?;
        // log::info!("show text glyphs");
        // log::info!("{} {:?}", self.glyphs.len(), &self.glyphs);
        // log::info!("{} {:?}", self.clusters.len(), &self.clusters);
        // assert!(self.glyphs.len() == self.clusters.len());
        // log::debug!("text scales {:?}", cr.target().device_scale());
        let mut options = cairo::FontOptions::new().unwrap();
        options.set_hint_style(cairo::HintStyle::None);
        options.set_hint_metrics(cairo::HintMetrics::Off);
        cr.set_font_options(&options);
        let y = -self.metrics.ascent();
        let x = 0.;
        let height = self.metrics.height() as f64;
        for item in self.items.iter() {
            let defaults = self.hldefs.defaults().unwrap();
            let hldef = item.cells[0].hldef.unwrap_or(HighlightDefinitions::DEFAULT);
            let hldef = self.hldefs.get(hldef).unwrap();
            let width = item.cells.len() as f64 * self.metrics.charwidth();

            if let Some(background) = hldef.background() {
                cr.save()?;
                cr.set_source_rgba(
                    background.red() as _,
                    background.green() as _,
                    background.blue() as _,
                    background.alpha() as _,
                );
                cr.rectangle(x, y, width, height);
                cr.fill()?;
                cr.restore()?;
            }
            // x += width as f64;
            let foreground = hldef.foreground(defaults);
            cr.set_source_rgba(
                foreground.red() as _,
                foreground.green() as _,
                foreground.blue() as _,
                foreground.alpha() as _,
            );
            log::debug!("using '{}' for '{}'", item.font.desc(), item.text);

            cr.set_scaled_font(&item.font.scaled_font().unwrap());

            let mut index = 0;
            let mut sbytes = 0;

            for (glyph, cluster) in item.glyphs.iter().zip(item.clusters.iter()) {
                let mut nbyte = 0;
                let mut ncell = 0;
                // println!("index {}", index);
                // println!("sbytes {}", sbytes);
                // println!("num bytes {}", cluster.num_bytes());
                // println!("text len {}", item.text.len());
                while nbyte < cluster.num_bytes() as usize {
                    // println!("ncell {}", ncell);
                    nbyte += item.cells[ncell + index].text.len();
                    // println!(
                    //     "text {} len {}",
                    //     item.cells[ncell + index].text,
                    //     item.cells[ncell + index].text.len()
                    // );
                    ncell += 1;
                    // println!("nbyte {}", nbyte);
                }
                nbyte = cluster.num_bytes() as usize;
                // println!("\"{}\"[{}:{}]", &item.text, sbytes, sbytes + nbyte,);
                cr.show_text_glyphs(
                    &item.text[sbytes..sbytes + nbyte],
                    &[*glyph],
                    &[*cluster],
                    cairo::TextClusterFlags::None,
                )
                .unwrap_or_else(|_| {
                    panic!(
                        "\"{}\"[{}:{}] -> '{}' {}/{} {:?} {:?}",
                        &item.text,
                        sbytes,
                        sbytes + nbyte,
                        &item.text[sbytes..nbyte],
                        item.glyphs.len(),
                        item.clusters.len(),
                        glyph,
                        cluster
                    )
                });

                index += ncell;
                sbytes += nbyte;
                cr.translate(self.metrics.width() * ncell as f64, 0.);
            }
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
        cr.restore()?;
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
        fn _font(fm: &FontMap, hldefs: &HighlightDefinitions, cell: &TextCell, text: &str) -> Font {
            let hldef = cell.hldef.unwrap_or(HighlightDefinitions::DEFAULT);
            let style = hldefs.get(hldef).unwrap();

            let c = text.chars().next().unwrap();
            if !is_keycap_base(c) && c.is_emoji() {
                fm.emoji.font(text).unwrap()
            } else if style.italic && style.bold {
                fm.bold_italic.font(text).unwrap()
            } else if style.italic {
                fm.italic.font(text).unwrap()
            } else if style.bold {
                fm.bold.font(text).unwrap()
            } else {
                fm.regular.font(text).unwrap()
            }
        }
        let mut iter = tl.iter();
        let cell = iter.next().unwrap();
        let font = self.regular().font("a").unwrap();
        let item = Item::new(String::new(), font, cell);
        items.push(item.clone());
        let font = _font(self, hldefs, cell, &cell.text);
        let last_mut = items.last_mut().unwrap();
        last_mut.font = font;
        last_mut.text.push_str(&cell.text);
        text.push_str(&cell.text);

        let mut prevhldef = cell.hldef.unwrap_or(HighlightDefinitions::DEFAULT);

        for cell in iter {
            if cell.text.is_empty() {
                items.last_mut().unwrap().push_cell(cell);
                continue;
            }
            let item = item.with_cell(cell);

            let font = _font(self, hldefs, cell, &cell.text);
            let hldef = cell.hldef.unwrap_or(HighlightDefinitions::DEFAULT);
            if hldef != prevhldef {
                log::debug!("Item {}", hldef);
                items.push(item.with_font(font));
                prevhldef = hldef;
            }

            let last = items.last_mut().unwrap();
            text.push_str(&cell.text);
            last.push_str(&cell.text);
            last.push_cell(cell);
        }
        (items, text)
    }

    pub fn shape(&self, buf: &mut harfbuzz::Buffer, text: &str, items: &mut [Item]) {
        let mut start_at = 0;
        let iter = items.iter_mut().peekable();
        // TODO: shape only with preview and next item.
        for item in iter {
            let end_at = start_at + item.text.len();
            let (glyphs_, clusters_) = self.shape_(&item.font.hb(), buf, text, start_at, end_at);
            log::debug!(
                "shaping {} '{}'",
                end_at - start_at,
                &text[start_at..end_at]
            );
            item.glyphs = glyphs_;
            item.clusters = clusters_;
            start_at = end_at;
        }
    }

    fn shape_(
        &self,
        font: &harfbuzz::Font,
        buf: &mut harfbuzz::Buffer,
        text: &str,
        start_at: usize,
        end_at: usize,
    ) -> (Vec<cairo::Glyph>, Vec<cairo::TextCluster>) {
        if text.is_empty() {
            return (Vec::new(), Vec::new());
        }

        buf.clear_contents();
        buf.add_str(text, start_at, Some(end_at - start_at));
        buf.set_direction(harfbuzz::Direction::LTR);
        buf.set_flags(harfbuzz::BufferFlags::BOT | harfbuzz::BufferFlags::EOT);
        buf.guess_segment_properties();

        let features = vec![
            harfbuzz::Feature::from_str("calt=1").unwrap(),
            harfbuzz::Feature::from_str("ss02=1").unwrap(),
            harfbuzz::Feature::from_str("ss20=1").unwrap(),
        ];

        harfbuzz::shape(font, buf, &features);

        let num_glyphs = buf.len();
        let glyph_infos = buf.glyph_infos();
        let glyph_positions = buf.glyph_positions();

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

        // const SCALE_FACTOR: f64 = PANGO_SCALE / 72. * 96.;
        const SCALE_FACTOR: f64 = PANGO_SCALE;
        for (glyph_info, position) in glyph_infos.iter().zip(glyph_positions.iter()) {
            let index = glyph_info.codepoint() as u64;
            let x = position.x_offset() as f64 / SCALE_FACTOR;
            let y = -position.y_offset() as f64 / SCALE_FACTOR;
            // log::info!("glyph {{ index: {index}, x: {x_}, y: {y_} }}",);
            glyphs.push(cairo::Glyph::new(index, x, y));
        }

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
                    log::trace!("{} - {} = {}", cluster1, cluster2, num_bytes);
                    c.set_num_bytes(num_bytes);
                    bytes += num_bytes;
                    index += 1;
                }
                c = unsafe { clusters.get_unchecked_mut(index) };
                c.set_num_glyphs(c.num_glyphs() + 1);
            }
            c = unsafe { clusters.get_unchecked_mut(index) };
            c.set_num_bytes(end_at as i32 - start_at as i32 - bytes);
        }

        (glyphs, clusters)
    }
}
