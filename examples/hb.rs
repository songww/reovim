fn main() {
    let mut buf = harfbuzz::Buffer::new();
    buf.add_str("▎     [No Name]", 0, Some(7));
    buf.guess_segment_properties();
    println!("len: {}", buf.len());
    // harfbuzz::shape();
    let glyph_infos = buf.glyph_infos();
    let glyph_positions = buf.glyph_positions();
    println!("glyph infos {:?}", &glyph_infos);
    println!("glyph positions: {:?}", &glyph_positions);
}
