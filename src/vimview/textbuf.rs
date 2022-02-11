use std::{
    cell::RefCell,
    mem::MaybeUninit,
    ops::{Deref, DerefMut},
};

use glib::subclass::prelude::*;

use crate::color::ColorExt;

use super::highlights::HighlightDefinitions;

mod imp {
    use std::borrow::Borrow;
    use std::cell::{Ref, RefCell};
    use std::ops::Deref;

    use glib::prelude::*;
    use glib::subclass::prelude::*;

    #[derive(Debug)]
    pub struct _TextBuf {
        pub(super) rows: usize,
        pub(super) cols: usize,
        pub(super) cells: super::TextBufCells,
    }

    impl Default for _TextBuf {
        fn default() -> Self {
            _TextBuf::new(0, 0)
        }
    }

    impl _TextBuf {
        fn new(rows: usize, cols: usize) -> _TextBuf {
            _TextBuf {
                rows,
                cols,
                cells: super::TextBufCells::new(rows, cols),
            }
        }

        fn resize(&mut self, rows: usize, cols: usize) {
            log::warn!(
                "_TextBuf resize to {}x{} from {}x{}",
                cols,
                rows,
                self.cols,
                self.rows
            );
            self.rows = rows;
            self.cols = cols;
            self.cells.resize(rows, cols);
        }

        fn clear(&mut self) {
            self.cells = super::TextBufCells::new(self.rows, self.cols);
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
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
        }
    }

    impl TextBuf {
        pub(super) fn set_cells(&self, row: usize, col: usize, cells: &[super::TextCell]) {
            log::info!(
                "textbuf {}x{}",
                self.inner.borrow().rows,
                self.inner.borrow().cols
            );
            log::warn!("textbuf setting cells of row {}", row);
            let row = &mut self.inner.borrow_mut().cells[row];
            let mut expands = Vec::with_capacity(row.len());
            for cell in cells.iter() {
                for _ in 0..cell.repeat.unwrap_or(1) {
                    expands.push(cell.clone());
                }
            }
            let col_to = col + expands.len();
            log::info!(
                "textbuf setting {} cells from {} to {}",
                expands.len(),
                col,
                col_to
            );
            // log::info!("cells: {:?}", &expands);
            row[col..col_to].clone_from_slice(&expands);
        }

        pub(super) fn clear(&self) {
            log::warn!("textbuf cleared");
            self.inner.borrow_mut().clear();
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
        glib::Object::new::<Self>(&[]).expect("Failed to initialize TextBuf object")
    }

    fn imp(&self) -> &imp::TextBuf {
        imp::TextBuf::from_instance(self)
    }

    pub fn clear(&self) {
        self.imp().clear();
    }

    pub fn resize(&self, rows: usize, cols: usize) {
        self.imp().resize(rows, cols);
    }

    pub fn rows(&self) -> usize {
        self.imp().rows()
    }

    pub fn cols(&self) -> usize {
        self.imp().cols()
    }

    pub fn cells(&self) -> &[TextLine] {
        self.imp().cells()
    }

    pub fn set_cells(&self, row: usize, col: usize, cells: &[TextCell]) {
        self.imp().set_cells(row, col, cells);
    }

    pub(super) fn layout(
        &self,
        pctx: &pango::Context,
        hldefs: &HighlightDefinitions,
    ) -> pango::Layout {
        const U16MAX: f32 = u16::MAX as f32 + 1.;
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
                let hldef = hldefs.get(cell.hldef.unwrap_or(1));
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
                let mut attr = pango::AttrInt::new_background_alpha(hldef.blend as _);
                attr.set_start_index(start_index as _);
                attr.set_end_index(end_index as _);
                attrs.insert(attr);
                if let Some(fg) = hldef.colors.foreground.or(default_colors.foreground) {
                    let mut attr = pango::AttrColor::new_foreground(
                        (fg.red() * U16MAX) as _,
                        (fg.green() * U16MAX) as _,
                        (fg.blue() * U16MAX) as _,
                    );
                    attr.set_start_index(start_index as _);
                    attr.set_end_index(end_index as _);
                    attrs.insert(attr);
                }
                if let Some(bg) = hldef.colors.background.or(default_colors.foreground) {
                    let mut attr = pango::AttrColor::new_background(
                        (bg.red() * U16MAX) as _,
                        (bg.green() * U16MAX) as _,
                        (bg.blue() * U16MAX) as _,
                    );
                    attr.set_start_index(start_index as _);
                    attr.set_end_index(end_index as _);
                    attrs.insert(attr);
                }
                if let Some(special) = hldef.colors.special.or(default_colors.special) {
                    let mut attr = pango::AttrColor::new_underline_color(
                        (special.red() * U16MAX) as _,
                        (special.green() * U16MAX) as _,
                        (special.blue() * U16MAX) as _,
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
        log::info!("pixel size {:?}", layout.pixel_size());
        log::info!("layout height: {}", layout.height());
        // let x801 = layout.index_to_line_x(801, false);
        // log::info!("layout index 801 to x {:?}", x801);
        // log::info!(
        //     "layout index 802 to x {:?}",
        //     layout.index_to_line_x(802, false)
        // );
        // let pos801 = layout.index_to_pos(801);
        // log::info!("layout index 801 to pos {:?}", pos801);
        // pango::extents_to_pixels(Some(&pos801), None);
        // log::info!("layout index 801 to pos {:?} in pixel", pos801);
        // let pos802 = layout.index_to_pos(802);
        // log::info!("layout index 802 to pos {:?}", pos802);
        // pango::extents_to_pixels(Some(&pos802), None);
        // log::info!("layout index 802 to pos {:?} in pixel", pos802);
        // let l = layout.line(1).unwrap();
        // log::info!(
        //     "line height: {} length {} start_index {}",
        //     l.height(),
        //     l.length(),
        //     l.start_index()
        // );
        // log::info!("extents: {:?}", l.extents());
        // log::info!("pixel extents: {:?}", l.pixel_extents());
        log::info!("text to render:\n{}", &text);
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

impl TextLine {
    fn new(cols: usize) -> TextLine {
        let mut line = Vec::with_capacity(1000);
        line.resize(cols, TextCell::default());
        // vec![TextCell::default(); cols].into_boxed_slice())
        Self(line.into_boxed_slice())
    }
}

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
    cells: RefCell<Box<[TextLine]>>,
}

impl Deref for TextBufCells {
    type Target = [TextLine];

    fn deref(&self) -> &[TextLine] {
        unsafe { &*self.cells.as_ptr() }
    }
}

impl DerefMut for TextBufCells {
    fn deref_mut(&mut self) -> &mut [TextLine] {
        self.cells.get_mut()
    }
}

impl AsRef<[TextLine]> for TextBufCells {
    fn as_ref(&self) -> &[TextLine] {
        unsafe { &*self.cells.as_ptr() }
    }
}

impl AsMut<[TextLine]> for TextBufCells {
    fn as_mut(&mut self) -> &mut [TextLine] {
        self.cells.get_mut()
    }
}

impl TextBufCells {
    pub fn new(rows: usize, cols: usize) -> TextBufCells {
        TextBufCells {
            cells: RefCell::new(cells(rows, cols)),
        }
    }

    pub fn resize(&mut self, rows: usize, cols: usize) {
        assert!(rows < 1000);
        assert!(cols < 1000);
        log::info!("1 buf cells resizing");
        log::info!("2 buf cells resizing");
        log::info!("3 buf cells resizing");
        log::info!("4 buf cells resizing");
        let cells = self.cells.take().into_vec();
        let old_rows = cells.len();
        let nrows = rows.min(old_rows);
        log::info!("5 buf cells resizing {} {}", cells.len(), nrows);
        let cells: Vec<_> = cells
            .into_iter()
            .take(nrows)
            .map(|line| {
                // log::info!("x1 buf cells resizing {}, {:?}", line.0.len(), line.0);
                let mut textline = line.0.into_vec();
                // let mut textline =
                //     unsafe { Vec::from_raw_parts(line.0.as_mut_ptr(), line.0.len(), 1000) };
                // log::info!("x2 buf cells resizing {}, {:?}", line.0.len(), line.0);
                textline.resize(cols, TextCell::default());
                // log::info!("x3 buf cells resizing {}, {:?}", line.0.len(), line.0);
                TextLine(textline.into_boxed_slice())
            })
            .chain(vec![TextLine::new(cols); rows.saturating_sub(old_rows)].into_iter())
            .collect();

        log::info!("6 buf cells resizing");
        // cells.append(&mut vec![
        //     TextLine::new(cols);
        //     rows.saturating_sub(old_rows)
        // ]);
        log::info!("buf cells resizing to {} rows from {}", rows, old_rows);
        // let cells_ = unsafe { Vec::from_raw_parts(cells.as_mut_ptr(), old_rows, 1000) };
        // let mut _cells = Box::new_uninit_slice(rows);
        //log::info!("buf cells resizing cloning head {} rows", nrows);
        //if nrows > 0 {
        //    MaybeUninit::write_slice_cloned(&mut _cells[0..nrows], &cells[0..nrows]);
        //}
        //if nrows < rows {
        //    for nrow in nrows..rows {
        //        log::info!("buf cells resizing writing row {}", nrow);
        //        _cells[nrow].write({
        //            let mut default_line = Vec::with_capacity(1000);
        //            default_line.resize(cols, TextCell::default());
        //            TextLine(default_line.into_boxed_slice())
        //        });
        //    }
        //}
        // cells.resize_with(rows, || {
        //     let mut default_line = Vec::with_capacity(1000);
        //     default_line.resize(cols, TextCell::default());
        //     TextLine(default_line.into_boxed_slice())
        // });
        log::info!("buf cells resized to {}x{}", cols, rows);
        // self.cells = cells.into_boxed_slice();
        log::info!("buf cells resized to {}x{}", cols, rows);
        // self.cells.replace(unsafe { _cells.assume_init() });
        self.cells.replace(cells.into_boxed_slice());
    }
}

fn cells(rows: usize, cols: usize) -> Box<[TextLine]> {
    log::error!("creating cells {}x{}", rows, cols);
    assert!(rows < 1000);
    assert!(cols < 1000);
    let mut row = Vec::with_capacity(1000);
    row.resize(cols, TextCell::default());
    // let row = vec![TextCell::default(); cols].into_boxed_slice();
    let tl = TextLine(row.into_boxed_slice());
    let mut cells = Vec::with_capacity(1000);
    cells.resize(rows, tl);
    cells.into_boxed_slice()
}
