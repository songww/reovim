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
            "{:02x}{:02x}{:02x}",
            (self.red() * 255.0) as u8,
            (self.green() * 255.0) as u8,
            (self.blue() * 255.0) as u8
        )
    }
}

/*
impl Color {
    pub fn new(red: f64, green: f64, blue: f64, alpha: f64) -> Self {
        Color(gdk::RGBA::new(
            red as f32,
            green as f32,
            blue as f32,
            alpha as f32,
        ))
    }

    pub fn red(&self) -> f64 {
        self.0.red() as _
    }

    pub fn green(&self) -> f64 {
        self.0.green() as _
    }

    pub fn blue(&self) -> f64 {
        self.0.blue() as _
    }

    pub fn alpha(&self) -> f64 {
        self.0.alpha() as _
    }

    pub fn from_hex_string(hex: &str) -> Result<Self, String> {
        let l = hex.chars().count();
        let hex: String = match l {
            7 => hex.chars().skip(1).collect(),
            6 => hex.to_string(),
            _ => {
                return Err(String::from("hex string has invalid length"));
            }
        };

        let res = u64::from_str_radix(hex.as_str(), 16);

        if let Ok(res) = res {
            Ok(Self::from_u64(res))
        } else {
            Err(format!(
                "Failed to parse hex string '{}': {:?}",
                hex,
                res.err()
            ))
        }
    }

    pub fn from_u64(v: u64) -> Self {
        Self(gdk::RGBA::new(
            ((v >> 16) & 255) as f32 / 255f32,
            ((v >> 8) & 255) as f32 / 255f32,
            (v & 255) as f32 / 255f32,
            1.0,
        ))
    }

    pub fn to_hex(&self) -> String {
        format!(
            "{:02x}{:02x}{:02x}",
            (self.red() * 255.0) as u8,
            (self.green() * 255.0) as u8,
            (self.blue() * 255.0) as u8
        )
    }

    /// Apply the blend value to color. Returns the color in `rgba()` format.
    /// Note that the blend value is inverted.
    pub fn to_rgba(&self, blend: f64) -> String {
        format!(
            "rgba({}, {}, {}, {})",
            (self.red() * 255.0) as u8,
            (self.green() * 255.0) as u8,
            (self.blue() * 255.0) as u8,
            1.0 - blend
        )
    }
}

impl Deref for Color {
    type Target = gdk::RGBA;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
*/
