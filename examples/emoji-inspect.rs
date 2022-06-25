use glib::translate::ToGlibPtr;
use pango::prelude::*;
use pangocairo::traits::FontExt as PangoCairoFontExt;

fn main() {
    let fontmap = pangocairo::FontMap::default().unwrap();
    let ctx = fontmap.create_context().unwrap();
    let desc = pango::FontDescription::from_string("Cascadia Code PL 11");
    let font = fontmap.load_font(&ctx, &desc).unwrap();
    println!("load font {}", font.describe().unwrap());
    let metrics = unsafe { *(font.metrics(None).unwrap().to_glib_none().0) };
    println!(
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
    );
    let desc = pango::FontDescription::from_string("Emoji 11");
    let font = fontmap.load_font(&ctx, &desc).unwrap();
    println!("load font {}", font.describe().unwrap());
    let metrics = unsafe { *(font.metrics(None).unwrap().to_glib_none().0) };
    println!(
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
    );
    let face = font.face().unwrap();
    println!("face desc: {}", face.describe().unwrap());
    println!("face name: {}", face.face_name().unwrap());
    println!(
        "face sizes: {:?}",
        face.list_sizes()
            .into_iter()
            .map(|v| v as f64 / 1024.)
            .collect::<Vec<_>>()
    );
    let scaled_font = font
        .downcast::<pangocairo::Font>()
        .unwrap()
        .scaled_font()
        .unwrap();
    println!("scaled scale matrix {:?}", scaled_font.scale_matrix());
    println!("scaled font matrix {:?}", scaled_font.font_matrix());
    println!("scaled ctm {:?}", scaled_font.ctm());
    println!("scaled extents {:?}", scaled_font.extents());
}
