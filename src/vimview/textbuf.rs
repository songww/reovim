use std::cell::Cell;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;

use glib::subclass::prelude::*;
use parking_lot::RwLock;

use super::highlights::HighlightDefinitions;
use crate::text::{TextCell, TextLine};

type Nr = usize;

mod imp {
    use std::cell::Cell;
    use std::collections::VecDeque;
    use std::rc::Rc;

    use glib::subclass::prelude::*;
    use parking_lot::{RwLock, RwLockReadGuard};

    use crate::text::{TextCell, TextLine};
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
        textlines: VecDeque<TextLine>,
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
        fn make_textlines(rows: usize, cols: usize) -> VecDeque<TextLine> {
            let tl = TextLine::new(cols);
            let mut textlines = VecDeque::with_capacity(rows);
            textlines.resize(rows, tl);
            textlines
        }

        fn new(rows: usize, cols: usize) -> _TextBuf {
            let textlines = _TextBuf::make_textlines(rows, cols);
            _TextBuf {
                rows,
                cols,
                top: 0.,
                bottom: 0.,
                textlines,
                pctx: None,
                hldefs: None,
                metrics: None,
            }
        }

        fn clear(&mut self) {
            self.textlines = _TextBuf::make_textlines(self.rows, self.cols);
            let nr = self.top.floor() as usize;
            self.textlines
                .iter_mut()
                .enumerate()
                .for_each(|(idx, tl)| tl.nr = nr + idx);
        }

        fn reset_cache(&mut self) {
            let pctx = self.pctx.as_ref().unwrap();
            let hldefs = self.hldefs.as_ref().unwrap().read();
            let metrics = self.metrics.as_ref().unwrap().get();
            self.textlines.iter_mut().for_each(|line| {
                line.iter_mut().for_each(|cell| {
                    cell.reset_attrs(pctx, &hldefs, &metrics);
                });
            });
        }

        pub fn nlines(&self) -> usize {
            self.textlines.len()
        }

        fn ensure_rows(&mut self, rows: usize) {
            match rows {
                rows if rows < self.textlines.len() => {
                    log::error!(
                        "resizing truncate from {} to {}",
                        self.textlines.len(),
                        rows
                    );
                    self.textlines.truncate(rows);
                }
                rows if rows == self.textlines.len() => {
                    // do not change, do nothing.
                }
                _ => {
                    self.append_rows(rows - self.textlines.len(), None);
                }
            };
        }

        fn append_rows(&mut self, rows: usize, nr: impl Into<Option<Nr>>) {
            log::error!("resizing extend {} from {}", rows, self.textlines.len());
            let mut lines = vec![TextLine::new(self.cols); rows];
            let nr = nr.into().unwrap_or_else(|| {
                self.textlines
                    .back()
                    .map(|tl| tl.nr() + 1)
                    .unwrap_or(self.top as usize)
            });
            lines
                .iter_mut()
                .enumerate()
                .for_each(|(idx, line)| line.nr = nr + idx);
            self.textlines.extend(lines);
        }

        /// 在scroll之后丢弃之前(已失效)的cells
        pub fn discard(&mut self) {
            // discard before top.
            for tl in self.textlines.iter() {
                let mut s = String::with_capacity(20);
                let end = tl.len().min(10);
                tl[..end].iter().for_each(|c| s.push_str(&c.text));
                log::error!("x {}: '{}'", tl.nr(), s);
            }
            let nrs = self.textlines.iter().map(|tl| tl.nr()).collect::<Vec<_>>();
            log::error!("{} nrs {} {:?}", self.top, nrs.len(), nrs);
            let top = self.top.floor() as Nr;

            let mut dropped = 0;
            while let Some(front) = self.textlines.front() {
                if front.nr() < top {
                    dropped += 1;
                    self.textlines.pop_front();
                } else {
                    break;
                }
            }

            log::error!(
                "before {} erased, lift {} elements",
                dropped,
                self.textlines.len()
            );
            if self.textlines.is_empty() {
                self.textlines = _TextBuf::make_textlines(self.rows, self.cols);
                return;
            }
            // discard bottom.
            if self.textlines.len() > self.rows {
                self.textlines.truncate(self.rows);
            }

            // make sure in right size.
            if self.textlines.len() != self.rows {
                self.ensure_rows(self.rows);
            }

            let mut prevnr = self.textlines.front().unwrap().nr();
            // let mut split_at = None;
            for tl in self.textlines.iter().skip(1) {
                assert_eq!(tl.nr(), prevnr + 1);
                prevnr = tl.nr();
            }

            assert_eq!(self.rows, self.textlines.len());
            // if let Some(at) = split_at {
            //     self.textlines.truncate(at);
            // }

            for tl in self.textlines.iter() {
                let mut s = String::with_capacity(20);
                let end = tl.len().min(10);
                tl[..end].iter().for_each(|c| s.push_str(&c.text));
                log::error!(
                    "discarded {}-{} {}: '{}'",
                    self.top,
                    self.bottom,
                    tl.nr(),
                    s
                );
            }
        }

        pub fn flush_nrs(&mut self) {
            let topnr = self.top.floor() as usize;
            for (idx, tl) in self.textlines.iter_mut().enumerate() {
                tl.nr = topnr + idx;
            }
        }

        pub fn set_viewport(&mut self, top: f64, bottom: f64) {
            log::info!(
                "setting viewport {}-{} / {}-{}",
                top,
                bottom,
                self.top,
                self.bottom
            );
            let self_top = self.top;
            let self_bottom = self.bottom;
            self.top = top;
            self.bottom = bottom;
            if (self_top, self_bottom) == (0., 0.) {
                self.flush_nrs();
                return;
            }
            if top == self_top {
                log::debug!("setting viewport {}-{} ignored.", top, bottom);
                self.top = top;
                self.bottom = bottom;
                return;
            }
            log::error!(
                "setting viewport {}-{} old nrs: {:?}",
                top,
                bottom,
                self.textlines.iter().map(|tl| tl.nr()).collect::<Vec<_>>()
            );
            let topusize = top.floor() as usize;

            let botnr = self.textlines.back().unwrap().nr();
            let topnr = self.textlines.front().unwrap().nr();
            if topusize < topnr {
                // push_front 5 4 3 2 1
                for nr in (topusize..topnr).rev() {
                    let mut elt = TextLine::new(self.cols);
                    elt.nr = nr;
                    self.textlines.push_front(elt);
                }
                log::error!("topusize: {} topnr: {}", topusize, topnr);
                log::error!(
                    "setting viewport {}-{} insert {} at front",
                    top,
                    bottom,
                    topnr - topusize
                );
                if topusize < botnr {
                    // FIXME: preserve line number.
                    // new cells maybe not contains line number, if relativenumber is enabled.
                }
            } else if botnr < topusize {
                let rows = (bottom - top).floor() as usize;
                self.append_rows(rows, topusize);
                // FIXME: preserve line number.
                // new cells maybe not contains line number, if relativenumber is enabled.
            }

            // make sure that has self.rows after top.
            let mut start_at = None;
            let mut insert_at = None;
            for (idx, tl) in self.textlines.iter().enumerate() {
                if tl.nr() == topusize {
                    start_at.replace(idx);
                    break;
                }
                if tl.nr() > topusize {
                    insert_at.replace((idx, tl.nr()));
                    break;
                }
            }
            log::error!(
                "setting viewport {}-{} start-at {:?} insert-at {:?}",
                top,
                bottom,
                start_at,
                insert_at,
            );
            if let Some((idx, lastnr)) = insert_at {
                // lastnr - topusize
                let mut afters = self.textlines.split_off(idx);
                for nr in topusize..lastnr {
                    let mut elt = TextLine::new(self.cols);
                    elt.nr = nr;
                    self.textlines.push_back(elt);
                }
                self.textlines.append(&mut afters);
                // check from idx
                // elements before idx is ok.
                start_at.replace(idx);
            }
            // remains to check.
            let mut remains = self.rows as isize;
            while let Some(at) = start_at.take() {
                log::info!("still should append to end {}", remains);
                assert!(
                    remains.is_positive(),
                    "remains should be positive ({}).",
                    remains
                );
                let mut iter = self.textlines.iter().skip(at).take(remains as usize);
                // 检查是否连续,且长度是否够self.rows.
                let mut rows_contiguous = 0;
                let mut lacks = None;
                // which exists exactly.
                let mut prevnr = iter.next().unwrap().nr();
                for tl in iter {
                    if prevnr + 1 != tl.nr() {
                        lacks.replace(tl.nr() - prevnr - 1);
                        break;
                    }
                    remains -= 1;
                    rows_contiguous += 1;
                    prevnr = tl.nr();
                }
                if let Some(lacks) = lacks {
                    assert!(
                        remains.is_positive(),
                        "remains should be positive ({}).",
                        remains
                    );
                    // at + rows 处 append (self.rows - rows)
                    let mut afters = self.textlines.split_off(at + rows_contiguous + 1);
                    for offset in 0..((self.rows - rows_contiguous).min(lacks)) {
                        let mut elt = TextLine::new(self.cols);
                        elt.nr = prevnr + offset + 1;
                        self.textlines.push_back(elt);
                    }
                    self.textlines.append(&mut afters);
                    start_at.replace(at + rows_contiguous + 1);
                }
            }
            log::info!("finnal remains {}", remains);
            if remains.is_positive() {
                self.append_rows(remains as usize, None);
            }
            log::error!(
                "setting viewport {}-{} new nrs: {:?}",
                top,
                bottom,
                self.textlines.iter().map(|tl| tl.nr()).collect::<Vec<_>>()
            );
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
            let rows = self.textlines.len();
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
            let nrs = self.textlines.iter().map(|c| c.nr).collect::<Vec<_>>();
            let line = &mut self
                .textlines
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
                    let mut cell = TextCell {
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
            // let line = &mut self.textlines[row];
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

            log::error!(
                "resizing from {}x{} to {}x{}",
                old_rows,
                old_cols,
                rows,
                cols
            );
            self.ensure_rows(rows);

            assert_eq!(self.textlines.len(), rows);

            self.cols = cols;
            self.rows = rows;

            if old_cols == cols {
                return;
            }
            self.textlines.iter_mut().for_each(|tl| {
                let mut start_index = tl.last().map(|last| last.start_index).unwrap_or(0);
                let old = std::mem::take(&mut tl.boxed);
                let mut cells: Vec<_> = old.into();
                if cols > old_cols {
                    for _ in 0..(cols - old_cols) {
                        cells.push({
                            let mut cell = TextCell::default();
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
            assert_eq!(self.textlines.len(), rows);

            log::info!(
                "resizing buf cells from {}x{} to {}x{} {}",
                old_cols,
                old_rows,
                cols,
                rows,
                self.textlines.len()
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

        pub fn cell(&self, row: usize, col: usize) -> Option<TextCell> {
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
        pub fn get(&self, no: usize) -> Option<&TextLine> {
            self.guard.textlines.get(no)
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
        type Item = &'b TextLine;
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
