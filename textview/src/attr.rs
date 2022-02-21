bitflags::bitflags! {
    pub struct Attr: u32 {
        const NORMAL = 0b00000001;
        const BOLD   = 0b00000010;
        const ITALIC = 0b00000100;
        const BOLD_ITALIC = Self::BOLD.bits | Self::ITALIC.bits;
    }
}
