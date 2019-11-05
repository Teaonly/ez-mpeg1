// http://dvd.sourceforge.net/dvdinfo/mpeghdrs.html

use crate::bitbuf;

#[derive(Default)]
struct PictureInfo {
    pub pic_width: u32,
    pub pic_height: u32,
    pub frame_rate: f32,
}

#[derive(Default)]
pub struct Mpeg1Video {
    seq_info:   PictureInfo,
    has_seq:    bool,
}

impl Mpeg1Video {
    pub fn new() -> Self {
        let mut v:Mpeg1Video = Default::default();
        v.has_seq = false;
        v
    }
}

