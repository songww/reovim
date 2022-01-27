use std::ops::{Deref, DerefMut};

use glib::subclass::prelude::*;

use super::highlights::HighlightDefinitions;

mod imp {
    use std::borrow::Borrow;
    use std::cell::{Ref, RefCell};
    use std::ops::Deref;

    use glib::prelude::*;
    use glib::subclass::prelude::*;

    #[derive(Debug, Default)]
    pub struct _TextBuf {
        pub(super) rows: usize,
        pub(super) cols: usize,
        pub(super) cells: super::TextBufCells,
    }

    impl _TextBuf {
        fn resize(&mut self, rows: usize, cols: usize) {
            self.rows = rows;
            self.cols = cols;
            self.cells = super::TextBufCells::new(rows, cols);
        }
    }

    #[derive(Debug, Default)]
    pub struct TextBuf {
        inner: RefCell<_TextBuf>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TextBuf {
        const NAME: &'static str = "TextBuf";
        type Type = super::TextBuf;
        type ParentType = glib::Object;
    }

    impl ObjectImpl for TextBuf {
        // fn signals() -> &'static [Signal] {
        //     static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
        //         vec![
        //             Signal::builder(
        //                 "countdown-update",
        //                 &[u32::static_type().into(), u32::static_type().into()],
        //                 <()>::static_type().into(),
        //             )
        //             .build(),
        //             Signal::builder("lap", &[], <()>::static_type().into()).build(),
        //         ]
        //     });
        //     SIGNALS.as_ref()
        // }
    }

    impl TextBuf {
        pub(super) fn set_cells(&self, row: usize, col: usize, cells: &[super::TextCell]) {
            let row = &mut self.inner.borrow_mut().cells[row];
            let mut expands = Vec::with_capacity(row.len());
            for cell in cells.iter() {
                for _ in 0..cell.repeat.unwrap_or(1) {
                    expands.push(cell.clone());
                }
            }
            let col_to = col + expands.len();
            row[col..col_to].clone_from_slice(&expands);
        }

        pub(super) fn clear(&self) {
            let inner = self.inner.borrow();
            let rows = inner.rows;
            let cols = inner.cols;
            self.inner.borrow_mut().cells = super::TextBufCells::new(rows, cols);
        }

        pub(super) fn resize(&self, rows: usize, cols: usize) {
            self.inner.borrow_mut().resize(rows, cols);
        }

        pub(super) fn cells(&self) -> &[super::TextLine] {
            &unsafe { &*self.inner.as_ptr() }.cells
        }

        pub(super) fn rows(&self) -> usize {
            self.inner.borrow().rows
        }

        pub(super) fn cols(&self) -> usize {
            self.inner.borrow().cols
        }
    }
}

glib::wrapper! {
    pub struct TextBuf(ObjectSubclass<imp::TextBuf>);
}

impl TextBuf {
    pub fn new() -> Self {
        glib::Object::new::<Self>(&[]).expect("Failed to initialize Timer object")
    }

    fn imp(&self) -> &imp::TextBuf {
        imp::TextBuf::from_instance(self)
    }

    fn clear(&self) {
        self.imp().clear();
    }

    pub(super) fn layout(
        &self,
        pctx: &pango::Context,
        hldefs: &HighlightDefinitions,
    ) -> pango::Layout {
        let imp = self.imp();
        let cells = imp.cells();
        let layout = pango::Layout::new(pctx);
        if imp.cols() == 0 || imp.rows() == 0 {
            return layout;
        }
        let attrs = pango::AttrList::new();
        let mut text = String::with_capacity(imp.cols() * imp.rows() + imp.rows());
        let default_colors = hldefs.get(0).colors;
        for line_cells in cells.iter() {
            // let mut linetext = String::with_capacity(imp.cols + 1);
            for cell in line_cells.iter() {
                let start_index = text.len();
                text.push_str(&cell.text);
                let end_index = text.len();
                let hldef = hldefs.get(cell.hldef.unwrap());
                if hldef.italic {
                    let mut attr = pango::AttrInt::new_style(pango::Style::Italic);
                    attr.set_start_index(start_index as _);
                    attr.set_end_index(end_index as _);
                    attrs.insert(attr);
                }
                if hldef.bold {
                    let mut attr = pango::AttrInt::new_weight(pango::Weight::Bold);
                    attr.set_start_index(start_index as _);
                    attr.set_end_index(end_index as _);
                    attrs.insert(attr);
                }
                if hldef.strikethrough {
                    let mut attr = pango::AttrInt::new_strikethrough(true);
                    attr.set_start_index(start_index as _);
                    attr.set_end_index(end_index as _);
                    attrs.insert(attr);
                }
                if hldef.underline {
                    let mut attr = pango::AttrInt::new_underline(pango::Underline::SingleLine);
                    attr.set_start_index(start_index as _);
                    attr.set_end_index(end_index as _);
                    attrs.insert(attr);
                }
                if hldef.undercurl {
                    let mut attr = pango::AttrInt::new_underline(pango::Underline::ErrorLine);
                    attr.set_start_index(start_index as _);
                    attr.set_end_index(end_index as _);
                    attrs.insert(attr);
                }
                // alpha color
                log::error!("alpha blend {}", hldef.blend);
                let mut attr = pango::AttrInt::new_background_alpha(hldef.blend as _);
                attr.set_start_index(start_index as _);
                attr.set_end_index(end_index as _);
                attrs.insert(attr);
                if let Some(fg) = hldef
                    .colors
                    .foreground
                    .or_else(|| default_colors.foreground)
                {
                    let color = pango::Color::parse(&fg.to_str()).unwrap();
                    let mut attr =
                        pango::AttrColor::new_foreground(color.red(), color.green(), color.blue());
                    attr.set_start_index(start_index as _);
                    attr.set_end_index(end_index as _);
                    attrs.insert(attr);
                }
                if let Some(bg) = hldef
                    .colors
                    .background
                    .or_else(|| default_colors.foreground)
                {
                    let color = pango::Color::parse(&bg.to_str()).unwrap();
                    let mut attr =
                        pango::AttrColor::new_background(color.red(), color.green(), color.blue());
                    attr.set_start_index(start_index as _);
                    attr.set_end_index(end_index as _);
                    attrs.insert(attr);
                }
                if let Some(special) = hldef.colors.special.or_else(|| default_colors.special) {
                    let color = pango::Color::parse(&special.to_str()).unwrap();
                    let mut attr = pango::AttrColor::new_underline_color(
                        color.red(),
                        color.green(),
                        color.blue(),
                    );
                    attr.set_start_index(start_index as _);
                    attr.set_end_index(end_index as _);
                    attrs.insert(attr);
                }
            }
            text.push('\n');
        }
        layout.set_text(&text);
        layout.set_attributes(Some(&attrs));
        // let mut cached_attr = attrs.iterator();
        // let items = pango::itemize(pctx, cell.text, 0, text.len(), &attrs, cached_attr.as_ref());
        // for item in items {
        //     // item.analysis();
        //     item
        // }
        layout
    }
}

#[derive(Clone, Debug)]
pub struct TextCell {
    pub text: String,
    pub hldef: Option<u64>,
    pub repeat: Option<u64>,
    pub double_width: bool,
}

impl Default for TextCell {
    fn default() -> TextCell {
        TextCell {
            text: String::from(" "),
            hldef: None,
            repeat: None,
            double_width: false,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct TextLine(Box<[TextCell]>);

impl Deref for TextLine {
    type Target = [TextCell];

    fn deref(&self) -> &[TextCell] {
        &self.0
    }
}

impl DerefMut for TextLine {
    fn deref_mut(&mut self) -> &mut [TextCell] {
        &mut self.0
    }
}

impl AsRef<[TextCell]> for TextLine {
    fn as_ref(&self) -> &[TextCell] {
        &self.0
    }
}

impl AsMut<[TextCell]> for TextLine {
    fn as_mut(&mut self) -> &mut [TextCell] {
        &mut self.0
    }
}

#[derive(Debug, Default)]
struct TextBufCells {
    cells: Box<[TextLine]>,
}

impl Deref for TextBufCells {
    type Target = [TextLine];

    fn deref(&self) -> &[TextLine] {
        &self.cells
    }
}

impl DerefMut for TextBufCells {
    fn deref_mut(&mut self) -> &mut [TextLine] {
        &mut self.cells
    }
}

impl AsRef<[TextLine]> for TextBufCells {
    fn as_ref(&self) -> &[TextLine] {
        &self.cells
    }
}

impl AsMut<[TextLine]> for TextBufCells {
    fn as_mut(&mut self) -> &mut [TextLine] {
        &mut self.cells
    }
}

impl TextBufCells {
    pub fn new(rows: usize, cols: usize) -> TextBufCells {
        TextBufCells {
            cells: cells(rows, cols),
        }
    }
}

fn cells(rows: usize, cols: usize) -> Box<[TextLine]> {
    let row = vec![TextCell::default(); rows].into_boxed_slice();
    vec![TextLine(row); cols].into_boxed_slice()
}
