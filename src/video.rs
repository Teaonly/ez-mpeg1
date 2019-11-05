// http://dvd.sourceforge.net/dvdinfo/mpeghdrs.html

use crate::bitbuf;

pub enum DecodeResult {
    GotOneFrame,
    NeedMoreData,
    InternalError,
}

#[derive(Default)]
struct SequenceInfo {
    pub pic_width: u32,
    pub pic_height: u32,
    pub frame_rate: f32,
    pub _parsed_:    bool,
}

pub struct Mpeg1Video {
    seq_:       SequenceInfo,
    buffer_:    bitbuf::RingBitBuffer,
}

impl Mpeg1Video {
    const PICTURE_START_CODE: u32  = 0x00000100;
    const SEQUENCE_START_CODE: u32 = 0x000001B3;

    pub fn new() -> Self {
        let mut seq:    SequenceInfo = Default::default();
        seq._parsed_ = false;

        let buffer = bitbuf::RingBitBuffer::new();
        Mpeg1Video {
            seq_:       seq,
            buffer_:    buffer,
        }
    }

    pub fn push(&mut self, data: &[u8]) -> Option<usize> {
        self.buffer_.push(data)
    }

    pub fn include_one_frame(&mut self) -> bool {
        self.buffer_.include_two_code(Mpeg1Video::PICTURE_START_CODE)
    }

    pub fn decode(&mut self) -> DecodeResult {
        if self.seq_._parsed_ == false {
            if self.buffer_.find_start_code(Mpeg1Video::SEQUENCE_START_CODE) == false {
                return DecodeResult::NeedMoreData;
            }
            self.decode_sequence_header();
            if self.seq_._parsed_ == false {
                return DecodeResult::NeedMoreData;
            }
        }
        return DecodeResult::NeedMoreData;
    }

    fn decode_sequence_header(&mut self) {
        if self.buffer_.has(24) == false {
            return;
        }
        self.seq_.pic_width = self.buffer_.read(12);
        self.seq_.pic_height = self.buffer_.read(12);
        println!("============{}|{}", self.seq_.pic_width, self.seq_.pic_height);
        panic!("EXIT FOR DEBUG");
    }

}

