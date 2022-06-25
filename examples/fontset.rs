use pango::prelude::{FontExt, FontMapExt, FontsetExt};

fn main() {
    let desc = pango::FontDescription::from_string("emoji 13");
    let fm = pangocairo::FontMap::default().unwrap();
    let ctx = fm.create_context().unwrap();
    let fset = fm
        .load_fontset(&ctx, &desc, &pango::Language::default())
        .unwrap();
    fset.foreach(|_fset, font| {
        println!("{}", font.describe().unwrap());
        false
    })
}
