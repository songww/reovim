use glib::translate::ToGlibPtr;
use harfbuzz_sys as hbsys;

fn main() {
    use pango::prelude::*;
    use pangocairo::prelude::FontMapExtManual;
    use pangocairo::traits::FontExt as PangoCairoFontExt;

    let fm: pangocairo::FontMap = pangocairo::FontMap::default().unwrap().downcast().unwrap();
    // println!("list family:");
    // for family in fm.list_families() {
    //     println!("{}", family.name().unwrap());
    // }
    println!("------------------------------------");
    let family = fm.family("Monospace").unwrap();
    println!("family name: {}", family.name().unwrap());
    let face = family.face(None).unwrap();
    println!("face name: {}", face.face_name().unwrap());
    println!("face sizes: {:?}", face.list_sizes());

    println!("------------------------------------");

    // let desc = pango::FontDescription::from_string("Monaco, Symbols Nerd Font Mono ExtraLight 11");
    let desc = pango::FontDescription::from_string("Cascadia Code Light 14");
    let ctx = fm.create_context().unwrap();
    let fontset = fm
        .load_fontset(&ctx, &desc, &pango::Language::default())
        .unwrap();
    fontset.foreach(|_set, font| {
        println!("-> {}", font.describe().unwrap());
        true
    });
    println!("\u{f2b3}");
    println!(
        "--> {}",
        fontset.font('\u{f307}' as u32).unwrap().describe().unwrap()
    );
    let font = fontset.font('\u{f307}' as u32).unwrap();
    let hb_font = unsafe { pango::ffi::pango_font_get_hb_font(font.to_glib_none().0) };
    let is_immutable = unsafe { hbsys::hb_font_is_immutable(hb_font as _) };
    println!("harfbuzz is immutable: {}", is_immutable);
    let hbface = unsafe { hbsys::hb_font_get_face(hb_font as _) };
    let upem = unsafe { hbsys::hb_face_get_upem(hbface as _) };
    println!("harfbuzz upem: {}", upem);
    let ptem = unsafe { hbsys::hb_font_get_ptem(hb_font as _) };
    println!("harfbuzz ptem: {}", ptem);
    let mut xppem = 0;
    let mut yppem = 0;
    unsafe { hbsys::hb_font_get_ppem(hb_font as _, &mut xppem, &mut yppem) };
    println!("harfbuzz ppem x: {} y: {}", xppem, yppem);
    let mut x_scale = 0;
    let mut y_scale = 0;
    unsafe { hbsys::hb_font_get_scale(hb_font as _, &mut x_scale, &mut y_scale) };
    println!("harfbuzz scale x: {} y: {}", x_scale, y_scale);
    println!("font type {}", fm.font_type());
    let font = font.downcast::<pangocairo::Font>().unwrap();
    let scaled_font = font.scaled_font().unwrap();
    println!("font_matrix: {:?}", scaled_font.font_matrix());
    println!("xx: {:?}", scaled_font.font_matrix().xx() / 96. * 72.);
    let metrics = fontset.metrics().unwrap();
    println!(
        "metrics digit width {} height {}",
        metrics.approximate_digit_width(),
        metrics.height()
    );
}
