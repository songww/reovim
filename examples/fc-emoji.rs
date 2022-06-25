use fontconfig::{properties, FontConfig, MatchKind, OwnedPattern, Pattern};
use std::io::Write;

fn main() {
    let mut config = FontConfig::default();

    let mut pattern = OwnedPattern::new();
    pattern.add_family("emoji".to_string());
    pattern.add_size(32.);

    config.substitute(&mut pattern, MatchKind::Pattern);
    pattern.default_substitute();

    let mut font = pattern.font_match(&mut config);

    let font = pattern.render_prepare(&mut config, &mut font);
    let ptem = font.get(&properties::FC_SIZE, 0).unwrap();
    println!("{} {}", font.get(&properties::FC_FAMILY, 0).unwrap(), ptem);
    let matrix = font.matrix().unwrap();
    println!("{:?}", matrix);
    let pixel_size = font.get(&properties::FC_PIXEL_SIZE, 0).unwrap();
    println!("pixel size {}", pixel_size);
    println!("dpi {}", font.get(&properties::FC_DPI, 0).unwrap());
    println!("width {}", font.get(&properties::FC_WIDTH, 0).unwrap());

    let path = font.get(&properties::FC_FILE, 0).unwrap();
    let index = font.get(&properties::FC_INDEX, 0).unwrap();

    let ftlib = freetype::Library::init().unwrap();
    let ftface = ftlib.new_face(path, index as isize).unwrap();
    let mut ftmatrix = freetype::Matrix {
        xx: (matrix.xx() * 64.) as i64,
        xy: (matrix.xy() * 64.) as i64,
        yx: (matrix.yx() * 64.) as i64,
        yy: (matrix.yy() * 64.) as i64,
    };
    ftface.set_transform(&mut ftmatrix, &mut freetype::Vector::default());
    ftface
        .set_pixel_sizes(0, pixel_size.round() as u32)
        .unwrap();

    // let hbfont = unsafe { harfbuzz::sys::hb_ft_font_create_referenced(ftface.raw_mut()) };
    //
    // let hb = unsafe { harfbuzz::Font::from_raw(hbfont) };
    //
    // println!("ppem: {:?}", hb.ppem());
    // println!("ptem: {}", hb.ptem());
    // println!("scale: {:?}", hb.scale());
    // let filename = font.get(&properties::FC_FILE, 0).unwrap();
    // let index = font.get(&properties::FC_INDEX, 0).unwrap();
    let antialias = font.get(&properties::FC_ANTIALIAS, 0).unwrap();
    println!("antialias: {}", antialias);
    let hintstyle = font.get(&properties::FC_HINT_STYLE, 0).unwrap();
    println!("hintstyle: {}", hintstyle);

    let hinting = font.get(&properties::FC_HINT_STYLE, 0).unwrap();
    println!("hinting: {}", hinting);
    let autohint = font.get(&properties::FC_AUTOHINT, 0).unwrap();
    println!("autohint: {}", autohint);

    // let data = std::fs::read(filename).unwrap();
    // let blob = harfbuzz::Blob::new_read_only(&data);
    //
    // let mut face = harfbuzz::Face::new(&blob, index as _);
    //
    // let mut hb_font = harfbuzz::Font::new(&mut face);
    // hb_font.set_ptem(ptem as f32);
    // hb_font.set_scale(matrix.xx() as i32, matrix.yy() as i32);

    let surf = cairo::ImageSurface::create(cairo::Format::ARgb32, 100, 100).unwrap();
    let cr = cairo::Context::new(&surf).unwrap();
    cr.set_source_rgb(1., 1., 1.);
    cr.paint().unwrap();
    cr.set_source_rgb(0., 0., 0.);

    let face =
        cairo::FontFace::create_from_ft_with_flags(&ftface, freetype::face::LoadFlag::COLOR.bits())
            .unwrap();
    let matrix = cairo::Matrix::new(256., 0., 0., 256., 0., 0.);
    // let matrix = cairo::Matrix::identity();
    let ctm = cairo::Matrix::identity();
    println!("{:?}", ctm);
    println!("{:?}", matrix);
    // cr.set_font_face(&face);
    let mut options = cairo::FontOptions::new().unwrap();
    options.set_antialias(cairo::Antialias::None);
    options.set_hint_style(cairo::HintStyle::Slight);
    options.set_hint_metrics(cairo::HintMetrics::On);
    options.set_subpixel_order(cairo::SubpixelOrder::Vbgr);

    let scaled_font = cairo::ScaledFont::new(&face, &matrix, &ctm, &options).unwrap();
    cr.set_scaled_font(&scaled_font);

    // cr.set_font_options(&options);
    cr.move_to(30., 30.);
    cr.show_text("💩").unwrap();

    let mut f = std::fs::File::options()
        .write(true)
        .create(true)
        .open("emoji.png")
        .unwrap();
    surf.write_to_png(&mut f).unwrap();

    f.flush().unwrap();
}
