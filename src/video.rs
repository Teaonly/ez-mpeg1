// http://dvd.sourceforge.net/dvdinfo/mpeghdrs.html

use crate::bitbuf;
use crate::vlc;

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

static MP1V_PREMULTIPLIER_MATRIX: [u8; 64] = [
    32, 44, 42, 38, 32, 25, 17,  9,
    44, 62, 58, 52, 44, 35, 24, 12,
    42, 58, 55, 49, 42, 33, 23, 12,
    38, 52, 49, 44, 38, 30, 20, 10,
    32, 44, 42, 38, 32, 25, 17,  9,
    25, 35, 33, 30, 25, 20, 14,  7,
    17, 24, 23, 20, 17, 14,  9,  5,
     9, 12, 12, 10,  9,  7,  5,  2
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

    pub mb_width: u32,
    pub mb_height: u32,
    pub mb_size: u32,

    pub luma_width: u32,
    pub luma_height: u32,

    pub chroma_width: u32,
    pub chroma_height: u32,

    pub _parsed_:    bool,
}

struct QuantMatrix {
    pub intra_quant_matrix: [u8;64],
    pub non_intra_quant_matrix: [u8;64],
}

#[derive(Default, Clone)]
pub struct VideoMotion {
    pub full_px: i32,
    pub is_set: i32,
    pub r_size: i32,
    pub h:  i32,
    pub v:  i32,
}

#[derive(Default)]
pub struct VideoRuntime {
    pub quantizer_scale:   u32,
    pub picture_type:      u32,

    pub dc_predictor_0:    i32,
    pub dc_predictor_1:    i32,
    pub dc_predictor_2:    i32,

    pub motion_forward:    VideoMotion,

    pub mb_row : i32,
    pub mb_col : i32,
    pub macroblock_pattern:  i32,
    pub macroblock_intra:   i32,
    pub macroblock_type:    i32,
    pub macroblock_address: i32,
}

pub struct Mpeg1Video {
    buffer_:    bitbuf::RingBitBuffer,

    info_:      CodecInfo,
    qmatrix_:   QuantMatrix,
    runtime_:   VideoRuntime,
}

impl Mpeg1Video {
    const PICTURE_START_CODE: u32  = 0x00000100;
    const SEQUENCE_START_CODE: u32 = 0x000001B3;
    const START_EXTENSION_CODE: u32 = 0xB5;
    const USER_DATA_CODE: u32 = 0xB2;
    const SLICE_START: u32 = 0x01;
    const SLICE_LAST: u32 = 0xAF;

    const PICTURE_TYPE_I: u32 = 0x01;
    const PICTURE_TYPE_P: u32 = 0x02;

    pub fn new() -> Self {
        let mut info:    CodecInfo = Default::default();
        info._parsed_ = false;

        let intra_quant_matrix:[u8; 64] = [0; 64];
        let non_intra_quant_matrix:[u8; 64] = [0; 64];
        let qm = QuantMatrix{ intra_quant_matrix, non_intra_quant_matrix};
        let runtime: VideoRuntime = Default::default();

        let buffer = bitbuf::RingBitBuffer::new();
        Mpeg1Video {
            info_:      info,
            qmatrix_:   qm,
            runtime_:   runtime,
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

        self.info_.mb_width = (self.info_.pic_width + 15) >> 4;
        self.info_.mb_height = (self.info_.pic_height + 15) >> 4;
        self.info_.luma_width = self.info_.mb_width << 4;
        self.info_.luma_height = self.info_.mb_height << 4;
        self.info_.chroma_width = self.info_.mb_width << 3;
        self.info_.chroma_height = self.info_.mb_height << 3;

        // TODO: init framebuffer
        /*
        // Allocate one big chunk of data for all 3 frames = 9 planes
        size_t luma_plane_size = self->luma_width * self->luma_height;
        size_t chroma_plane_size = self->chroma_width * self->chroma_height;
        size_t frame_data_size = (luma_plane_size + 2 * chroma_plane_size);

        self->frames_data = (uint8_t*)malloc(frame_data_size * 3);
        plm_video_init_frame(self, &self->frame_current, self->frames_data + frame_data_size * 0);
        plm_video_init_frame(self, &self->frame_forward, self->frames_data + frame_data_size * 1);
        plm_video_init_frame(self, &self->frame_backward, self->frames_data + frame_data_size * 2);
        */

        self.info_._parsed_ = true;
    }

    fn decode_picture(&mut self) -> DecodeResult {
        if self.buffer_.find_start_code(Mpeg1Video::PICTURE_START_CODE) == false {
            panic!("##########");
            return DecodeResult::InternalError;
        }

        // get current picture type
        self.buffer_.skip(10); // skip temporalReference
        self.runtime_.picture_type = self.buffer_.read(3);
        self.buffer_.skip(16); // skip vbv_delay

        if self.runtime_.picture_type != Mpeg1Video::PICTURE_TYPE_I &&
            self.runtime_.picture_type != Mpeg1Video::PICTURE_TYPE_P {
            panic!("##########");
            return DecodeResult::InternalError;
        }

        // forward full_px, f_code
        if self.runtime_.picture_type  == Mpeg1Video::PICTURE_TYPE_P {
            self.runtime_.motion_forward.full_px = self.buffer_.read(1) as i32;

            let f_code: i32 = self.buffer_.read(3) as i32;
            if f_code == 0x00 {
                panic!("##########");
                return DecodeResult::InternalError;
            }

            self.runtime_.motion_forward.r_size = f_code - 1;
        }

        loop {
            // skip user data and extension
            if self.buffer_.find_start() == false {
                return DecodeResult::InternalError;
            }
            let code = self.buffer_.read(8);
            if code == Mpeg1Video::USER_DATA_CODE || code == Mpeg1Video::START_EXTENSION_CODE {
                continue;
            }
            if code == Mpeg1Video::SLICE_START {
                break;
            }
            panic!("#### EXIT FOR DEBUG ###");
            return DecodeResult::InternalError;
        }

        // TODO
        /*
        plm_frame_t frame_temp = self->frame_forward;

        if (
            self->picture_type == PLM_VIDEO_PICTURE_TYPE_INTRA ||
            self->picture_type == PLM_VIDEO_PICTURE_TYPE_PREDICTIVE
        ) {
            self->frame_forward = self->frame_backward;
        }
        */

        let mut next_code = Mpeg1Video::SLICE_START;
        while next_code >= Mpeg1Video::SLICE_START && next_code <= Mpeg1Video::SLICE_LAST {
            self.decode_slice(next_code);

            if self.buffer_.find_start() == true {
                next_code = self.buffer_.read(8);
            } else {
                panic!("#### EXIT FOR DEBUG ###");
            }
        }

        self.buffer_.back(32);

        // TODO
        /*
        plm_frame_t frame_temp = self->frame_forward;

        if (
            self->picture_type == PLM_VIDEO_PICTURE_TYPE_INTRA ||
            self->picture_type == PLM_VIDEO_PICTURE_TYPE_PREDICTIVE
        ) {
            self->frame_forward = self->frame_backward;
        }
        */


        return DecodeResult::GotOneFrame;
    }

    fn decode_slice(&mut self, slice_code: u32) {
        self.runtime_.macroblock_address = ((slice_code - 1) * self.info_.mb_width) as i32 - 1;

        // Reset motion vectors and DC predictors
        self.runtime_.motion_forward.h = 0;
        self.runtime_.motion_forward.v = 0;
        self.runtime_.dc_predictor_0 = 128;
        self.runtime_.dc_predictor_1 = 128;
        self.runtime_.dc_predictor_2 = 128;

        // quantizer scale
        self.runtime_.quantizer_scale = self.buffer_.read(5);

        // skip extra
        while self.buffer_.read(1) != 0x00 {
            self.buffer_.read(8);
        }

        let mut slice_begin = true;
        loop {
            self.decode_macroblock(slice_begin);
            slice_begin = false;

            if  (self.runtime_.macroblock_address >= self.info_.mb_size as i32 - 1)
                || self.buffer_.next_is_start() {
                break;
            }
        }
    }

    fn decode_macroblock(&mut self, slice_begin:bool) {
        // Decode self->macroblock_address_increment
        let mut increment:i16 = 0;

        let mut t = self.buffer_.read_vlc(&vlc::MP1V_MACROBLOCK_ADDRESS_INCREMENT);
        while t == 34 {
            // macroblock_stuffing
            t = self.buffer_.read_vlc(&vlc::MP1V_MACROBLOCK_ADDRESS_INCREMENT);
        }
        while t == 35 {
            increment += 33;
            t = self.buffer_.read_vlc(&vlc::MP1V_MACROBLOCK_ADDRESS_INCREMENT);
        }
        increment += t;

        if slice_begin {
            // The first self->macroblock_address_increment of each slice is relative
            // to beginning of the preverious row, not the preverious macroblock
            self.runtime_.macroblock_address += increment as i32;
        } else {
            if self.runtime_.macroblock_address + increment as i32 >= self.info_.mb_size as i32 {
                panic!("macroblock skip out of picture size");
                return; // invalid
            }

            if increment > 1 {
                // Skipped macroblocks reset DC predictors
                self.runtime_.dc_predictor_0 = 128;
                self.runtime_.dc_predictor_1 = 128;
                self.runtime_.dc_predictor_2 = 128;

                // Skipped macroblocks in P-pictures reset motion vectors
                if self.runtime_.picture_type == Mpeg1Video::PICTURE_TYPE_P {
                    self.runtime_.motion_forward.h = 0;
                    self.runtime_.motion_forward.v = 0;
                }
            }

            // Predict skipped macroblocks
            while increment > 1 {
                self.runtime_.macroblock_address += 1;
                self.runtime_.mb_row = self.runtime_.macroblock_address / self.info_.mb_width as i32;
                self.runtime_.mb_col = self.runtime_.macroblock_address % self.info_.mb_width as i32;

                self.predict_macroblock();
                increment -= 1;
            }
            self.runtime_.macroblock_address += 1;
        }

        self.runtime_.mb_row = self.runtime_.macroblock_address / self.info_.mb_width as i32;
        self.runtime_.mb_col = self.runtime_.macroblock_address % self.info_.mb_width as i32;

        if self.runtime_.mb_col >= self.info_.mb_width as i32
           || self.runtime_.mb_row >= self.info_.mb_height as i32 {
            panic!("macroblock skip out of picture size");
            return; // corrupt stream;
        }

        // Process the current macroblock
        if self.runtime_.picture_type == Mpeg1Video::PICTURE_TYPE_I {
            self.runtime_.macroblock_type = self.buffer_.read_vlc(&vlc::MP1V_MACROBLOCK_TYPE_INTRA) as i32;
        } else if self.runtime_.picture_type == Mpeg1Video::PICTURE_TYPE_P {
            self.runtime_.macroblock_type = self.buffer_.read_vlc(&vlc::MP1V_MACROBLOCK_TYPE_PREDICTIVE) as i32;
        } else {
            panic!("Dont' support B/D picture type");
        }

        self.runtime_.macroblock_intra = (self.runtime_.macroblock_type & 0x01);
        self.runtime_.macroblock_pattern = (self.runtime_.macroblock_type & 0x02);
        self.runtime_.motion_forward.is_set = (self.runtime_.macroblock_type & 0x08);

        // Quantizer scale
        if ((self.runtime_.macroblock_type & 0x10) != 0) {
            self.runtime_.quantizer_scale = self.buffer_.read(5);
        }

        if (self->macroblock_intra) {
            // Intra-coded macroblocks reset motion vectors
            self.runtime_.motion_forward.h = 0;
            self.runtime_.motion_forward.v = 0;

        } else {
            // Non-intra macroblocks reset DC predictors
            self.runtime_.dc_predictor_0 = 128;
            self.runtime_.dc_predictor_1 = 128;
            self.runtime_.dc_predictor_2 = 128;

            plm_video_decode_motion_vectors(self);
            plm_video_predict_macroblock(self);
        }

        // Decode blocks
        let cbp:u32 =
            if self.runtime_.macroblock_pattern != 0 {
                self.buffer_.read_vlc(&vlc::CODE_BLOCK_PATTERN) as u32
            } else {
                if self.runtime_.macroblock_intra != 0x00 {
                    0x3f
                } else {
                    0x00
                }
            };

        for (int block = 0, mask = 0x20; block < 6; block++) {
            if ((cbp & mask) != 0) {
                plm_video_decode_block(self, block);
            }
            mask >>= 1;
        }

    }

    fn predict_macroblock(&mut self) {

    }

    fn decode_motion_vectors(&mut self) {

    }

    fn predict_macroblock(&mut self) {

    }

}

