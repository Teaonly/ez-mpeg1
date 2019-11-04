// https://en.wikipedia.org/wiki/MPEG_program_stream
// https://en.wikipedia.org/wiki/Packetized_elementary_stream

use std::ptr;
use crate::bitbuf;

#[allow(non_camel_case_types)]
#[derive(Debug)]
pub enum PacketError {
    OUT_LENGTH(usize),
    NO_START_CODE,
    CODE_NOT_MATCH,
    FORMAT_ERROR,
}

#[allow(non_camel_case_types)]
#[derive(Debug)]
pub enum PacketType {
    PS_PACK_HEADER,
    PS_SYSTEM_HEADER,
    PES_AUDIO,
    PES_VIDEO,
    PES_UNKNOW,
}

pub struct MpegPS {
    pub system_clock_ref: u64,
    pub bit_rate:         u64,
    pub num_audio_streams: i32,
    pub num_video_streams: i32,

    pub has_pack_header: bool,
    pub has_system_header: bool,

    buffer_: Vec<u8>,
    offset_: usize,
    len_: usize,
}

#[derive(Debug)]
pub struct PESPacketInfo {
    pub pkt_type: PacketType,
    pub code: u32,
    pub offset: usize,
    pub len: usize,
    pub pts: u64,
}

impl PESPacketInfo {
    pub fn pos(&self) ->usize{
        self.offset + self.len
    }
}

impl MpegPS {
    const PACK_HEADER_CODE: u32 = 0x000001BA;
    const SYSTEM_HEADER_CODE: u32 = 0x000001BB;
    const PES_VIDEO_BEGIN: u32 = 0x000001E0;
    const PES_VIDEO_END: u32 = 0x000001EF;
    const PES_AUDIO_BEGIN: u32 = 0x000001C0;
    const PES_AUDIO_END: u32 = 0x000001DF;
    const PES_PRIVATE_CODE: u32 = 0x000001BD;

    fn code2type(code:u32) -> PacketType {
        if code >= MpegPS::PES_AUDIO_BEGIN &&
           code <= MpegPS::PES_AUDIO_END {
            return PacketType::PES_AUDIO;
        }

        if code >= MpegPS::PES_VIDEO_BEGIN &&
           code <= MpegPS::PES_VIDEO_END {
            return PacketType::PES_VIDEO;
        }

        return PacketType::PES_UNKNOW;
    }

    pub fn new() -> MpegPS {
        let mut buffer:Vec<u8> = Vec::new();
        buffer.resize(1024*1024*4, 0);
        MpegPS {
            has_pack_header:    false,
            has_system_header:  false,
            system_clock_ref:   0x00,
            bit_rate:           0,
            num_audio_streams:  -1,
            num_video_streams:  -1,

            buffer_:    buffer,
            offset_:    0,
            len_:       0,
        }
    }

    pub fn push(&mut self, data: &[u8]) -> usize {
        let len = data.len();

        let mut copy_len = self.buffer_.len() - self.len_;
        if copy_len > data.len() {
            copy_len = len;
        }
        unsafe {
            let dst: *mut u8 = self.buffer_.as_ptr().add(self.len_) as *mut u8;
            let src: *const u8 = data.as_ptr();
            ptr::copy(src, dst, copy_len);
        }
        self.len_ = self.len_ + copy_len;

        return copy_len;
    }

    pub fn get(&mut self) -> Result<PESPacketInfo, PacketError> {
        let data = &self.buffer_[self.offset_..self.len_];
        let pkt_result = self.get_packet( data );

        if let Ok(ref pkt) = pkt_result {
            self.offset_ = self.offset_ + pkt.pos();
        }
        if let Err(ref e) = pkt_result {
            if let PacketError::OUT_LENGTH(more) = e {
                let push_size = more + 1280;
                let remain = self.buffer_.len() - self.len_;
                if push_size > remain {
                    unsafe {
                        let dst: *mut u8 = self.buffer_.as_ptr() as *mut u8;
                        let src: *const u8 = self.buffer_.as_ptr().add(self.offset_);
                        ptr::copy(src, dst, self.len_ - self.offset_);
                    }
                    self.len_ = self.len_ - self.offset_;
                    self.offset_ = 0;
                }
            }
        }
        pkt_result
    }

    fn get_packet(&mut self, data: &[u8]) -> Result<PESPacketInfo, PacketError> {
        if !self.has_pack_header {
            if let Some(pos) = MpegPS::find_start_code(data, MpegPS::PACK_HEADER_CODE) {
                return self.get_pack_header_packet(data, pos);
            } else {
                return Err(PacketError::NO_START_CODE);
            }
        }

        if !self.has_system_header {
            if let Some(pos) = MpegPS::find_start_code(data, MpegPS::SYSTEM_HEADER_CODE) {
                return self.get_system_header_packet(data, pos);
            } else {
                return Err(PacketError::NO_START_CODE);
            }
        }

        if let Some(pos) = MpegPS::find_start_code_of_av(data) {
            return self.get_pes_packet(data, pos);
        }

        return Err(PacketError::NO_START_CODE);
    }

    fn get_pes_packet(&mut self, data: &[u8], pos:usize) -> Result<PESPacketInfo, PacketError> {
        let mut buffer = bitbuf::BitBuffer::new( &data[pos..] );
        if buffer.len() < 6 {
            return Err(PacketError::OUT_LENGTH(0));
        }

        // check code
        let code:u32 = buffer.read(32).unwrap();

        // get length
        let mut pes_length = buffer.read(16).unwrap() as usize;
        if buffer.len() < 6 + pes_length {
            return Err(PacketError::OUT_LENGTH(6 + pes_length - buffer.len()));
        }

        if pes_length == 0 {
            if let Some(pos) = MpegPS::find_start_code_of_av( &data[6..] ) {
                pes_length = pos;
            } else {
                return Err(PacketError::OUT_LENGTH(0));
            }
        }

        if buffer.read(2).unwrap() == 0x01 {
            pes_length = pes_length - 2;
            buffer.skip(16);
        }

        // 11 = both present, 01 is forbidden, 10 = only PTS, 00 = no PTS or DTS
        let indicator = buffer.read(2).unwrap();
        if indicator == 0x00 {
            buffer.skip(4);
            pes_length = pes_length - 1;
            return Ok(PESPacketInfo {
                pkt_type:   MpegPS::code2type(code),
                code: code,
                offset: pos + buffer.pos() >> 3,
                len: pes_length,
                pts: 0
            });
        } else if indicator == 0x01 {
            return Err(PacketError::FORMAT_ERROR);
        }

        let mut ts:u64 = 0;
        ts = ts | (buffer.read(3).unwrap() as u64) << 30;
        buffer.skip(1);
        ts = ts | (buffer.read(15).unwrap() as u64) << 15;
        buffer.skip(1);
        ts = ts | (buffer.read(15).unwrap() as u64);
        buffer.skip(1);
        pes_length = pes_length - 5;

        if indicator == 0x03 {
            // skip dts
            buffer.skip(40);
            pes_length = pes_length -5;
        }

        return Ok(PESPacketInfo {
            pkt_type:   MpegPS::code2type(code),
            code: code,
            offset: pos + (buffer.pos()>>3),
            len: pes_length,
            pts: ts
        });
    }

    fn get_system_header_packet(&mut self, data: &[u8], pos:usize) -> Result<PESPacketInfo, PacketError> {
        let mut buffer = bitbuf::BitBuffer::new( &data[pos..] );
        if buffer.len() < 6 {
            return Err(PacketError::OUT_LENGTH(0));
        }

        // check code
        let code:u32 = buffer.read(32).unwrap();
        if code != MpegPS::SYSTEM_HEADER_CODE {
            return Err(PacketError::CODE_NOT_MATCH);
        }

        // get length
        let pes_length = buffer.read(16).unwrap() as usize;
        if buffer.len() < 6 + pes_length {
            return Err(PacketError::OUT_LENGTH(6 + pes_length - buffer.len()));
        }
        // get audio&video number
        buffer.skip(24);    //rate bound and marker bits
        self.num_audio_streams = buffer.read(6).unwrap() as i32;
        buffer.skip(5);
        self.num_video_streams = buffer.read(5).unwrap() as i32;

        // skip to end of packet
        buffer.skip( (pes_length - 5) * 8);

        let pkt = PESPacketInfo {
            pkt_type: PacketType::PS_SYSTEM_HEADER,
            code: code,
            offset: pos,
            len: buffer.pos() >> 3,
            pts: 0
        };

        self.has_system_header = true;
        Ok(pkt)
    }

    fn get_pack_header_packet(&mut self, data: &[u8], pos:usize) -> Result<PESPacketInfo, PacketError> {
        let mut buffer = bitbuf::BitBuffer::new( &data[pos..] );
        if buffer.len() < 12 {
            return Err(PacketError::OUT_LENGTH(12 - buffer.len()));
        }

        // check code
        let code:u32 = buffer.read(32).unwrap();
        if code != MpegPS::PACK_HEADER_CODE {
            return Err(PacketError::CODE_NOT_MATCH);
        }

        // check marker
        let marker_bits = buffer.read(4).unwrap();
        if marker_bits != 0x02 {
            return Err(PacketError::FORMAT_ERROR);
        }

        // get clock
        let mut clock:u64 = 0;
        clock = clock | (buffer.read(3).unwrap() as u64) << 30;
        buffer.skip(1);
        clock = clock | (buffer.read(15).unwrap() as u64) << 15;
        buffer.skip(1);
        clock = clock | (buffer.read(15).unwrap() as u64);
        buffer.skip(1);
        self.system_clock_ref = clock;

        // skip bitrate and stuff
        buffer.skip(1);
        let bit_rate = buffer.read(22).unwrap() as u64;
        buffer.skip(1);
        self.bit_rate = bit_rate;

        let pkt = PESPacketInfo {
            pkt_type: PacketType::PS_PACK_HEADER,
            code: code,
            offset: pos,
            len: buffer.pos() / 8,
            pts: 0
        };

        self.has_pack_header = true;
        Ok(pkt)
    }

    fn find_start_code_of_av(data: &[u8]) -> Option<usize> {
        if data.len() < 4 {
            return None
        }
        for pos in 0..data.len() - 3 {
            if data[pos] == 0x00
               && data[pos+1] == 0x00
               && data[pos+2] == 0x01 {
                let flag:u32 = 0x00000100 | data[pos+3] as u32;
                if (flag >= MpegPS::PES_VIDEO_BEGIN &&
                    flag <= MpegPS::PES_VIDEO_END) ||
                   (flag >= MpegPS::PES_AUDIO_BEGIN &&
                    flag <= MpegPS::PES_AUDIO_END) {
                    return Some(pos);
                }
            }
        }
        None
    }

    fn find_start_code(data: &[u8], code: u32) -> Option<usize> {
        if data.len() < 4 {
            return None
        }
        let c0 = (code >> 24) as u8;
        let c1 = (code >> 16) as u8;
        let c2 = (code >> 8) as u8;
        let c3 = code as u8;

        for pos in 0..data.len() - 3 {
            if data[pos] == c0
               && data[pos+1] == c1
               && data[pos+2] == c2
               && data[pos+3] == c3 {
                return Some(pos);
            }
        }
        None
    }
}
