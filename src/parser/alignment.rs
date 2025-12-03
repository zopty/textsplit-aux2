#![allow(dead_code)]

#[derive(Debug)]
pub enum HDir {
    Left,
    Mid,
    Right,
}

#[derive(Debug)]
pub enum VDir {
    Top,
    Center,
    Bottom,
}

#[derive(Debug)]
pub struct TextAlignment {
    pub hdir: HDir,
    pub vdir: VDir,
    pub is_vert: bool,
}

pub fn parse_alignment(input: &str) -> TextAlignment {
    let h = if input.contains("左") {
        HDir::Left
    } else if input.contains("右") {
        HDir::Right
    } else {
        HDir::Mid
    };

    let v = if input.contains("上") {
        VDir::Top
    } else if input.contains("下") {
        VDir::Bottom
    } else {
        VDir::Center
    };

    let is_vert = input.contains("縦書");

    TextAlignment {
        hdir: h,
        vdir: v,
        is_vert,
    }
}
