// cellwidth: f64, charwidth: f64, charheight: f64
#[derive(Clone, Copy, Debug, Default)]
pub struct Metrics {
    /// by pango glyph string extens -> logical width.
    charwidth: f64,
    /// by pango font metrics, dose not include line spacing.
    charheight: f64,
    // charwidth: f64,
    linespace: f64,
    /// charheight + linespace
    height: f64,
    /// by pango font metrics
    width: f64,
}

impl Metrics {
    pub fn new() -> Metrics {
        Metrics {
            // prevent zero window size.
            charwidth: 1.,
            charheight: 2.,

            linespace: 0.,

            width: 1.,
            height: 2.,
        }
    }

    /// 实际渲染到屏幕上的字符像素宽度
    pub fn charwidth(&self) -> f64 {
        self.charwidth
    }

    pub fn set_charwidth(&mut self, charwidth: f64) {
        self.charwidth = charwidth;
    }

    /// without linespace
    pub fn charheight(&self) -> f64 {
        self.charheight
    }

    pub fn set_charheight(&mut self, charheight: f64) {
        self.charheight = charheight;
        self.height = charheight + self.linespace;
    }

    /// charheight + linespace
    pub fn height(&self) -> f64 {
        self.height
    }

    /// 每个cell的像素宽度
    pub fn width(&self) -> f64 {
        self.width
    }

    pub fn set_width(&mut self, width: f64) {
        self.width = width;
    }

    pub fn linespace(&self) -> f64 {
        self.linespace
    }

    pub fn set_linespace(&mut self, linespace: f64) {
        self.linespace = linespace;
    }
}
