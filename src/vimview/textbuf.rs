use std::cell::Cell;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;

use glib::subclass::prelude::*;
use parking_lot::RwLock;

use super::highlights::HighlightDefinitions;

type Nr = usize;

mod imp {
    use std::cell::Cell;
    use std::rc::Rc;

    use glib::subclass::prelude::*;
    use parking_lot::{RwLock, RwLockReadGuard};

    use crate::vimview::HighlightDefinitions;

    use super::Nr;

    #[derive(Derivative)]
    #[derivative(Debug)]
    pub struct _TextBuf {
        rows: usize,
        cols: usize,

        top: f64,
        bottom: f64,

        #[derivative(Debug = "ignore")]
        cells: Vec<super::TextLine>,
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
        fn make_cells(rows: usize, cols: usize) -> Vec<super::TextLine> {
            let tl = super::TextLine::new(cols);
            vec![tl; rows]
        }

        fn new(rows: usize, cols: usize) -> _TextBuf {
            let cells = _TextBuf::make_cells(rows, cols);
            _TextBuf {
                rows,
                cols,
                top: 0.,
                bottom: 0.,
                cells,
                pctx: None,
                hldefs: None,
                metrics: None,
            }
        }

        fn clear(&mut self) {
            self.cells = _TextBuf::make_cells(self.rows, self.cols);
            let nr = self.top.floor() as usize;
            self.cells
                .iter_mut()
                .enumerate()
                .for_each(|(idx, tl)| tl.nr = nr + idx);
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

        pub fn nlines(&self) -> usize {
            self.cells.len()
        }

        /// 在scroll之后丢弃之前(已失效)的cells
        pub fn discard(&mut self) {
            // discard before top.
            for tl in self.cells.iter() {
                let mut s = String::with_capacity(20);
                let end = tl.len().min(10);
                tl[..end].iter().for_each(|c| s.push_str(&c.text));
                log::error!("x {}: '{}'", tl.nr(), s);
            }
            let nrs = self.cells.iter().map(|tl| tl.nr()).collect::<Vec<_>>();
            log::error!("{} nrs {} {:?}", self.top, nrs.len(), nrs);
            let top = self.top.floor() as Nr;
            let mut idx = 0;
            for tl in self.cells.iter() {
                if tl.nr() >= top {
                    break;
                }
                idx += 1;
            }
            log::error!("before point: {}", idx);
            let _: Vec<_> = self.cells.drain(0..idx).collect();
            log::error!("after erase before {}", self.cells.len());
            // discard after bottom.
            // let bottom = self.rows + self.top.ceil() as Nr;
            // let after = self.cells.partition_point(|line| line.nr <= bottom);
            log::error!(
                "before {} rows discarded, now {} should be {}.",
                idx,
                self.cells.len(),
                self.rows
            );
            self.cells.truncate(self.rows);
            log::error!("remain {} rows after discarded.", self.cells.len());
            for tl in self.cells.iter() {
                let mut s = String::with_capacity(20);
                let end = tl.len().min(10);
                tl[..end].iter().for_each(|c| s.push_str(&c.text));
                log::error!("{}: '{}'", tl.nr(), s);
            }
        }

        pub fn set_viewport(&mut self, top: f64, bottom: f64) {
            log::error!(
                "setting viewport {}-{} / {}-{}",
                top,
                bottom,
                self.top,
                self.bottom
            );
            if top == self.top && bottom == self.bottom {
                log::error!("setting viewport {}-{} ignored.", top, bottom);
                return;
            }
            log::error!(
                "setting viewport {}-{} old nrs: {:?}",
                top,
                bottom,
                self.cells.iter().map(|tl| tl.nr()).collect::<Vec<_>>()
            );
            if self.top == 0. {
                let topidx = top.floor() as usize;
                for (idx, line) in self.cells.iter_mut().enumerate() {
                    line.nr = topidx + idx;
                }
                self.top = top;
                self.bottom = bottom;
                log::error!(
                    "setting viewport {}-{} new nrs: {:?}",
                    top,
                    bottom,
                    self.cells.iter().map(|tl| tl.nr()).collect::<Vec<_>>()
                );
                return;
            }
            let first_nr = self.cells.first().unwrap().nr() as f64;
            if self.top > top && first_nr > top {
                assert!(
                    first_nr >= 1.,
                    "first_nr({}) should >= 1 top {}",
                    first_nr,
                    top
                );
                let nr = (first_nr - 1.) as usize;
                let rows_prepend = (first_nr - top).floor() as usize;
                log::error!(
                    "setting viewport preppend accord top {} nr {} first_nr {} with {}",
                    top,
                    nr,
                    first_nr,
                    rows_prepend
                );
                assert!(first_nr as usize >= rows_prepend);
                let mut lines = vec![super::TextLine::new(self.cols); rows_prepend];
                lines.iter_mut().rev().enumerate().for_each(|(idx, line)| {
                    line.nr = nr - idx;
                });
                let _ = self.cells.splice(0..0, lines).collect::<Vec<_>>();
                assert_eq!(self.cells.first().unwrap().nr(), top as Nr);
                assert!(
                    self.cells.len() > self.rows,
                    "{} > {}",
                    self.cells.len(),
                    self.rows
                );
            }
            let last_nr = self.cells.last().unwrap().nr() as f64;
            let required_rows = self.rows.max((bottom - top).floor() as usize);
            let rows = (last_nr - top).floor() as usize;
            if rows < required_rows {
                let nr = last_nr as usize + 1;
                let rows_append = required_rows - rows;
                log::error!(
                    "setting viewport max nr {} required max nr {} append {}",
                    last_nr,
                    top + self.rows as f64,
                    rows_append
                );
                let mut lines = vec![super::TextLine::new(self.cols); rows_append];
                lines.iter_mut().enumerate().for_each(|(idx, line)| {
                    line.nr = nr + idx;
                });
                self.cells.extend(lines);
                assert!(
                    self.cells.last().unwrap().nr() >= bottom as Nr,
                    "setting viewport {} nrs: {:?}",
                    bottom,
                    self.cells.iter().map(|c| c.nr()).collect::<Vec<_>>()
                );
                assert!(
                    self.cells.len() > self.rows,
                    "setting viewport {} > {}",
                    self.cells.len(),
                    self.rows
                );
            }
            log::error!(
                "setting viewport {}-{} new nrs: {:?}",
                top,
                bottom,
                self.cells.iter().map(|tl| tl.nr()).collect::<Vec<_>>()
            );
            self.top = top;
            self.bottom = bottom;
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
            let nrows = (self.bottom - self.top).ceil() as usize;
            let rows = self.cells.len();
            let ncols = self.cols;
            assert!(self.rows <= rows, "{} <= {}", self.rows, rows);
            // if nrows <= row {
            //     log::error!(
            //         "set cells dest line {} dose not exists, total {} lines.",
            //         row,
            //         nrows
            //     );
            //     return;
            // }
            let nr = self.top.floor() as usize + row;
            let nrs = self.cells.iter().map(|c| c.nr).collect::<Vec<_>>();
            let line = &self
                .cells
                .iter_mut()
                .find(|line| line.nr == nr)
                .or_else(|| {
                    log::error!(
                        "current top: {} bottom: {} cols: {} rows: {}/{} of nr {} nrs: {:?}",
                        self.top,
                        self.bottom,
                        self.cols,
                        self.rows,
                        rows,
                        nr,
                        &nrs
                    );
                    None
                })
                .unwrap();
            line.cache.set(None);
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
                    log::trace!(
                        "Setting cell {}[{}] start_index {} end_index {}",
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
            log::info!(
                "textbuf {}-{} {}x{}/{} setting line {}/{} with {} cells from {} to {}",
                self.top,
                self.bottom,
                ncols,
                rows,
                nrows,
                line.nr(),
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
            // self.cells.last().unwrap().nr + self.top.floor() as usize + self.rows
            // let nr = self.cells.last().unwrap().nr + 1;
            // let mut lines = vec![super::TextLine::new(self.cols); rows];
            // lines.iter_mut().enumerate().for_each(|(idx, line)| {
            //     line.nr = nr + idx;
            // });
            // self.cells.extend(lines);
        }

        /// drop tail of {} rows. leave head as empty.
        fn down(&mut self, rows: usize) {
            // let nr = self.cells.first().unwrap().nr - 1;
            // let mut lines = vec![super::TextLine::new(self.cols); rows];
            // lines.iter_mut().rev().enumerate().for_each(|(idx, line)| {
            //     line.nr = nr - idx;
            // });
            // let _ = self.cells.splice(0..0, lines).collect::<Vec<_>>();
        }

        fn pango_context(&self) -> Rc<pango::Context> {
            self.pctx.clone().unwrap()
        }

        fn resize(&mut self, rows: usize, cols: usize) {
            self.discard();
            let old_rows = self.rows;
            let old_cols = self.cols;
            if old_rows == rows && old_cols == cols {
                return;
            }
            // let nrows = rows.min(old_rows);
            match rows {
                rows if rows < self.rows => {
                    log::error!("resizing truncate from {} to {}", self.cells.len(), rows);
                    self.cells.truncate(rows);
                }
                rows if rows == self.rows => {
                    // do not change, do nothing.
                }
                _ => {
                    log::error!("resizing extend from {} to {}", self.cells.len(), rows);
                    let mut lines = vec![super::TextLine::new(self.cols); rows - self.cells.len()];
                    let nr = self
                        .cells
                        .last()
                        .map(|tl| tl.nr() + 1)
                        .unwrap_or(self.top as usize);
                    lines
                        .iter_mut()
                        .enumerate()
                        .for_each(|(idx, line)| line.nr = nr + idx);
                    self.cells.extend(lines);
                }
            };

            assert_eq!(self.cells.len(), rows);

            self.cols = cols;
            self.rows = rows;

            if old_cols == cols {
                return;
            }
            self.cells.iter_mut().for_each(|tl| {
                let mut start_index = tl.last().map(|last| last.start_index).unwrap_or(0);
                let old = std::mem::take(&mut tl.boxed);
                let mut cells: Vec<_> = old.into();
                if cols > old_cols {
                    for _ in 0..(cols - old_cols) {
                        cells.push({
                            let mut cell = super::TextCell::default();
                            cell.start_index = start_index;
                            cell.end_index = start_index + 1;
                            start_index += 1;
                            cell
                        });
                    }
                } else {
                    cells.truncate(cols);
                }
                assert_eq!(cells.len(), cols);
                tl.boxed = cells.into_boxed_slice();
            });
            assert_eq!(self.cells.len(), rows);

            log::info!(
                "resizing buf cells from {}x{} to {}x{} {}",
                old_cols,
                old_rows,
                cols,
                rows,
                self.cells.len()
            );
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
            log::debug!("textbuf rebuild cache");
            self.inner.write().reset_cache();
        }

        pub(super) fn clear(&self) {
            log::debug!("textbuf cleared");
            self.inner.write().clear();
        }

        pub(super) fn discard(&self) {
            self.inner.write().discard();
        }

        pub(super) fn resize(&self, rows: usize, cols: usize) {
            self.inner.write().resize(rows, cols);
        }

        pub(super) fn set_viewport(&self, top: f64, bottom: f64) {
            self.inner.write().set_viewport(top, bottom);
        }

        pub(super) fn nlines(&self) -> usize {
            self.inner.read().nlines()
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

    pub struct Lines<'a> {
        guard: RwLockReadGuard<'a, _TextBuf>,
    }

    impl<'a> Lines<'a> {
        pub fn get(&self, no: usize) -> Option<&super::TextLine> {
            self.guard.cells.get(no)
        }

        pub fn iter(&self) -> LineIter {
            LineIter {
                lines: self,
                index: 0,
            }
        }
    }

    pub struct LineIter<'a, 'b> {
        lines: &'b Lines<'a>,
        index: usize,
    }

    impl<'b, 'a: 'b> Iterator for LineIter<'a, 'b> {
        type Item = &'b super::TextLine;
        fn next(&mut self) -> Option<Self::Item> {
            let v = self.lines.get(self.index);
            self.index += 1;
            v
        }
    }
}

pub use imp::Lines;

glib::wrapper! {
    pub struct TextBuf(ObjectSubclass<imp::TextBuf>);
}

unsafe impl Sync for TextBuf {}
unsafe impl Send for TextBuf {}

impl TextBuf {
    pub fn new(cols: usize, rows: usize) -> Self {
        let tb = glib::Object::new::<Self>(&[]).expect("Failed to initialize TextBuf object");
        tb.resize(rows, cols);
        tb
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

    pub fn set_viewport(&self, top: f64, bottom: f64) {
        self.imp().set_viewport(top, bottom);
    }

    pub fn nlines(&self) -> usize {
        self.imp().nlines()
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

    pub fn lines(&self) -> Lines {
        self.imp().lines()
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

    pub fn discard(&self) {
        self.imp().discard();
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
}

#[derive(Default)]
pub struct TextLine {
    nr: Nr,
    boxed: Box<[TextCell]>,
    cache: Cell<Option<(pango::Layout, pango::LayoutLine)>>,
}

impl Clone for TextLine {
    fn clone(&self) -> Self {
        TextLine {
            nr: self.nr,
            boxed: self.boxed.clone(),
            cache: Cell::new(unsafe { &*self.cache.as_ptr() }.clone()),
        }
    }
}

impl TextLine {
    fn new(cols: usize) -> TextLine {
        let mut line = Vec::with_capacity(cols);
        line.resize(cols, TextCell::default());
        line.iter_mut().enumerate().for_each(|(start_index, cell)| {
            cell.start_index = start_index;
            cell.end_index = start_index + 1;
        });
        Self {
            nr: 0,
            cache: Cell::new(None),
            boxed: line.into_boxed_slice(),
        }
    }

    pub fn nr(&self) -> Nr {
        self.nr
    }

    pub fn cache(&self) -> Option<(pango::Layout, pango::LayoutLine)> {
        unsafe { &*self.cache.as_ptr() }.clone()
    }

    pub fn set_cache(&self, layout: pango::Layout, line: pango::LayoutLine) {
        self.cache.set((layout, line).into());
    }
}

impl Deref for TextLine {
    type Target = [TextCell];

    fn deref(&self) -> &[TextCell] {
        &self.boxed
    }
}

impl DerefMut for TextLine {
    fn deref_mut(&mut self) -> &mut [TextCell] {
        &mut self.boxed
    }
}

impl AsRef<[TextCell]> for TextLine {
    fn as_ref(&self) -> &[TextCell] {
        &self.boxed
    }
}

impl AsMut<[TextCell]> for TextLine {
    fn as_mut(&mut self) -> &mut [TextCell] {
        &mut self.boxed
    }
}

impl From<Box<[TextCell]>> for TextLine {
    fn from(boxed: Box<[TextCell]>) -> Self {
        TextLine {
            boxed,
            ..Default::default()
        }
    }
}

impl Into<Box<[TextCell]>> for TextLine {
    fn into(self) -> Box<[TextCell]> {
        self.boxed
    }
}
