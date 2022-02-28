use std::cell::Cell;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;

use glib::subclass::prelude::*;
use parking_lot::RwLock;

use super::highlights::HighlightDefinitions;

mod imp {
    use std::cell::Cell;

    use std::rc::Rc;

    use glib::prelude::*;
    use glib::subclass::prelude::*;
    use parking_lot::{RwLock, RwLockReadGuard};

    use crate::vimview::HighlightDefinitions;

    #[derive(Derivative)]
    #[derivative(Debug)]
    pub struct _TextBuf {
        rows: usize,
        cols: usize,
        cells: Box<[super::TextLine]>,
        metrics: Option<Rc<Cell<crate::metrics::Metrics>>>,

        #[derivative(Debug = "ignore")]
        hldefs: Option<Rc<RwLock<HighlightDefinitions>>>,

        #[derivative(Debug = "ignore")]
        pctx: Option<Rc<pango::Context>>,
    }

    impl Default for _TextBuf {
        fn default() -> Self {
            _TextBuf::new(1, 1)
        }
    }

    impl _TextBuf {
        fn new(rows: usize, cols: usize) -> _TextBuf {
            let cells = _TextBuf::make(rows, cols);
            _TextBuf {
                rows,
                cols,
                cells,
                pctx: None,
                hldefs: None,
                metrics: None,
            }
        }

        fn clear(&mut self) {
            self.cells = _TextBuf::make(self.rows, self.cols);
        }

        fn reset_cache(&mut self) {
            let pctx = self.pctx.as_ref().unwrap();
            let hldefs = self.hldefs.as_ref().unwrap().read();
            let metrics = self.metrics.as_ref().unwrap().get();
            self.cells.iter_mut().for_each(|line| {
                line.iter_mut().for_each(|cell| {
                    cell.reset_attrs(pctx, &hldefs, &metrics);
                });
            });
        }

        pub fn set_hldefs(&mut self, hldefs: Rc<RwLock<HighlightDefinitions>>) {
            self.hldefs.replace(hldefs);
        }

        pub fn set_metrics(&mut self, metrics: Rc<Cell<crate::metrics::Metrics>>) {
            self.metrics.replace(metrics);
        }

        pub fn set_pango_context(&mut self, pctx: Rc<pango::Context>) {
            self.pctx.replace(pctx);
        }

        fn set_cells(&mut self, row: usize, col: usize, cells: &[crate::bridge::GridLineCell]) {
            let nrows = self.rows;
            let ncols = self.cols;
            if nrows <= row {
                log::error!(
                    "set cells dest line {} dose not exists, total {} lines.",
                    row,
                    nrows
                );
                return;
            }
            let line = &self.cells[row];
            let pctx = self.pctx.as_ref().unwrap();
            let hldefs = self.hldefs.as_ref().unwrap().read();
            let metrics = self.metrics.as_ref().unwrap().get();
            let mut expands = Vec::with_capacity(line.len());
            let mut start_index = line.get(col).map(|cell| cell.start_index).unwrap_or(0);
            for cell in cells.iter() {
                let crate::bridge::GridLineCell {
                    text,
                    hldef,
                    repeat,
                    double_width,
                } = cell;
                for _ in 0..repeat.unwrap_or(1) {
                    // FIXME: invalid start_index
                    let end_index = start_index + text.len();
                    let attrs = Vec::new();
                    let mut cell = super::TextCell {
                        text: text.to_string(),
                        hldef: hldef.clone(),
                        double_width: *double_width,
                        attrs,
                        start_index,
                        end_index,
                    };
                    cell.reset_attrs(pctx, &hldefs, &metrics);
                    log::debug!(
                        "Setting cell {}x{} start_index {} end_index {}",
                        row,
                        col + expands.len(),
                        start_index,
                        end_index
                    );
                    expands.push(cell);
                    start_index = end_index;
                }
            }
            let col_to = col + expands.len();
            // line.iter()
            //     .enumerate()
            //     .skip(col)
            //     .take(expands.len())
            //     .for_each(|(idx, cell)| {
            //         log::info!(
            //             "old cell {} start_index {} end_index {}",
            //             idx,
            //             cell.start_index,
            //             cell.end_index
            //         )
            //     });
            log::debug!(
                "textbuf {}x{} setting line {} with {} cells from {} to {}",
                ncols,
                nrows,
                row,
                expands.len(),
                col,
                col_to
            );
            let line = &mut self.cells[row];
            line[col..col_to].swap_with_slice(&mut expands);
            line.iter_mut().fold(0, |start_index, cell| {
                cell.start_index = start_index;
                cell.end_index = start_index + cell.text.len();
                cell.reset_attrs(pctx, &hldefs, &metrics);
                cell.end_index
            });
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

        fn pango_context(&self) -> Rc<pango::Context> {
            self.pctx.clone().unwrap()
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
            self.inner.write().up(rows);
        }
        pub(super) fn down(&self, rows: usize) {
            self.inner.write().down(rows);
        }

        pub(super) fn set_cells(
            &self,
            row: usize,
            col: usize,
            cells: &[crate::bridge::GridLineCell],
        ) {
            self.inner.write().set_cells(row, col, cells);
        }

        pub(super) fn set_hldefs(&self, hldefs: Rc<RwLock<HighlightDefinitions>>) {
            self.inner.write().set_hldefs(hldefs);
        }

        pub(super) fn set_metrics(&self, metrics: Rc<Cell<crate::metrics::Metrics>>) {
            self.inner.write().set_metrics(metrics);
        }

        pub(super) fn set_pango_context(&self, pctx: Rc<pango::Context>) {
            self.inner.write().set_pango_context(pctx);
        }

        pub(super) fn pango_context(&self) -> Rc<pango::Context> {
            self.inner.write().pango_context()
        }

        pub fn cell(&self, row: usize, col: usize) -> Option<super::TextCell> {
            self.lines()
                .get(row)
                .and_then(|line| line.get(col))
                .cloned()
        }

        pub(super) fn reset_cache(&self) {
            log::warn!("textbuf rebuild cache");
            self.inner.write().reset_cache();
        }

        pub(super) fn clear(&self) {
            log::warn!("textbuf cleared");
            self.inner.write().clear();
        }

        pub(super) fn resize(&self, rows: usize, cols: usize) {
            self.inner.write().resize(rows, cols);
        }

        pub(super) fn rows(&self) -> usize {
            self.inner.read().rows
        }

        pub(super) fn cols(&self) -> usize {
            self.inner.read().cols
        }

        pub(super) fn lines(&self) -> Lines {
            Lines {
                guard: self.inner.read(),
            }
        }

        pub(super) fn hldefs(&self) -> Option<Rc<RwLock<HighlightDefinitions>>> {
            self.inner.read().hldefs.clone()
        }

        pub(super) fn metrics(&self) -> Option<Rc<Cell<crate::metrics::Metrics>>> {
            self.inner.read().metrics.clone()
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
                    let mut start_index = tl.last().map(|last| last.start_index).unwrap_or(0);
                    let old_cols = tl.len();
                    tl.resize(cols, super::TextCell::default());
                    if cols > old_cols {
                        tl.iter_mut().skip(old_cols).for_each(|cell| {
                            cell.start_index = start_index;
                            cell.end_index = start_index + 1;
                            start_index += 1;
                        });
                    }
                    super::TextLine(tl.into_boxed_slice())
                })
                .collect();

            log::debug!(
                "resizing buf cells from {}x{} to {}x{}",
                old_cols,
                old_rows,
                cols,
                rows
            );

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

    pub fn hldefs(&self) -> Option<Rc<RwLock<HighlightDefinitions>>> {
        self.imp().hldefs()
    }

    pub fn metrics(&self) -> Option<Rc<Cell<crate::metrics::Metrics>>> {
        self.imp().metrics()
    }

    pub fn set_cells(&self, row: usize, col: usize, cells: &[crate::bridge::GridLineCell]) {
        self.imp().set_cells(row, col, cells);
    }

    pub fn set_hldefs(&self, hldefs: Rc<RwLock<HighlightDefinitions>>) {
        self.imp().set_hldefs(hldefs);
    }
    pub fn set_metrics(&self, metrics: Rc<Cell<crate::metrics::Metrics>>) {
        self.imp().set_metrics(metrics);
    }

    pub fn set_pango_context(&self, pctx: Rc<pango::Context>) {
        self.imp().set_pango_context(pctx);
    }

    pub fn pango_context(&self) -> Rc<pango::Context> {
        self.imp().pango_context()
    }

    pub fn cell(&self, row: usize, col: usize) -> Option<TextCell> {
        self.imp().cell(row, col)
    }

    pub fn up(&self, rows: usize) {
        self.imp().up(rows);
    }

    pub fn down(&self, rows: usize) {
        self.imp().down(rows);
    }

    pub fn reset_cache(&self) {
        self.imp().reset_cache();
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextCell {
    pub text: String,
    pub hldef: Option<u64>,
    pub double_width: bool,
    pub attrs: Vec<pango::Attribute>,
    pub start_index: usize,
    pub end_index: usize,
}

impl Default for TextCell {
    fn default() -> TextCell {
        TextCell {
            text: String::from(" "),
            hldef: None,
            double_width: false,
            attrs: Vec::new(),
            start_index: 0,
            end_index: 0,
        }
    }
}

impl TextCell {
    fn reset_attrs(
        &mut self,
        pctx: &pango::Context,
        hldefs: &HighlightDefinitions,
        metrics: &crate::metrics::Metrics,
    ) {
        const U16MAX: f32 = u16::MAX as f32;
        const PANGO_SCALE: f64 = pango::SCALE as f64;

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
}

#[derive(Clone, Debug, Default)]
pub struct TextLine(Box<[TextCell]>);

impl TextLine {
    fn new(cols: usize) -> TextLine {
        let mut line = Vec::with_capacity(cols);
        line.resize(cols, TextCell::default());
        line.iter_mut().enumerate().for_each(|(start_index, cell)| {
            cell.start_index = start_index;
            cell.end_index = start_index + 1;
        });
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
