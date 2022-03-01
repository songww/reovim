use relm4::factory::positions::FixedPosition;

#[derive(Debug, Clone)]
pub struct Coord {
    pub col: f64,
    pub row: f64,
}

impl From<(usize, usize)> for Coord {
    fn from((col, row): (usize, usize)) -> Self {
        Coord {
            col: col as f64,
            row: row as f64,
        }
    }
}

impl From<(f64, f64)> for Coord {
    fn from((col, row): (f64, f64)) -> Self {
        Coord { col, row }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Pos {
    pub x: f64,
    pub y: f64,
}

impl Pos {
    pub fn new(x: f64, y: f64) -> Pos {
        Pos { x, y }
    }
}

impl Into<FixedPosition> for Pos {
    fn into(self) -> FixedPosition {
        FixedPosition {
            x: self.x,
            y: self.y,
        }
    }
}

impl From<(f64, f64)> for Pos {
    fn from((x, y): (f64, f64)) -> Self {
        Pos { x, y }
    }
}

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
