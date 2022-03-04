use gtk::gdk;

// #[derive(Copy, Clone, Debug, PartialEq)]
pub type Color = gdk::RGBA;

pub trait ColorExt {
    fn from_u64(u: u64) -> Self;

    fn to_hex(&self) -> String;
}

impl ColorExt for Color {
    fn from_u64(v: u64) -> Self {
        gdk::RGBA::new(
            ((v >> 16) & 255) as f32 / 255f32,
            ((v >> 8) & 255) as f32 / 255f32,
            (v & 255) as f32 / 255f32,
            1.0,
        )
    }
    fn to_hex(&self) -> String {
        format!(
            "#{:02x}{:02x}{:02x}",
            (self.red() * 255.0) as u8,
            (self.green() * 255.0) as u8,
            (self.blue() * 255.0) as u8
        )
    }
}

#[derive(new, Copy, Clone, Debug, Default, PartialEq)]
pub struct Colors {
    pub foreground: Option<Color>,
    pub background: Option<Color>,
    pub special: Option<Color>,
}
