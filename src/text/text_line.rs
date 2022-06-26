use std::cell::Cell;
use std::fmt;
use std::ops::{Deref, DerefMut};

use super::{Nr, TextCell};

#[derive(Clone, Copy, Debug)]
pub struct LayoutCache;

impl LayoutCache {
    pub fn build(tl: &TextLine) -> Self {
        LayoutCache
    }
}

#[derive(Default)]
pub struct TextLine {
    nr: Nr,
    boxed: Box<[TextCell]>,
    cache: Cell<Option<LayoutCache>>,
}

impl Clone for TextLine {
    fn clone(&self) -> Self {
        TextLine {
            nr: self.nr,
            boxed: self.boxed.clone(),
            cache: Cell::new(*unsafe { &*self.cache.as_ptr() }),
        }
    }
}

impl fmt::Debug for TextLine {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(
            fmt,
            "TextLine {{ nr: {}, cells: {:?} }}",
            self.nr, self.boxed
        )
    }
}

impl TextLine {
    pub fn new(cols: usize) -> TextLine {
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

    // pub fn cache(&self) -> Option<&LayoutCache> {
    //     unsafe { &*self.cache.as_ptr() }.as_ref()
    // }

    // pub fn set_cache(&self, lc: LayoutCache) {
    //     self.cache.set(lc.into());
    // }

    pub fn set_nr(&mut self, nr: Nr) {
        self.nr = nr;
    }

    pub fn set_cells(&mut self, cells: Vec<TextCell>) {
        self.boxed = cells.into_boxed_slice();
    }

    pub fn take_cells(&mut self) -> Box<[TextCell]> {
        std::mem::take(&mut self.boxed)
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

impl From<TextLine> for Box<[TextCell]> {
    fn from(val: TextLine) -> Self {
        val.boxed
    }
}
