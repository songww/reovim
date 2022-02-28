use crate::color::Color;
use crate::style::{Colors, Style};
use crate::vimview::{HighlightDefinitions, TextCell};

#[derive(Debug, Clone, PartialEq)]
pub enum CursorShape {
    Block,
    Horizontal,
    Vertical,
}

impl CursorShape {
    pub fn from_type_name(name: &str) -> Option<CursorShape> {
        match name {
            "block" => Some(CursorShape::Block),
            "horizontal" => Some(CursorShape::Horizontal),
            "vertical" => Some(CursorShape::Vertical),
            _ => None,
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq)]
pub struct CursorMode {
    pub shape: Option<CursorShape>,
    pub style: Option<u64>,
    pub cell_percentage: Option<f64>,
    pub blinkwait: Option<u64>,
    pub blinkon: Option<u64>,
    pub blinkoff: Option<u64>,
}

#[derive(Clone, Debug)]
pub struct Cursor {
    pub pos: (f64, f64),
    pub grid: u64,
    pub shape: CursorShape,
    pub cell_percentage: Option<f64>,
    pub blinkwait: Option<u64>,
    pub blinkon: Option<u64>,
    pub blinkoff: Option<u64>,
    pub style: Option<Style>,
    pub enabled: bool,
    pub width: f64,
    pub cell: TextCell,
}

impl Cursor {
    pub fn new() -> Cursor {
        Cursor {
            pos: (0., 0.).into(),
            grid: 0,
            shape: CursorShape::Block,
            style: None,
            cell_percentage: None,
            blinkwait: None,
            blinkon: None,
            blinkoff: None,
            enabled: true,
            width: 1.,
            cell: TextCell::default(),
        }
    }

    pub fn rectangle(&self, width: f64, height: f64) -> (f64, f64, f64, f64) {
        let percentage = self.cell_percentage.unwrap_or(1.);
        log::debug!(
            "cursor percentage {:?} {}",
            self.cell_percentage,
            percentage
        );
        match self.shape {
            CursorShape::Block => (self.pos.0, self.pos.1, width, height),
            CursorShape::Vertical => (self.pos.0, self.pos.1, width * percentage, height),
            CursorShape::Horizontal => (
                self.pos.0,
                self.pos.1 + height - height * percentage,
                width,
                height * percentage,
            ),
        }
    }

    pub fn foreground(&self, default_colors: &Colors) -> Color {
        if let Some(style) = &self.style {
            style
                .colors
                .foreground
                .unwrap_or_else(|| default_colors.background.unwrap())
        } else {
            default_colors.background.unwrap()
        }
    }

    pub fn background(&self, default_colors: &Colors) -> Color {
        if let Some(style) = &self.style {
            let alpha = (100 - style.blend) as f32 / 100.;
            let mut color = style
                .colors
                .background
                .unwrap_or_else(|| default_colors.foreground.unwrap());
            color.set_alpha(alpha);
            color
        } else {
            default_colors.foreground.unwrap()
        }
    }

    pub fn cell(&self) -> &TextCell {
        &self.cell
    }

    pub fn pos(&self) -> (f64, f64) {
        self.pos
    }

    pub fn set_pos(&mut self, x: f64, y: f64) {
        self.pos = (x, y);
    }

    pub fn set_cell(&mut self, cell: TextCell) {
        // let character = cell.text.chars().next().unwrap();
        // let width = unsafe {
        //     if from_glib(g_unichar_iswide(character as u32))
        //         || from_glib(g_unichar_iswide_cjk(character as u32))
        //     {
        //         // if from_glib(g_unichar_iswide_cjk(character as u32)) {
        //         2.
        //     } else if from_glib(g_unichar_iszerowidth(character as u32)) {
        //         0.
        //     } else {
        //         1.
        //     }
        // };
        let width = if cell.double_width {
            2.
        } else if cell.text.is_empty() {
            0.
        } else {
            1.
        };
        self.cell = cell;
        self.width = width;
    }

    pub fn change_mode(&mut self, cursor_mode: &CursorMode, styles: &HighlightDefinitions) {
        let CursorMode {
            shape,
            style,
            cell_percentage,
            blinkwait,
            blinkon,
            blinkoff,
        } = cursor_mode;

        if let Some(shape) = shape {
            self.shape = shape.clone();
        }

        if let Some(style) = style {
            self.style = styles.get(*style).cloned();
        }

        self.cell_percentage = *cell_percentage;
        self.blinkwait = *blinkwait;
        self.blinkon = *blinkon;
        self.blinkoff = *blinkoff;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Color;
    use rustc_hash::FxHashMap;
    use std::sync::Arc;

    const COLORS: Colors = Colors {
        foreground: Some(Color::new(0.1, 0.1, 0.1, 0.1)),
        background: Some(Color::new(0.2, 0.1, 0.1, 0.1)),
        special: Some(Color::new(0.3, 0.1, 0.1, 0.1)),
    };

    const DEFAULT_COLORS: Colors = Colors {
        foreground: Some(Color::new(0.1, 0.2, 0.1, 0.1)),
        background: Some(Color::new(0.2, 0.2, 0.1, 0.1)),
        special: Some(Color::new(0.3, 0.2, 0.1, 0.1)),
    };

    const NONE_COLORS: Colors = Colors {
        foreground: None,
        background: None,
        special: None,
    };

    #[test]
    fn test_from_type_name() {
        assert_eq!(
            CursorShape::from_type_name("block"),
            Some(CursorShape::Block)
        );
        assert_eq!(
            CursorShape::from_type_name("horizontal"),
            Some(CursorShape::Horizontal)
        );
        assert_eq!(
            CursorShape::from_type_name("vertical"),
            Some(CursorShape::Vertical)
        );
    }

    #[test]
    fn test_foreground() {
        let mut cursor = Cursor::new();
        let style = Some(Arc::new(Style::new(COLORS)));

        assert_eq!(
            cursor.foreground(&DEFAULT_COLORS),
            DEFAULT_COLORS.background.unwrap()
        );
        cursor.style = style.clone();
        assert_eq!(
            cursor.foreground(&DEFAULT_COLORS),
            COLORS.foreground.unwrap()
        );

        cursor.style = Some(Arc::new(Style::new(NONE_COLORS)));
        assert_eq!(
            cursor.foreground(&DEFAULT_COLORS),
            DEFAULT_COLORS.background.unwrap()
        );
    }

    #[test]
    fn test_background() {
        let mut cursor = Cursor::new();
        let style = Some(Arc::new(Style::new(COLORS)));

        assert_eq!(
            cursor.background(&DEFAULT_COLORS),
            DEFAULT_COLORS.foreground.unwrap()
        );
        cursor.style = style.clone();
        assert_eq!(
            cursor.background(&DEFAULT_COLORS),
            COLORS.background.unwrap()
        );

        cursor.style = Some(Arc::new(Style::new(NONE_COLORS)));
        assert_eq!(
            cursor.background(&DEFAULT_COLORS),
            DEFAULT_COLORS.foreground.unwrap()
        );
    }

    #[test]
    fn test_change_mode() {
        let cursor_mode = CursorMode {
            shape: Some(CursorShape::Horizontal),
            style: Some(1),
            cell_percentage: Some(100.0),
            blinkwait: Some(1),
            blinkon: Some(1),
            blinkoff: Some(1),
        };
        let mut styles = FxHashMap::default();
        styles.insert(1, Arc::new(Style::new(COLORS)));

        let mut cursor = Cursor::new();

        cursor.change_mode(&cursor_mode, &styles);
        assert_eq!(cursor.shape, CursorShape::Horizontal);
        assert_eq!(cursor.style, styles.get(&1).cloned());
        assert_eq!(cursor.cell_percentage, Some(100.0));
        assert_eq!(cursor.blinkwait, Some(1));
        assert_eq!(cursor.blinkon, Some(1));
        assert_eq!(cursor.blinkoff, Some(1));

        let cursor_mode_with_none = CursorMode {
            shape: None,
            style: None,
            cell_percentage: None,
            blinkwait: None,
            blinkon: None,
            blinkoff: None,
        };
        cursor.change_mode(&cursor_mode_with_none, &styles);
        assert_eq!(cursor.shape, CursorShape::Horizontal);
        assert_eq!(cursor.style, styles.get(&1).cloned());
        assert_eq!(cursor.cell_percentage, None);
        assert_eq!(cursor.blinkwait, None);
        assert_eq!(cursor.blinkon, None);
        assert_eq!(cursor.blinkoff, None);
    }
}
