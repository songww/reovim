use std::rc::Rc;

use once_cell::sync::Lazy;

mod attributes;
mod layout;
mod text_cell;
mod text_line;

pub use text_cell::TextCell;
pub use text_line::TextLine;

static BOX_DRAWING: &'static [u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/box-drawing.ttf"
));

pub struct Builtin {
    hb: harfbuzz::Font,
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
        let ftface = ft.new_memory_face2(BOX_DRAWING, 0).unwrap();
        Builtin { hb, ftface }
    });
    Font.hb.set_ptem(ptem);
    Font.ft.set_char_size((ptem * 64.) as isize, 0, 0, 0);
    Builtin {
        hb: Font.hb.clone(),
        ft: Font.ft.clone(),
    }
}

impl Builtin {
    pub fn has_char(c: char) -> bool {
        c >= '\u{2500}' && c <= '\u{257f}'
    }

    pub fn scaled_font(&self) -> anyhow::Result<cairo::ScaledFont> {
        static FT_FACE: cairo::UserDataKey<freetype::face::Face> = cairo::UserDataKey::new();
        // TODO: variations
        let face = cairo::FontFace::create_from_ft(self.ft)?;
        face.set_user_data(&FT_FACE, Rc::new(self.ft.clone()));
        let hb_face = self.hb.face();
        let upem = self.hb.ptem() as f64 / 72. * 96.;
        let (sx, sy) = (upem, upem);
        let ctm = cairo::Matrix::identity();
        let font_matrix = cairo::Matrix::new(sx, 0., 0., sy, 0., 0.);
        let mut options = cairo::FontOptions::new().unwrap();
        options.set_hint_style(cairo::HintStyle::None);
        options.set_hint_metrics(cairo::HintMetrics::Off);
        let scaled_font = cairo::ScaledFont::new(face, &font_matrix, &ctm, &options)?;
        Ok(scaled_font)
    }
}

pub type Nr = usize;

#[derive(Debug)]
pub struct Item;

pub fn itemize(grids: &[TextCell]) -> Vec<Item> {
    todo!()
}
