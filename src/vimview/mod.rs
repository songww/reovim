//mod commandview;
mod gridview;
mod highlights;
mod messageview;
mod textbuf;
mod widgets;

use std::ops::{Deref, DerefMut};
use std::sync::Arc;

pub use gridview::VimGridView;
pub use highlights::HighlightDefinitions;
pub use messageview::{MessageViewWidgets, VimMessage, VimMessageView};
pub use widgets::{VimGrid, VimGridWidgets};

#[derive(Clone, Debug)]
pub struct TextBuf(Arc<textbuf::TextBuf>);

unsafe impl Sync for TextBuf {}
unsafe impl Send for TextBuf {}

impl TextBuf {
    pub fn new(rows: usize, cols: usize) -> Self {
        let buf = TextBuf(Arc::new(textbuf::TextBuf::new(cols, rows)));
        buf
    }
}

impl Deref for TextBuf {
    type Target = Arc<textbuf::TextBuf>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for TextBuf {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl AsRef<textbuf::TextBuf> for TextBuf {
    fn as_ref(&self) -> &textbuf::TextBuf {
        &self.0
    }
}

impl AsMut<Arc<textbuf::TextBuf>> for TextBuf {
    fn as_mut(&mut self) -> &mut Arc<textbuf::TextBuf> {
        &mut self.0
    }
}
