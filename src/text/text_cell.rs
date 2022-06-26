// use crate::style::Style;
// use crate::vimview::HighlightDefinitions;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextCell {
    pub text: String,
    pub hldef: Option<u64>,
    pub width: u8,
    // pub style: Style,
    pub start_index: usize,
    pub end_index: usize,
}

impl Default for TextCell {
    fn default() -> TextCell {
        TextCell {
            text: String::from(" "),
            hldef: None,
            width: 1,
            // style: Style::default(),
            start_index: 0,
            end_index: 0,
        }
    }
}

impl TextCell {
    /*
    pub fn reset_attrs(
        &mut self,
        _pctx: &pango::Context,
        hldefs: &HighlightDefinitions,
        _metrics: &crate::metrics::Metrics,
    ) {
        const U16MAX: f32 = u16::MAX as f32;

        self.attrs.clear();
        let attrs = pango::AttrList::new();

        if self.end_index == self.start_index {
            return;
        }

        let start_index = self.start_index as u32;
        let end_index = self.end_index as u32;
        let default_hldef = hldefs.get(HighlightDefinitions::DEFAULT).unwrap();
        let default_colors = hldefs.defaults().unwrap();
        let mut background = None;
        let mut hldef = default_hldef;
        if let Some(ref id) = self.hldef {
            let style = hldefs.get(*id);
            if let Some(style) = style {
                background = style.background();
                hldef = style;
            }
        }
        if hldef.italic {
            let mut attr = pango::AttrInt::new_style(pango::Style::Italic);
            attr.set_start_index(start_index);
            attr.set_end_index(end_index);
            attrs.insert(attr);
        }
        if hldef.bold {
            let mut attr = pango::AttrInt::new_weight(pango::Weight::Semibold);
            attr.set_start_index(start_index);
            attr.set_end_index(end_index);
            attrs.insert(attr);
        }
        if hldef.strikethrough {
            let mut attr = pango::AttrInt::new_strikethrough(true);
            attr.set_start_index(start_index);
            attr.set_end_index(end_index);
            attrs.insert(attr);
        }
        if hldef.underline {
            let mut attr = pango::AttrInt::new_underline(pango::Underline::Single);
            attr.set_start_index(start_index);
            attr.set_end_index(end_index);
            attrs.insert(attr);
        }
        if hldef.undercurl {
            let mut attr = pango::AttrInt::new_underline(pango::Underline::Error);
            attr.set_start_index(start_index);
            attr.set_end_index(end_index);
            attrs.insert(attr);
        }
        // alpha color
        // blend is 0 - 100. Could be used by UIs to support
        // blending floating windows to the background or to
        // signal a transparent cursor.
        // let blend = u16::MAX as u32 * hldef.blend as u32 / 100;
        // let mut attr = pango::AttrInt::new_background_alpha(blend as u16);
        // log::info!("blend {}", hldef.blend);
        // attr.set_start_index(start_index as _);
        // attr.set_end_index(end_index as _);
        // attrs.insert(attr);
        if let Some(fg) = hldef.colors.foreground.or(default_colors.foreground) {
            let mut attr = pango::AttrColor::new_foreground(
                (fg.red() * U16MAX).round() as u16,
                (fg.green() * U16MAX).round() as u16,
                (fg.blue() * U16MAX).round() as u16,
            );
            attr.set_start_index(start_index);
            attr.set_end_index(end_index);
            attrs.insert(attr);
        }
        if let Some(bg) = background {
            let mut attr = pango::AttrColor::new_background(
                (bg.red() * U16MAX).round() as u16,
                (bg.green() * U16MAX).round() as u16,
                (bg.blue() * U16MAX).round() as u16,
            );
            attr.set_start_index(start_index);
            attr.set_end_index(end_index);
            attrs.insert(attr);
        }
        if let Some(special) = hldef.colors.special.or(default_colors.special) {
            let mut attr = pango::AttrColor::new_underline_color(
                (special.red() * U16MAX).round() as u16,
                (special.green() * U16MAX).round() as u16,
                (special.blue() * U16MAX).round() as u16,
            );
            attr.set_start_index(start_index);
            attr.set_end_index(end_index);
            attrs.insert(attr);
        }

        self.attrs = attrs.attributes();
    }
    */
}
