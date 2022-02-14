#[derive(Clone, Copy, Debug, Default)]
pub struct Rectangle {
    pub width: usize,
    pub height: usize,
}

impl Rectangle {
    fn new(width: usize, height: usize) -> Rectangle {
        Rectangle { width, height }
    }
}

impl From<(usize, usize)> for Rectangle {
    fn from((width, height): (usize, usize)) -> Self {
        Rectangle { width, height }
    }
}

impl From<(u64, u64)> for Rectangle {
    fn from((width, height): (u64, u64)) -> Self {
        Rectangle {
            width: width as usize,
            height: height as usize,
        }
    }
}
