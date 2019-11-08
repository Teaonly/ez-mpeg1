// http://dvd.sourceforge.net/dvdinfo/mpeghdrs.html

use crate::bitbuf;
use crate::vlc;

static MP1V_FRAME_RATE: [f32; 16] = [
    0.000, 23.976, 24.000, 25.000, 29.970, 30.000, 50.000, 59.940,
    60.000, 0.000, 0.000, 0.000, 0.000, 0.000, 0.000, 0.000
];

const MP1V_ZIG_ZAG: [i32; 64] = [
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
    pub frame_current:     i32,
    pub frame_forward:     i32,

    pub quantizer_scale:   u32,
    pub picture_type:      u32,

    pub dc_predictor:      [i32;3],

    pub motion_forward:    VideoMotion,

    pub mb_row : u32,
    pub mb_col : u32,
    pub macroblock_pattern:  i32,
    pub macroblock_intra:   i32,
    pub macroblock_type:    i32,
    pub macroblock_address: i32,
}

#[derive(Default)]
pub struct VideoPlane {
    pub base: usize,
    pub width: u32,
    pub height: u32,
}

#[derive(Default)]
pub struct VideoFrame {
    pub time: f64,
    pub width: u32,
    pub height: u32,
    pub y:      VideoPlane,
    pub cb:     VideoPlane,
    pub cr:     VideoPlane,
}

pub struct Mpeg1Video {
    buffer_:      bitbuf::RingBitBuffer,

    frame_base_:  Box<[u8]>,
    frames_:      Vec<VideoFrame>,
    block_data_:  [i32; 64],

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

    const MAX_PICTURE_WIDTH: usize = 1920;
    const MAX_PICTURE_HEIGHT: usize = 1080;

    const DCT_SIZE_TABLE: [&'static [(i16, i16)];3]  = [&vlc::MP1V_DCT_SIZE_LUMINANCE,
                                                &vlc::MP1V_DCT_SIZE_CHROMINANCE,
                                                &vlc::MP1V_DCT_SIZE_CHROMINANCE];

    pub fn new() -> Self {
        let mut info:    CodecInfo = Default::default();
        info._parsed_ = false;

        let intra_quant_matrix:[u8; 64] = [0; 64];
        let non_intra_quant_matrix:[u8; 64] = [0; 64];
        let qm = QuantMatrix{ intra_quant_matrix, non_intra_quant_matrix};

        let runtime: VideoRuntime = Default::default();

        let buffer = bitbuf::RingBitBuffer::new();
        let mut fbase: Vec<u8> = Vec::new();
        let fbase_size = Mpeg1Video::MAX_PICTURE_WIDTH * Mpeg1Video::MAX_PICTURE_HEIGHT * 6;
        fbase.resize(fbase_size , 0);
        fbase.reserve_exact(0);

        let frame_current: VideoFrame = Default::default();
        let frame_forward: VideoFrame = Default::default();

        Mpeg1Video {
            info_:          info,
            qmatrix_:       qm,
            runtime_:       runtime,
            buffer_:        buffer,
            frame_base_:    fbase.into_boxed_slice(),
            frames_:        vec![frame_current, frame_forward],
            block_data_:    [0; 64]
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
        self.info_._parsed_ = true;

        self.init_frames();
        self.runtime_.frame_current = 0;
        self.runtime_.frame_forward = 0;
    }

    fn init_frames(&mut self) {
        // Allocate one big chunk of data for all 3 frames = 9 planes
        let luma_plane_size: u32 = self.info_.luma_width * self.info_.luma_height;
        let chroma_plane_size: u32 = self.info_.chroma_width * self.info_.chroma_height;
        let frame_data_size: u32 = luma_plane_size + 2 * chroma_plane_size;

        for i in  0..self.frames_.len() {
            let frame = &mut self.frames_[i];
            let base = i * frame_data_size as usize;

            frame.width = self.info_.pic_width;
            frame.height = self.info_.pic_height;

            frame.y.width = self.info_.luma_width;
            frame.y.height = self.info_.luma_height;
            frame.y.base = base + 0;

            frame.cr.width = self.info_.chroma_width;
            frame.cr.height = self.info_.chroma_height;
            frame.cr.base = base + luma_plane_size as usize;

            frame.cb.width = self.info_.chroma_width;
            frame.cb.height = self.info_.chroma_height;
            frame.cb.base = base + luma_plane_size as usize + chroma_plane_size as usize;
        }
    }

    fn decode_picture(&mut self) -> DecodeResult {
        if self.buffer_.find_start_code(Mpeg1Video::PICTURE_START_CODE) == false {
            return DecodeResult::InternalError;
        }

        // get current picture type
        self.buffer_.skip(10); // skip temporalReference
        self.runtime_.picture_type = self.buffer_.read(3);
        self.buffer_.skip(16); // skip vbv_delay

        if self.runtime_.picture_type != Mpeg1Video::PICTURE_TYPE_I &&
            self.runtime_.picture_type != Mpeg1Video::PICTURE_TYPE_P {
            return DecodeResult::InternalError;
        }

        // forward full_px, f_code
        if self.runtime_.picture_type  == Mpeg1Video::PICTURE_TYPE_P {
            self.runtime_.motion_forward.full_px = self.buffer_.read(1) as i32;

            let f_code: i32 = self.buffer_.read(3) as i32;
            if f_code == 0x00 {
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
            return DecodeResult::InternalError;
        }

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

        self.runtime_.frame_forward = self.runtime_.frame_current;
        self.runtime_.frame_current = 1 - self.runtime_.frame_current;

        return DecodeResult::GotOneFrame;
    }

    fn decode_slice(&mut self, slice_code: u32) {
        self.runtime_.macroblock_address = ((slice_code - 1) * self.info_.mb_width) as i32 - 1;

        // Reset motion vectors and DC predictors
        self.runtime_.motion_forward.h = 0;
        self.runtime_.motion_forward.v = 0;
        self.runtime_.dc_predictor[0] = 128;
        self.runtime_.dc_predictor[1] = 128;
        self.runtime_.dc_predictor[2] = 128;

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
                return; // invalid
            }

            if increment > 1 {
                // Skipped macroblocks reset DC predictors
                self.runtime_.dc_predictor[0] = 128;
                self.runtime_.dc_predictor[1] = 128;
                self.runtime_.dc_predictor[2] = 128;

                // Skipped macroblocks in P-pictures reset motion vectors
                if self.runtime_.picture_type == Mpeg1Video::PICTURE_TYPE_P {
                    self.runtime_.motion_forward.h = 0;
                    self.runtime_.motion_forward.v = 0;
                }
            }

            // Predict skipped macroblocks
            while increment > 1 {
                self.runtime_.macroblock_address += 1;
                self.runtime_.mb_row = self.runtime_.macroblock_address as u32 / self.info_.mb_width;
                self.runtime_.mb_col = self.runtime_.macroblock_address as u32 % self.info_.mb_width;

                self.predict_macroblock();
                increment -= 1;
            }
            self.runtime_.macroblock_address += 1;
        }

        self.runtime_.mb_row = self.runtime_.macroblock_address as u32 / self.info_.mb_width;
        self.runtime_.mb_col = self.runtime_.macroblock_address as u32 % self.info_.mb_width;

        if self.runtime_.mb_col >= self.info_.mb_width
           || self.runtime_.mb_row >= self.info_.mb_height {
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

        self.runtime_.macroblock_intra = self.runtime_.macroblock_type & 0x01;
        self.runtime_.macroblock_pattern = self.runtime_.macroblock_type & 0x02;
        self.runtime_.motion_forward.is_set = self.runtime_.macroblock_type & 0x08;

        // Quantizer scale
        if (self.runtime_.macroblock_type & 0x10) != 0 {
            self.runtime_.quantizer_scale = self.buffer_.read(5);
        }

        if self.runtime_.macroblock_intra != 0 {
            // Intra-coded macroblocks reset motion vectors
            self.runtime_.motion_forward.h = 0;
            self.runtime_.motion_forward.v = 0;

        } else {
            // Non-intra macroblocks reset DC predictors
            self.runtime_.dc_predictor[0] = 128;
            self.runtime_.dc_predictor[1] = 128;
            self.runtime_.dc_predictor[2] = 128;

            self.decode_motion_vectors();
            self.predict_macroblock();
        }

        // Decode blocks
        let cbp:u32 =
            if self.runtime_.macroblock_pattern != 0 {
                self.buffer_.read_vlc(&vlc::MP1V_CODE_BLOCK_PATTERN) as u32
            } else {
                if self.runtime_.macroblock_intra != 0 {
                    0x3f
                } else {
                    0x00
                }
            };

        let mut mask:u32 = 0x20;
        for block in 0..6 {
            if (cbp & mask) != 0 {
                self.decode_block(block as i32);
            }
            mask >>= 1;
        }
    }

    fn predict_macroblock(&mut self) {
        let mut fw_h = self.runtime_.motion_forward.h;
        let mut fw_v = self.runtime_.motion_forward.v;

        if self.runtime_.motion_forward.full_px == 1 {
            fw_h <<= 1;
            fw_v <<= 1;
        }

        let dst = &self.frames_[self.runtime_.frame_current as usize];
        let src = &self.frames_[self.runtime_.frame_forward as usize];

        self.process_macroblock(src.y.base, dst.y.base, fw_h, fw_v, 16);
        self.process_macroblock(src.cr.base, dst.cr.base, fw_h/2, fw_v/2, 8);
        self.process_macroblock(src.cb.base, dst.cb.base, fw_h/2, fw_v/2, 8);
    }

    // copy from source to dest with motion vector
    fn process_macroblock(&self, src: usize, dst:  usize,
                          mv_h: i32, mv_v: i32, block_size: u32) {

        let dw = block_size * self.info_.mb_width;

        let hp = mv_h >> 1;
        let vp = mv_v >> 1;
        let odd_h = (mv_h & 1) == 1;
        let odd_v = (mv_v & 1) == 1;

        let di:u32 = (self.runtime_.mb_row * dw + self.runtime_.mb_col) * block_size;
        let si:i32 = ((self.runtime_.mb_row * block_size) as i32 + vp) * dw as i32 + (self.runtime_.mb_col * block_size) as i32 + hp;
        let si:u32 = si as u32;

        let max_address = dw * (self.info_.mb_height * block_size - block_size + 1) - block_size;
        if si > max_address || di > max_address {
            panic!("motion vector outof picture");
            return; // corrupt video
        }

        //TODO
    }

    fn decode_motion_vectors(&mut self) {
        // Forward
        if self.runtime_.motion_forward.is_set == 0x01 {
            let r_size = self.runtime_.motion_forward.r_size;
            self.runtime_.motion_forward.h = self.decode_motion_vector(r_size, self.runtime_.motion_forward.h);
            self.runtime_.motion_forward.v = self.decode_motion_vector(r_size, self.runtime_.motion_forward.v);
        } else if self.runtime_.picture_type == Mpeg1Video::PICTURE_TYPE_P {
            // No motion information in P-picture, reset vectors
            self.runtime_.motion_forward.h = 0;
            self.runtime_.motion_forward.v = 0;
        }
    }

    fn decode_motion_vector(&mut self, r_size:i32, mut motion: i32) -> i32 {
        let fscale = 1 << r_size;
        let m_code = self.buffer_.read_vlc(&vlc::MP1V_VIDEO_MOTION) as i32;
        let r:i32;
        let mut d:i32;

        if (m_code != 0) && (fscale != 1) {
            r = self.buffer_.read(r_size as usize) as i32;
            d = ((m_code.abs() - 1) << r_size) + r + 1;
            if m_code < 0 {
                d = -d;
            }
        }
        else {
            d = m_code;
        }

        motion += d;
        if motion >(fscale << 4) - 1 {
            motion -= fscale << 5;
        } else if motion < ((-fscale) << 4) {
            motion += fscale << 5;
        }
        return motion;
    }

    fn decode_block(&mut self, block: i32) {
        let mut n:i32 = 0;
        let quant_matrix: &[u8];

        // Decode DC coefficient of intra-coded blocks
        if self.runtime_.macroblock_intra != 0 {
            // DC prediction
            let plane_index = if block > 3 {
                block - 3
            } else {
                0
            };

            let dct_size = self.buffer_.read_vlc(Mpeg1Video::DCT_SIZE_TABLE[plane_index as usize]);
            let predictor = self.runtime_.dc_predictor[plane_index as usize];

            // Read DC coeff
            if dct_size > 0 {
                let differential:i32  = self.buffer_.read(dct_size as usize) as i32;

                if (differential & (1 << (dct_size - 1))) != 0 {
                    self.block_data_[0] = predictor + differential;
                }
                else {
                    self.block_data_[0] = predictor + ((-1 << dct_size) | (differential + 1));
                }
            } else {
                self.block_data_[0] = predictor;
            }

            // Save predictor value
            self.runtime_.dc_predictor[plane_index as usize] = self.block_data_[0];

            // Dequantize + premultiply
            self.block_data_[0] <<= 3 + 5;

            quant_matrix = &self.qmatrix_.intra_quant_matrix;
            n = 1;
        } else {
            quant_matrix = &self.qmatrix_.non_intra_quant_matrix;
        }

        // Decode AC coefficients (+DC for non-intra)
        let mut level:i32 = 0;
        loop {
            let run:i32;
            let coeff:u16 = self.buffer_.read_vlc_u16(&vlc::MP1V_DCT_COEFF);

            if (coeff == 0x0001) && (n > 0) && (self.buffer_.read(1) == 0) {
                // end_of_block
                break;
            }

            if coeff == 0xffff {
                // escape
                run = self.buffer_.read(6) as i32;
                level = self.buffer_.read(8) as i32;
                if level == 0 {
                    level = self.buffer_.read(8) as i32;
                } else if level == 128 {
                    level = self.buffer_.read(8) as i32 - 256;
                } else if level > 128 {
                    level = level - 256;
                }
            } else {
                run = (coeff >> 8) as i32;
                level = (coeff & 0xff) as i32;
                if self.buffer_.read(1) != 0 {
                    level = -level;
                }
            }

            n += run;
            if n < 0 || n >= 64 {
                return; // invalid
            }

            let de_zig_zagged = MP1V_ZIG_ZAG[n as usize];
            n+=1;

            // Dequantize, oddify, clip
            level <<= 1;
            if (self.runtime_.macroblock_intra == 0) {
                level += if level < 0 { -1 } else { 1};
            }
            level = (level * self.runtime_.quantizer_scale as i32 * quant_matrix[de_zig_zagged as usize] as i32) >> 4;
            if (level & 1) == 0 {
                level -= if level > 0 { 1 } else { -1 };
            }
            if level > 2047 {
                level = 2047;
            } else if level < -2048 {
                level = -2048;
            }

            // Save premultiplied coefficient
            self.block_data_[de_zig_zagged as usize] = level * MP1V_PREMULTIPLIER_MATRIX[de_zig_zagged as usize] as i32;
        }


    }
}

