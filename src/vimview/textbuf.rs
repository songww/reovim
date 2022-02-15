use std::cell::RefCell;
use std::ops::{Deref, DerefMut};

use gdk::prelude::FontExt;
use glib::subclass::prelude::*;
use rustc_hash::FxHashMap;
use vector_map::VecMap;

use crate::color::ColorExt;

use super::highlights::HighlightDefinitions;

mod imp {
    use std::marker::PhantomData;
    use std::sync::{RwLock, RwLockReadGuard};

    use glib::prelude::*;
    use glib::subclass::prelude::*;

    #[derive(Debug)]
    pub struct _TextBuf {
        pub(super) rows: usize,
        pub(super) cols: usize,
        pub(super) cells: Box<[super::TextLine]>,
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
                cells: _TextBuf::make(rows, cols),
            }
        }

        fn clear(&mut self) {
            self.cells = _TextBuf::make(self.rows, self.cols);
        }

        fn set_cells(&mut self, row: usize, col: usize, cells: &[super::TextCell]) {
            let nrows = self.rows;
            let ncols = self.cols;
            let row = &mut self.cells[row];
            let mut expands = Vec::with_capacity(row.len());
            for cell in cells.iter() {
                for _ in 0..cell.repeat.unwrap_or(1) {
                    expands.push(cell.clone());
                }
            }
            let col_to = col + expands.len();
            log::info!(
                "textbuf {}x{} setting {} cells from {} to {}",
                ncols,
                nrows,
                expands.len(),
                col,
                col_to
            );
            // log::info!("cells: {:?}", &expands);
            row[col..col_to].swap_with_slice(&mut expands); //
        }

        /// drop head of {} rows. leave tail as empty.
        fn up(&mut self, rows: usize) {
            let mut cells = _TextBuf::make(self.rows, self.cols);
            cells[..(self.rows - rows)].swap_with_slice(&mut self.cells[rows..]);
            self.cells = cells;
        }

        /// drop tail of {} rows. leave head as empty.
        fn down(&mut self, rows: usize) {
            let mut cells = _TextBuf::make(self.rows, self.cols);
            cells[rows..].swap_with_slice(&mut self.cells[..(self.rows - rows)]);
            self.cells = cells;
        }
    }

    #[derive(Debug, Default)]
    pub struct TextBuf {
        inner: RwLock<_TextBuf>,
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
        pub(super) fn up(&self, rows: usize) {
            //
            self.inner.write().unwrap().up(rows);
        }
        pub(super) fn down(&self, rows: usize) {
            self.inner.write().unwrap().down(rows);
        }
        pub(super) fn set_cells(&self, row: usize, col: usize, cells: &[super::TextCell]) {
            self.inner.write().unwrap().set_cells(row, col, cells);
        }

        pub(super) fn clear(&self) {
            log::warn!("textbuf cleared");
            self.inner.write().unwrap().clear();
        }

        pub(super) fn resize(&self, rows: usize, cols: usize) {
            self.inner.write().unwrap().resize(rows, cols);
        }

        pub(super) fn rows(&self) -> usize {
            self.inner.read().unwrap().rows
        }

        pub(super) fn cols(&self) -> usize {
            self.inner.read().unwrap().cols
        }

        pub(super) fn lines(&self) -> Lines {
            Lines {
                guard: self.inner.read().unwrap(),
                // at: 0,
            }
        }
    }

    trait TextBufExt {
        fn resize(&mut self, rows: usize, cols: usize);

        fn make(rows: usize, cols: usize) -> Box<[super::TextLine]> {
            let tl = super::TextLine::new(cols);
            vec![tl; rows].into_boxed_slice()
        }
    }

    impl TextBufExt for _TextBuf {
        fn resize(&mut self, rows: usize, cols: usize) {
            let old_rows = self.rows;
            let old_cols = self.cols;
            if old_rows == rows && old_cols == cols {
                return;
            }
            self.cols = cols;
            self.rows = rows;
            let nrows = rows.min(old_rows);
            let mut cells = vec![super::TextLine::new(0); rows];
            cells[..nrows].swap_with_slice(&mut self.cells[..nrows]);
            let cells: Vec<_> = cells
                .into_iter()
                .map(|tl| {
                    let mut tl = tl.into_inner().into_vec();
                    tl.resize(cols, super::TextCell::default());
                    super::TextLine(tl.into_boxed_slice())
                })
                .collect();

            log::debug!("buf cells resizing to {} rows from {}", rows, old_rows);

            self.cells = cells.into_boxed_slice();
        }
    }

    pub(super) struct Lines<'a> {
        guard: RwLockReadGuard<'a, _TextBuf>,
        // at: usize,
    }

    impl<'a> Lines<'a> {
        pub(super) fn get(&self, no: usize) -> Option<&super::TextLine> {
            self.guard.cells.get(no)
        }
    }

    // impl<'a> Iterator for Lines<'a> {
    //     type Item = &'a super::TextLine;
    //     fn next(&mut self) -> Option<Self::Item> {
    //         let at = self.at;
    //         self.at += 1;
    //         self.guard.cells.get(at)
    //     }
    // }
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

    pub fn set_cells(&self, row: usize, col: usize, cells: &[TextCell]) {
        self.imp().set_cells(row, col, cells);
    }

    pub fn up(&self, rows: usize) {
        self.imp().up(rows);
    }

    pub fn down(&self, rows: usize) {
        self.imp().down(rows);
    }

    pub(super) fn layout(
        &self,
        lineheight: i32,
        layout: &pango::Layout,
        hldefs: &HighlightDefinitions,
    ) {
        let imp = self.imp();
        if imp.cols() == 0 || imp.rows() == 0 {
            return;
        }
        const U16MAX: f32 = u16::MAX as f32 + 1.;
        let nchars = imp.cols() * imp.rows() + imp.rows();
        let mut text = String::with_capacity(nchars);
        let default_colors = hldefs.defaults().unwrap();
        let font_desc = layout.font_description().unwrap();
        let font_size = font_desc.size();
        log::error!(
            "layouting use font size {}/{} is absolute {}",
            font_size,
            font_size as f32 / pango::SCALE as f32,
            font_desc.is_size_absolute()
        );
        let attrs = pango::AttrList::new();
        // attrs.insert({
        //     let mut attr = pango::AttrInt::new_fallback(true);
        //     attr.set_start_index(0);
        //     attr.set_end_index(nchars as _);
        //     attr
        // });
        attrs.insert({
            log::error!("absolute line height set to {}", lineheight);
            let mut attr = pango::AttrInt::new_line_height_absolute(lineheight);
            attr.set_start_index(0);
            attr.set_end_index(nchars as _);
            attr
        });
        let default_hldef = hldefs.get(0).unwrap();
        let rows = imp.rows();
        let lines = imp.lines();
        for lno in 0..rows {
            let line_cells = lines.get(lno).unwrap();
            for cell in line_cells.iter() {
                if cell.text.is_empty() {
                    continue;
                }
                let start_index = text.len();
                text.push_str(&cell.text);
                let end_index = text.len();
                let mut background = None;
                let mut hldef = default_hldef;
                if let Some(ref id) = cell.hldef {
                    let style = hldefs.get(*id);
                    if let Some(style) = style {
                        background = style.background();
                        hldef = style;
                    }
                }
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
                    let mut attr = pango::AttrInt::new_underline(pango::Underline::Single);
                    attr.set_start_index(start_index as _);
                    attr.set_end_index(end_index as _);
                    attrs.insert(attr);
                }
                if hldef.undercurl {
                    let mut attr = pango::AttrInt::new_underline(pango::Underline::Error);
                    attr.set_start_index(start_index as _);
                    attr.set_end_index(end_index as _);
                    attrs.insert(attr);
                }
                // alpha color
                // let mut attr =
                //     pango::AttrInt::new_background_alpha(u16::MAX - (hldef.blend as u16).pow(2));
                // log::info!("blend {}", hldef.blend);
                // attr.set_start_index(start_index as _);
                // attr.set_end_index(end_index as _);
                // attrs.insert(attr);
                if let Some(fg) = hldef.colors.foreground.or(default_colors.foreground) {
                    // log::info!(
                    //     "foreground #{:.2x}{:.2x}{:.2x}",
                    //     (fg.red() * U16MAX) as u16,
                    //     (fg.green() * U16MAX) as u16,
                    //     (fg.blue() * U16MAX) as u16
                    // );
                    let mut attr = pango::AttrColor::new_foreground(
                        (fg.red() * U16MAX) as _,
                        (fg.green() * U16MAX) as _,
                        (fg.blue() * U16MAX) as _,
                    );
                    attr.set_start_index(start_index as _);
                    attr.set_end_index(end_index as _);
                    attrs.insert(attr);
                }
                if let Some(bg) = background {
                    // log::info!(
                    //     "background #{:.2x}{:.2x}{:.2x}",
                    //     (bg.red() * U16MAX) as u16,
                    //     (bg.green() * U16MAX) as u16,
                    //     (bg.blue() * U16MAX) as u16
                    // );
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
        // layout.set_ellipsize(pango::EllipsizeMode::None);
        // layout.set_justify(true);
        // layout.set_alignment(pango::Alignment::Center);
        // let mut tabs = pango::TabArray::new(2, false);
        // tabs.set_tab(0, pango::TabAlign::Left, 0);
        // tabs.set_tab(1, pango::TabAlign::Left, 1);
        // layout.set_tabs(Some(&tabs));
        layout.set_text(&text);
        layout.set_attributes(Some(&attrs));
        log::info!("pixel size {:?}", layout.pixel_size());
        // log::info!("layout height: {}", layout.height());
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
        log::error!("text to render:\n{}", &text);
    }

    pub(super) fn for_itemize(&self, hldefs: &HighlightDefinitions) -> (Box<[String]>, AttrTable) {
        let imp = self.imp();
        let mut texts = Vec::with_capacity(imp.rows());
        let mut attrtable = AttrTable::new();
        if imp.cols() == 0 || imp.rows() == 0 {
            return (texts.into_boxed_slice(), attrtable);
        }
        const U16MAX: f32 = u16::MAX as f32 + 1.;
        let default_colors = hldefs.defaults().unwrap();

        // attrs.insert({
        //     let mut attr = pango::AttrInt::new_fallback(true);
        //     attr.set_start_index(0);
        //     attr.set_end_index(nchars as _);
        //     attr
        // });
        let default_hldef = hldefs.get(0).unwrap();
        let rows = imp.rows();
        let lines = imp.lines();
        for lno in 0..rows {
            attrtable.insert(lno, pango::AttrList::new());
            texts.push(String::with_capacity(imp.rows()));
            let attrs = attrtable.get(lno).unwrap();
            let line_cells = lines.get(lno).unwrap();
            let text = texts.last_mut().unwrap();
            for cell in line_cells.iter() {
                if cell.text.is_empty() {
                    continue;
                }
                let start_index = text.len();
                text.push_str(&cell.text);
                let end_index = text.len();
                // attrtable.insert(lno, start_index, end_index, pango::AttrList::new());
                let mut background = None;
                let mut hldef = default_hldef;
                if let Some(ref id) = cell.hldef {
                    let style = hldefs.get(*id);
                    if let Some(style) = style {
                        background = style.background();
                        hldef = style;
                    }
                }
                if hldef.italic {
                    let mut attr = pango::AttrInt::new_style(pango::Style::Italic);
                    attr.set_start_index(start_index as _);
                    attr.set_end_index(end_index as _);
                    attrs.change(attr);
                }
                if hldef.bold {
                    let mut attr = pango::AttrInt::new_weight(pango::Weight::Semibold);
                    attr.set_start_index(start_index as _);
                    attr.set_end_index(end_index as _);
                    attrs.change(attr);
                }
                if hldef.strikethrough {
                    let mut attr = pango::AttrInt::new_strikethrough(true);
                    attr.set_start_index(start_index as _);
                    attr.set_end_index(end_index as _);
                    attrs.change(attr);
                }
                if hldef.underline {
                    let mut attr = pango::AttrInt::new_underline(pango::Underline::Single);
                    attr.set_start_index(start_index as _);
                    attr.set_end_index(end_index as _);
                    attrs.change(attr);
                }
                if hldef.undercurl {
                    let mut attr = pango::AttrInt::new_underline(pango::Underline::Error);
                    attr.set_start_index(start_index as _);
                    attr.set_end_index(end_index as _);
                    attrs.change(attr);
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
                    // log::info!(
                    //     "foreground #{:.2x}{:.2x}{:.2x}",
                    //     (fg.red() * U16MAX) as u16,
                    //     (fg.green() * U16MAX) as u16,
                    //     (fg.blue() * U16MAX) as u16
                    // );
                    let mut attr = pango::AttrColor::new_foreground(
                        (fg.red() * U16MAX) as _,
                        (fg.green() * U16MAX) as _,
                        (fg.blue() * U16MAX) as _,
                    );
                    attr.set_start_index(start_index as _);
                    attr.set_end_index(end_index as _);
                    attrs.change(attr);
                }
                if let Some(bg) = background {
                    // log::info!(
                    //     "background #{:.2x}{:.2x}{:.2x}",
                    //     (bg.red() * U16MAX) as u16,
                    //     (bg.green() * U16MAX) as u16,
                    //     (bg.blue() * U16MAX) as u16
                    // );
                    let mut attr = pango::AttrColor::new_background(
                        (bg.red() * U16MAX) as _,
                        (bg.green() * U16MAX) as _,
                        (bg.blue() * U16MAX) as _,
                    );
                    attr.set_start_index(start_index as _);
                    attr.set_end_index(end_index as _);
                    attrs.change(attr);
                }
                if let Some(special) = hldef.colors.special.or(default_colors.special) {
                    let mut attr = pango::AttrColor::new_underline_color(
                        (special.red() * U16MAX) as _,
                        (special.green() * U16MAX) as _,
                        (special.blue() * U16MAX) as _,
                    );
                    attr.set_start_index(start_index as _);
                    attr.set_end_index(end_index as _);
                    attrs.change(attr);
                }
            }
        }
        (texts.into_boxed_slice(), attrtable)
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
        let mut line = Vec::with_capacity(cols);
        line.resize(cols, TextCell::default());
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

impl From<Box<[TextCell]>> for TextLine {
    fn from(boxed: Box<[TextCell]>) -> Self {
        Self(boxed)
    }
}

impl Into<Box<[TextCell]>> for TextLine {
    fn into(self) -> Box<[TextCell]> {
        self.0
    }
}

impl TextLine {
    fn into_inner(self) -> Box<[TextCell]> {
        self.0
    }
}

pub struct AttrTable {
    // table: FxHashMap<(usize, usize, usize), pango::AttrList>,
    table: VecMap<usize, pango::AttrList>,
}

impl AttrTable {
    pub fn new() -> Self {
        AttrTable {
            table: VecMap::default(),
        }
    }

    /// lno: line number.
    pub fn get(
        &self,
        lno: usize,
        // start_index: usize,
        // end_index: usize,
    ) -> Option<&pango::AttrList> {
        self.table.get(&lno)
    }

    pub fn insert(
        &mut self,
        lno: usize,
        // start_index: usize,
        // end_index: usize,
        attrs: pango::AttrList,
    ) {
        self.table.insert(lno, attrs);
    }
}
