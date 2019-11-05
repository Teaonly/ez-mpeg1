// http://dvd.sourceforge.net/dvdinfo/mpeghdrs.html

use crate::bitbuf;

static MP1V_FRAME_RATE: [f32; 16] = [
    0.000, 23.976, 24.000, 25.000, 29.970, 30.000, 50.000, 59.940,
    60.000, 0.000, 0.000, 0.000, 0.000, 0.000, 0.000, 0.000
];

static MP1V_ZIG_ZAG: [u8; 64] = [
     0,  1,  8, 16,  9,  2,  3, 10,
    17, 24, 32, 25, 18, 11,  4,  5,
    12, 19, 26, 33, 40, 48, 41, 34,
    27, 20, 13,  6,  7, 14, 21, 28,
    35, 42, 49, 56, 57, 50, 43, 36,
    29, 22, 15, 23, 30, 37, 44, 51,
    58, 59, 52, 45, 38, 31, 39, 46,
    53, 60, 61, 54, 47, 55, 62, 63
];

static MP1V_INTRA_QUANT_MATRIX: [u8; 64] = [
     8, 16, 19, 22, 26, 27, 29, 34,
    16, 16, 22, 24, 27, 29, 34, 37,
    19, 22, 26, 27, 29, 34, 34, 38,
    22, 22, 26, 27, 29, 34, 37, 40,
    22, 26, 27, 29, 32, 35, 40, 48,
    26, 27, 29, 32, 35, 40, 48, 58,
    26, 27, 29, 34, 38, 46, 56, 69,
    27, 29, 35, 38, 46, 56, 69, 83
];

static MP1V_NON_INTRA_QUANT_MATRIX: [u8; 64] = [
    16, 16, 16, 16, 16, 16, 16, 16,
    16, 16, 16, 16, 16, 16, 16, 16,
    16, 16, 16, 16, 16, 16, 16, 16,
    16, 16, 16, 16, 16, 16, 16, 16,
    16, 16, 16, 16, 16, 16, 16, 16,
    16, 16, 16, 16, 16, 16, 16, 16,
    16, 16, 16, 16, 16, 16, 16, 16,
    16, 16, 16, 16, 16, 16, 16, 16
];


pub enum DecodeResult {
    GotOneFrame,
    NeedMoreData,
    InternalError,
}

#[derive(Default)]
struct CodecInfo {
    pub pic_width: u32,
    pub pic_height: u32,
    pub aspect_ratio: u32,
    pub frame_rate: f32,

    pub _parsed_:    bool,
}

struct QuantMatrix {
    pub intra_quant_matrix: [u8;64],
    pub non_intra_quant_matrix: [u8;64],
}

pub struct Mpeg1Video {
    info_:      CodecInfo,
    qmatrix_:   QuantMatrix,
    buffer_:    bitbuf::RingBitBuffer,
}

impl Mpeg1Video {
    const PICTURE_START_CODE: u32  = 0x00000100;
    const SEQUENCE_START_CODE: u32 = 0x000001B3;

    pub fn new() -> Self {
        let mut info:    CodecInfo = Default::default();
        info._parsed_ = false;

        let intra_quant_matrix:[u8; 64] = [0; 64];
        let non_intra_quant_matrix:[u8; 64] = [0; 64];
        let qm = QuantMatrix{ intra_quant_matrix, non_intra_quant_matrix};

        let buffer = bitbuf::RingBitBuffer::new();
        Mpeg1Video {
            info_:      info,
            qmatrix_:   qm,
            buffer_:    buffer,
        }
    }

    pub fn push(&mut self, data: &[u8]) -> Option<usize> {
        self.buffer_.push(data)
    }

    pub fn decode(&mut self) -> DecodeResult {
        if self.buffer_.include_two_code(Mpeg1Video::PICTURE_START_CODE) == false {
            return DecodeResult::NeedMoreData;
        }
        if self.info_._parsed_ == false {
            if self.buffer_.find_start_code(Mpeg1Video::SEQUENCE_START_CODE) == false {
                return DecodeResult::InternalError;
            }
            self.decode_sequence_header();
            if self.info_._parsed_ == false {
                return DecodeResult::InternalError;
            }
        }
        return self.decode_picture();
    }

    fn decode_picture(&mut self) -> DecodeResult {
        if self.buffer_.find_start_code(Mpeg1Video::PICTURE_START_CODE) == false {
            return DecodeResult::InternalError;
        }

        DecodeResult::InternalError
    }

    fn decode_sequence_header(&mut self) {
        if self.buffer_.has(8 * 8) == false {
            return;
        }
        self.info_.pic_width = self.buffer_.read(12);
        self.info_.pic_height = self.buffer_.read(12);
        self.info_.aspect_ratio = self.buffer_.read(4);
        self.info_.frame_rate = MP1V_FRAME_RATE[self.buffer_.read(4) as usize];

        //skip bitRate, marker, bufferSize and constrained bit
        self.buffer_.skip(18 + 1 + 10 + 1);

        //load intra quantiser matrix
        if self.buffer_.read(1) == 0x01 {
            if self.buffer_.has(64 * 8) == false {
                return;
            }
            for i in 0..64 {
                self.qmatrix_.intra_quant_matrix[i] = self.buffer_.read(8) as u8;
            }
        } else {
            for i in 0..64 {
                self.qmatrix_.intra_quant_matrix[i] = MP1V_INTRA_QUANT_MATRIX[i];
            }
        }

        //(load non-intra quantiser matrix)
        if self.buffer_.read(1) == 0x01 {
            if self.buffer_.has(64 * 8) == false {
                return;
            }
            for i in 0..64 {
                self.qmatrix_.non_intra_quant_matrix[i] = self.buffer_.read(8) as u8;
            }
        } else {
            for i in 0..64 {
                self.qmatrix_.non_intra_quant_matrix[i] = MP1V_NON_INTRA_QUANT_MATRIX[i];
            }
        }

        self.info_._parsed_ = true;
    }

}

