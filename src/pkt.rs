// https://en.wikipedia.org/wiki/MPEG_program_stream
// https://en.wikipedia.org/wiki/Packetized_elementary_stream

use std::ptr;
use crate::bitbuf;

#[allow(non_camel_case_types)]
#[derive(Debug)]
pub enum PacketError {
    OUT_LENGTH(usize),
    NO_START_CODE,
    FORMAT_ERROR,
}

#[allow(non_camel_case_types)]
#[derive(Debug, PartialEq)]
pub enum PacketType {
    PS_PACK_HEADER,
    PS_SYSTEM_HEADER,
    PES_AUDIO,
    PES_VIDEO,
    PES_SKIP,
    PES_UNKNOW,
}

#[derive(Debug)]
pub struct PESPacketInfo {
    pub pes_type: PacketType,
    pub code: u8,
    pub pts: u64,

    pub offset: usize,
    pub len: usize,
    pub payload: usize,
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

    ts_buffer_:     [u8; 188],
    ts_offset_:     usize,
    ts_pid_:        i32,
}

impl MpegPS {
    const PACK_HEADER_CODE: u8 = 0xBA;
    const SYSTEM_HEADER_CODE: u8 = 0xBB;
    const PES_VIDEO_BEGIN: u8 = 0xE0;
    const PES_VIDEO_END: u8 = 0xEF;
    const PES_AUDIO_BEGIN: u8 = 0xC0;
    const PES_AUDIO_END: u8 = 0xDF;
    const PES_PRIVATE_CODE: u8 = 0xBD;
    const PES_PADDING_CODE: u8 = 0xBE;

    fn code2type(code:u8) -> PacketType {
        if code >= MpegPS::PES_AUDIO_BEGIN &&
           code <= MpegPS::PES_AUDIO_END {
            return PacketType::PES_AUDIO;
        }

        if code >= MpegPS::PES_VIDEO_BEGIN &&
           code <= MpegPS::PES_VIDEO_END {
            return PacketType::PES_VIDEO;
        }

        if code == MpegPS::PACK_HEADER_CODE ||
           code == MpegPS::SYSTEM_HEADER_CODE ||
           code == MpegPS::PES_PADDING_CODE ||
           code == MpegPS::PES_PRIVATE_CODE {
            return PacketType::PES_SKIP;
        }

        return PacketType::PES_UNKNOW;
    }

    fn data<'a>(&'a self, pos: usize) -> &'a [u8] {
        return &self.buffer_[self.offset_ + pos .. self.len_];
    }

    fn rewind(&mut self) {
        unsafe {
            let dst: *mut u8 = self.buffer_.as_ptr() as *mut u8;
            let src: *const u8 = self.buffer_.as_ptr().add(self.offset_);
            ptr::copy(src, dst, self.len_ - self.offset_);
        }
        self.len_ = self.len_ - self.offset_;
        self.offset_ = 0;
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

            ts_buffer_: [0;188],
            ts_offset_: 0,
            ts_pid_:    -1,
        }
    }

    pub fn push(&mut self, data: &[u8]) -> usize {
        let mut copy_len = self.buffer_.len() - self.len_;
        if copy_len > data.len() {
            copy_len = data.len();
        }
        unsafe {
            let dst: *mut u8 = self.buffer_.as_ptr().add(self.len_) as *mut u8;
            let src: *const u8 = data.as_ptr();
            ptr::copy(src, dst, copy_len);
        }
        self.len_ = self.len_ + copy_len;

        return copy_len;
    }

    pub fn payload(&self, pkt: &PESPacketInfo) -> &[u8] {
        if pkt.offset + pkt.len > self.len_ {
            panic!("Can't get payload from buffer");
        }
        &self.buffer_[(pkt.offset  + pkt.payload)..(pkt.offset + pkt.len)]
    }

    pub fn get(&mut self) -> Result<PESPacketInfo, PacketError> {
        let mut pkt_result = self.get_packet();
        if let Ok(ref mut pkt) = pkt_result {
            self.offset_ = self.offset_ + pkt.len + pkt.offset;
            pkt.offset = self.offset_ - pkt.len;
        }
        if let Err(ref e) = pkt_result {
            if let PacketError::OUT_LENGTH(more) = e {
                let push_size = more + 1280;
                let remain = self.buffer_.len() - self.len_;
                if push_size > remain {
                    self.rewind();
                }
            }
            if let PacketError::NO_START_CODE = e {
                self.rewind();
            }
        }
        pkt_result
    }

    fn get_packet(&mut self) -> Result<PESPacketInfo, PacketError> {
        let code:u8;
        let pos:usize;
        if let Some((_pos, _code) ) = MpegPS::find_start_code(self.data(0)) {
            pos = _pos;
            code = _code;
        } else {
            return Err(PacketError::NO_START_CODE);
        }

        if code == MpegPS::PACK_HEADER_CODE {
            return self.get_pack_header_packet(pos);
        } else if code == MpegPS::SYSTEM_HEADER_CODE {
            return self.get_system_header_packet(pos);
        }
        if MpegPS::code2type(code) != PacketType::PES_UNKNOW {
            return self.get_pes_packet(pos);
        }

        return Err(PacketError::FORMAT_ERROR);
    }

    fn get_pes_packet(&mut self, begin:usize) -> Result<PESPacketInfo, PacketError> {
        let data = &self.data(begin);

        let mut buffer = bitbuf::BitBuffer::new( data );
        if buffer.len() < 6 {
            return Err(PacketError::OUT_LENGTH(0));
        }

        // check code
        let code:u8 = (buffer.read(32).unwrap() & 0xFF) as u8;

        // get length
        let mut pes_length = buffer.read(16).unwrap() as usize;
        if buffer.len() < 6 + pes_length {
            return Err(PacketError::OUT_LENGTH(6 + pes_length - buffer.len()));
        }
        if MpegPS::code2type(code) == PacketType::PES_SKIP {
            return Ok(PESPacketInfo {
                pes_type:   PacketType::PES_SKIP,
                code: code,
                offset: begin,
                len: pes_length + 6,
                pts: 0,
                payload: 0,
            });
        }

        let mut payload = 6;
        if pes_length == 0 {
            if let Some((pos, _)) = MpegPS::find_start_code( &data[6..] ) {
                pes_length = pos;
            } else {
                return Err(PacketError::OUT_LENGTH(0));
            }
        }

        if buffer.read(2).unwrap() == 0x01 {
            buffer.skip(16);
            payload += 2;
        }

        // 11 = both present, 01 is forbidden, 10 = only PTS, 00 = no PTS or DTS
        let indicator = buffer.read(2).unwrap();
        if indicator == 0x00 {
            buffer.skip(4);
            payload += 1;
            return Ok(PESPacketInfo {
                pes_type:   MpegPS::code2type(code),
                code: code,
                offset: begin ,
                len: pes_length + 6,
                pts: 0,
                payload: payload,
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
        payload += 5;

        if indicator == 0x03 {
            // skip dts
            buffer.skip(40);
            payload += 5;
        }

        return Ok(PESPacketInfo {
            pes_type:   MpegPS::code2type(code),
            code: code,
            offset: begin,
            len: pes_length + 6,
            pts: ts,
            payload: payload,
        });
    }

    fn get_system_header_packet(&mut self, begin:usize) -> Result<PESPacketInfo, PacketError> {
        let data = self.data(begin);

        let mut buffer = bitbuf::BitBuffer::new( data );
        if buffer.len() < 6 {
            return Err(PacketError::OUT_LENGTH(0));
        }

        // check code
        let code:u8 = (buffer.read(32).unwrap() & 0xFF) as u8;

        // get length
        let pes_length = buffer.read(16).unwrap() as usize;
        if buffer.len() < 6 + pes_length {
            return Err(PacketError::OUT_LENGTH(6 + pes_length - buffer.len()));
        }

        // get audio&video number
        buffer.skip(24);    //rate bound and marker bits
        let num_audio_streams = buffer.read(6).unwrap() as i32;
        buffer.skip(5);
        let num_video_streams = buffer.read(5).unwrap() as i32;

        // skip to end of packet
        buffer.skip( (pes_length - 5) * 8);

        let pkt = PESPacketInfo {
            pes_type: PacketType::PS_SYSTEM_HEADER,
            code: code,
            offset: begin,
            len: buffer.pos() >> 3,
            pts: 0,
            payload: 0,
        };

        self.num_audio_streams = num_audio_streams;
        self.num_video_streams = num_video_streams;
        self.has_system_header = true;
        Ok(pkt)
    }

    fn get_pack_header_packet(&mut self, begin:usize) -> Result<PESPacketInfo, PacketError> {
        let data = self.data(begin);

        let mut buffer = bitbuf::BitBuffer::new( data );
        if buffer.len() < 12 {
            return Err(PacketError::OUT_LENGTH(12 - buffer.len()));
        }

        // check code
        let code:u8 = (buffer.read(32).unwrap() & 0xFF) as u8;

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

        // skip bitrate and stuff
        buffer.skip(1);
        let bit_rate = buffer.read(22).unwrap() as u64;
        buffer.skip(1);

        let pkt = PESPacketInfo {
            pes_type: PacketType::PS_PACK_HEADER,
            code: code,
            offset: begin,
            len: buffer.pos() >> 3,
            pts: 0,
            payload: 0,
        };

        self.system_clock_ref = clock;
        self.bit_rate = bit_rate;
        self.has_pack_header = true;
        Ok(pkt)
    }

    fn find_start_code(data:&[u8]) -> Option<(usize, u8)> {
        if data.len() < 4 {
            return None
        }

        for pos in 0..data.len() - 3 {
            if data[pos] == 0x00
               && data[pos+1] == 0x00
               && data[pos+2] == 0x01 {
                let code = data[pos+3];
                if MpegPS::code2type(code) != PacketType::PES_UNKNOW {
                    return Some( (pos, code));
                }
            }
        }
        None
    }

    fn pid2pes(&mut self, ts:(u32, u32, u32, u32, usize)) -> bool {
        if ts.0 == 0x01 && self.ts_pid_ == -1 {
            if ts.3 >= 0x000001E0 && ts.3 <= 0x000001EF {
                self.ts_pid_ = ts.1 as i32;
                return true;
            }
        }

        if self.ts_pid_ == -1 {
            return false;
        }

        if ts.1 as i32 == self.ts_pid_ {
            return true;
        }

        return false;
    }

    // added implementation for MPEG-TS support
    fn remind(&self) -> usize {
        self.buffer_.len() - self.len_
    }

    fn parse_ts(data: &[u8]) -> (u32, u32, u32, u32, usize) {
        // check ts packet itself
        let mut buffer = bitbuf::BitBuffer::new(data);
        if buffer.read(8).unwrap() != 0x47 {
            panic!("Can't find 0x47 in TS package");
        }

        let _tei = buffer.read(1);
        let payload_start = buffer.read(1).unwrap();
        let _transport_priority = buffer.read(1);
        let pid = buffer.read(13).unwrap();
        let _tsc = buffer.read(2);
        let adaptation_field_control = buffer.read(2).unwrap();
        let continuity_counter = buffer.read(4).unwrap();

        if adaptation_field_control == 0x00 {
            panic!("Can't support adaptation_field_control = 0x00");
        }
        if (adaptation_field_control & 0x02) != 0 {
            let filed_length = buffer.read(8).unwrap();
            buffer.skip( (filed_length << 3) as usize);
        }
        let code:u32 = if buffer.has(32) {
            let ret = buffer.read(32).unwrap();
            buffer.back(32);
            ret
        } else {
            0xFFFFFFFF
        };

        return (payload_start, pid, continuity_counter, code, buffer.pos() / 8);
    }

    pub fn push_ts(&mut self, data: &[u8]) -> usize {
        if data.len() < 188 {
            return 0;
        }

        let mut offset:usize = 0;

        if self.ts_offset_ > 0 {
            offset = 188 - self.ts_offset_;
            unsafe {
                let dst: *mut u8 = self.ts_buffer_.as_ptr().add( self.ts_offset_ ) as *mut u8;
                let src: *const u8 = data.as_ptr();
                ptr::copy(src, dst, offset);
            }
            self.ts_offset_ = 0;

            let ret = MpegPS::parse_ts(&self.ts_buffer_);
            println!("============{:?}", ret);

            let payload_len = 188 - ret.4;
            if payload_len < self.remind() {
                if self.pid2pes(ret) {
                    unsafe {
                        let dst: *mut u8 = self.buffer_.as_ptr().add(self.len_) as *mut u8;
                        let src: *const u8 = self.ts_buffer_.as_ptr().add(ret.4);
                        ptr::copy(src, dst, payload_len);
                    }
                    self.len_ = self.len_ + payload_len;
                }
            } else {
                return 0;
            }
        }

        while data.len() >= 188 + offset {
            let ret = MpegPS::parse_ts(&data[offset..offset+188]);
            println!("============{:?}", ret);

            let payload_len = 188 - ret.4;
            if payload_len < self.remind() {
                if self.pid2pes(ret) {
                    self.push( &data[ret.4 + offset .. offset+188] );
                }
                offset += 188;
            } else {
                return offset;
            }
        }

        if offset < data.len() {
            unsafe {
                let dst: *mut u8 = self.ts_buffer_.as_ptr() as *mut u8;
                let src: *const u8 = data.as_ptr().add(offset);
                ptr::copy(src, dst, data.len() - offset);
            }
            self.ts_offset_ = data.len() - offset;
        }

        return  data.len();
    }

}

