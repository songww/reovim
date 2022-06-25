use std::io::Write;

use gtk::prelude::FontExt;

fn main() {
    gtk::init().unwrap();
    const SCALE: f64 = pango::SCALE as f64;
    let fontmap = pangocairo::FontMap::default().unwrap();
    let desc = pango::FontDescription::from_string("Cascadia Code Light 13px");
    let ctx = pango::Context::new();
    ctx.set_font_map(&fontmap);
    ctx.set_font_description(&desc);
    let text = "I'm rv, a frontend for neovim editor.\n我是rv, neovim编辑器的GUI.";
    let attrs = pango::AttrList::new();
    attrs.insert({
        let mut attr = pango::AttrInt::new_style(pango::Style::Italic);
        attr.set_start_index(4);
        attr.set_end_index(6);
        attr
    });
    attrs.insert({
        let mut attr = pango::AttrInt::new_style(pango::Style::Italic);
        attr.set_start_index(44);
        attr.set_end_index(46);
        attr
    });
    attrs.insert({
        // eb4034
        let mut attr = pango::AttrColor::new_foreground(235u16.pow(2), 64u16.pow(2), 52u16.pow(2));
        attr.set_start_index(0);
        attr.set_end_index(text.len() as _);
        attr
    });
    let surface = cairo::ImageSurface::create(cairo::Format::ARgb32, 500, 50).unwrap();
    let cr = cairo::Context::new(&surface).unwrap();
    cr.set_source_rgb(0.1, 0.1, 0.1);
    cr.paint().unwrap();
    let mut xoffset = 0.;
    let mut yoffset = 0.;
    let itemized = pango::itemize(&ctx, text, 0, text.len() as _, &attrs, None);
    for item in itemized.iter() {
        let desc_ = item.analysis().font().describe().unwrap();
        let metrics = ctx.metrics(Some(&desc_), None).unwrap();
        let lineheight = metrics.height() as f64 / SCALE;
        let charwidth = metrics.approximate_digit_width();
        let start = item.offset() as usize;
        let end = (item.offset() + item.length()) as usize;
        yoffset = match start {
            38 => yoffset + lineheight,
            0..=37 => lineheight,
            _ => yoffset,
        };
        if start == 38 {
            xoffset = 0.;
        }
        cr.move_to(xoffset, yoffset);
        xoffset += (charwidth * item.length()) as f64 / SCALE;
        let ch = &text[start..end];
        println!(
            "text[{}:{}] => '{}' with font {} height {} char-width {} digit-width {} at {}x{}",
            start,
            end,
            ch,
            desc_.to_str(),
            metrics.height(),
            metrics.approximate_char_width(),
            metrics.approximate_digit_width(),
            xoffset,
            yoffset
        );
        let mut glyphs = pango::GlyphString::new();
        pango::shape(ch, item.analysis(), &mut glyphs);
        // pangocairo::show_(&cr, &item.analysis().font(), &mut glyphs);
    }
    cr.paint().unwrap();

    // let layout = pangocairo::create_layout(&cr).unwrap();
    // layout.set_font_description(Some(&desc));
    // layout.set_text(text);
    // layout.set_attributes(Some(&attrs));
    // pangocairo::show_layout(&cr, &layout);

    let mut f = std::fs::File::options()
        .write(true)
        .create(true)
        .open("text.png")
        .unwrap();
    surface.write_to_png(&mut f).unwrap();
    f.flush().unwrap();
}
