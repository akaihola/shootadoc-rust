pub fn equalize(value: u8, black: u8, range: u8) -> u8 {
    255.min(255 * value.saturating_sub(black) as u32 / range as u32) as u8
}

pub fn brighter(a: u8, b: u8) -> u8 {
    a.max(b)
}

pub fn darker(a: u8, b: u8) -> u8 {
    a.min(b)
}
