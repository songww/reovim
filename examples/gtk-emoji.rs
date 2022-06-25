use gtk::prelude::*;
use gtk::Application;

use pangocairo::traits::FontExt;

const APP_ID: &str = "org.gtk-rs.examples.gtk-emoji";

fn main() {
    // Create a new application
    let app = Application::builder().application_id(APP_ID).build();

    // Connect to "activate" signal of `app`
    app.connect_activate(build_ui);

    // Run the application
    app.run();
}

fn build_ui(app: &Application) {
    // Create a button with label and margins
    let container = gtk::Box::builder().build();

    let da = gtk::DrawingArea::builder()
        .content_height(200)
        .content_width(150)
        .build();

    da.set_draw_func(drawing);

    let desc = pango::FontDescription::from_string("Emoji 14");
    let attrs = pango::AttrList::new();
    attrs.insert(pango::AttrFontDesc::new(&desc));

    let label = gtk::Label::builder()
        .label("💩")
        .margin_top(10)
        .margin_bottom(10)
        .margin_start(10)
        .margin_end(10)
        .attributes(&attrs)
        .build();

    container.append(&da);
    container.append(&label);

    println!(
        "lang: {}",
        label.pango_context().language().unwrap().to_string()
    );

    // Create a window
    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("emoji demo")
        .child(&container)
        .build();

    // Present window
    window.present();
}

fn drawing(da: &gtk::DrawingArea, cr: &cairo::Context, _width: i32, _height: i32) {
    // cr.scale(0.5, 0.5);
    cr.move_to(50., 107.);
    let (x, y) = cr.target().fallback_resolution();
    println!("resolution {} {}", x, y);
    cr.target().set_device_scale(2., 2.);
    let (xscale, yscale) = cr.target().device_scale();
    println!("x-scale {} y-scale {}", xscale, yscale);
    cr.set_antialias(cairo::Antialias::None);
    let mut options = cairo::FontOptions::new().unwrap();
    options.set_antialias(cairo::Antialias::Gray);
    options.set_hint_style(cairo::HintStyle::None);
    options.set_hint_metrics(cairo::HintMetrics::Off);
    cr.set_font_options(&options);
    let text = "💩";
    let desc = pango::FontDescription::from_string("Emoji 14");
    let pctx = da.pango_context();
    pctx.set_font_description(&desc);
    // pangocairo::context_set_resolution(&pctx, 192.);
    println!("resolution {}", pangocairo::context_get_resolution(&pctx));
    let mut glyphs = pango::GlyphString::new();
    let attrs = pango::AttrList::new();
    attrs.insert(pango::AttrFontDesc::new(&desc));
    let items = pango::itemize(&pctx, text, 0, text.len() as _, &attrs, None);
    pango::shape(text, items[0].analysis(), &mut glyphs);

    let layout = pango::Layout::new(&pctx);
    // layout.set_font_description(Some(&desc));
    layout.set_text(text);
    let l = layout.line_readonly(0).unwrap();

    // pangocairo::show_glyph_item(cr, text, &mut l.runs()[0]);
    pangocairo::show_layout_line(cr, &l);

    // cr.set_scaled_font(
    //     &items[0]
    //         .analysis()
    //         .font()
    //         .downcast::<pangocairo::Font>()
    //         .unwrap()
    //         .scaled_font()
    //         .unwrap(),
    // );
    // let glyphs = vec![{
    //     let glyph = glyphs.glyph_info()[0];
    //     let geometry = glyph.geometry();
    //     cairo::Glyph::new(
    //         glyph.glyph() as _,
    //         (geometry.x_offset() + 50) as _,
    //         (geometry.y_offset() + 107) as _,
    //     )
    // }];
    // cr.show_glyphs(&glyphs).unwrap();
    // pangocairo::show_glyph_string(cr, &items[0].analysis().font(), &mut glyphs)
    // cr.show_text("💩").unwrap();
}
