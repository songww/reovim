use relm4::factory::positions::FixedPosition;

#[derive(Clone, Copy, Debug, Default)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

impl Position {
    pub fn new(x: f64, y: f64) -> Position {
        Position { x, y }
    }
}

impl Into<FixedPosition> for Position {
    fn into(self) -> FixedPosition {
        FixedPosition {
            x: self.x,
            y: self.y,
        }
    }
}

impl From<(f64, f64)> for Position {
    fn from((x, y): (f64, f64)) -> Self {
        Position { x, y }
    }
}
