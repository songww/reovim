//mod commandview;
mod gridview;
mod highlights;
mod messageview;
mod textbuf;
mod widgets;

use std::{
    cell::RefCell,
    ops::{Deref, DerefMut},
    rc::Rc,
};

pub use gridview::VimGridView;
pub use highlights::HighlightDefinitions;
pub use messageview::{MessageViewWidgets, VimMessage, VimMessageView};
pub use textbuf::{TextCell, TextLine};
pub use widgets::{VimGrid, VimGridWidgets};

#[derive(Clone, Debug)]
pub struct TextBuf(Rc<RefCell<textbuf::TextBuf>>);

impl TextBuf {
    pub fn new(rows: usize, cols: usize) -> Self {
        let buf = Self::default();
        buf.0.borrow_mut().resize(rows, cols);
        buf
    }
}

impl Default for TextBuf {
    fn default() -> Self {
        Self(Rc::new(RefCell::new(textbuf::TextBuf::new())))
    }
}

impl Deref for TextBuf {
    type Target = Rc<RefCell<textbuf::TextBuf>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for TextBuf {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl AsRef<Rc<RefCell<textbuf::TextBuf>>> for TextBuf {
    fn as_ref(&self) -> &Rc<RefCell<textbuf::TextBuf>> {
        &self.0
    }
}

impl AsMut<Rc<RefCell<textbuf::TextBuf>>> for TextBuf {
    fn as_mut(&mut self) -> &mut Rc<RefCell<textbuf::TextBuf>> {
        &mut self.0
    }
}
