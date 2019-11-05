#[derive(Debug)]
pub struct RingBitBuffer {
    buffer_: Vec<u8>,
    rbi_:    usize,
    wi_:     usize,
    cap_:    usize,
}

impl RingBitBuffer {
    fn round(&self, p: usize) -> usize {
        (p + self.cap_) % self.cap_
    }

    pub fn len(&self) -> usize {
        if self.empty() {
            return 0;
        }
        let has_bytes:usize = (self.wi_ + self.cap_ - (self.rbi_ >> 3) ) % self.cap_;
        let has_bits = has_bytes * 8 - (self.rbi_ & 0x07);
        has_bits
    }

    pub fn has(&self, count: usize) -> bool {
        if self.len() >= count {
            return true;
        }
        return false;
    }

    pub fn new() -> Self {
        let mut buffer:Vec<u8> = Vec::new();
        buffer.resize(1024*1024*4, 0);

        RingBitBuffer {
            cap_:       buffer.len(),
            buffer_:    buffer,
            rbi_:       0,
            wi_:        0,
        }
    }

    pub fn empty(&self) -> bool {
        if (self.rbi_ >> 3) == self.wi_ {
            return true;
        }
        return false;
    }

    pub fn full(&self) -> bool {
        if self.round(self.wi_ + 1) == (self.rbi_ >> 3) {
            return true;
        }
        return false;
    }

    pub fn push(&mut self, data: &[u8]) -> Option<usize> {
        let mut byte_space:usize = ((self.rbi_>>3) + self.cap_ - self.wi_ ) % self.cap_;
        if byte_space == 0 {
            byte_space = self.cap_;
        }
        if byte_space < 1 + data.len() {
            return None
        }

        let mut wlen = 0;
        while wlen < data.len() {
            self.buffer_[self.wi_] = data[wlen];
            self.wi_ = self.round(self.wi_ + 1);
            wlen += 1;
        }
        Some(wlen)
    }

    pub fn skip(&mut self, count: usize) -> usize {
        if self.has(count) {
            self.rbi_ = (self.rbi_ + count) % (self.cap_ * 8);
            return count;
        }
        0
    }

    pub fn read(&mut self, mut count: usize) -> u32 {
        if !self.has(count) {
            return 0x00;
        }

        let mut value:u32 = 0x00;
        while count > 0 {
            let current_byte = self.buffer_[self.rbi_ >> 3] as u32;

            let remaining = 8 - (self.rbi_ & 7);  // Remaining bits in byte
            let read = if remaining < count {    // Bits in self run
                remaining
            } else {
                count
            };

            let shift = remaining - read;
            let mask = 0xff >> (8 - read);

            value = (value << read) | ((current_byte & (mask << shift)) >> shift);

            self.rbi_ = (self.rbi_ + read)  % (self.cap_ * 8);

            count = count - read;
        }
        value
    }

    pub fn include_two_code(&self, code:u32 ) -> bool {
        let mut pos = (self.rbi_ + 7) >> 3;
        let mut pattern:u32 = 0xFFFFFFFF;
        let mut times = 0;
        while pos != self.wi_ {
            pattern = (pattern << 8) | (self.buffer_[pos] as u32);
            pos = self.round(pos+1);

            if pattern == code {
                times += 1;
                if times == 2 {
                    return true;
                }
            }
        }
        return false;
    }

    pub fn find_start_code(&mut self, code: u32) -> bool {
        // aligen to byte
        self.rbi_ = ((self.rbi_ + 7) >> 3) << 3;
        self.rbi_ = self.rbi_ % (self.cap_ * 8);

        let mut pattern:u32 = 0xFFFFFFFF;
        while (self.rbi_ >> 3) != self.wi_ {
            pattern = (pattern << 8) | (self.buffer_[self.rbi_>>3] as u32);

            self.rbi_ += 8;
            self.rbi_ = self.rbi_ % (self.cap_ * 8);

            if pattern == code {
                return true;
            }
        }

        return false;
    }
}

pub struct BitBuffer<'a> {
    _buf:  &'a [u8],
    _bi:   usize,
}

impl<'a> BitBuffer<'a> {
    pub fn len(&self) -> usize {
        self._buf.len()
    }
    pub fn new(buf: &'a[u8]) -> Self {
        BitBuffer::<'a> {
            _buf: buf,
            _bi:  0
        }
    }
    pub fn pos(&self) -> usize {
        self._bi
    }
    pub fn has(&self, count: usize) -> bool {
        if count <= self._buf.len() * 8 - self._bi {
            return true;
        }
        return false;
    }
    pub fn skip(&mut self, count: usize) -> usize {
        if self.has(count) {
            self._bi = self._bi + count;
            return count;
        }
        0
    }
    pub fn read(&mut self, mut count: usize) -> Option<u32> {
        if !self.has(count) {
            return None;
        }
        let mut value:u32 = 0x00;

        while count > 0 {
            let current_byte = self._buf[self._bi >> 3] as u32;

            let remaining = 8 - (self._bi & 7);  // Remaining bits in byte
            let read = if remaining < count {    // Bits in self run
                remaining
            } else {
                count
            };

            let shift = remaining - read;
            let mask = 0xff >> (8 - read);

            value = (value << read) | ((current_byte & (mask << shift)) >> shift);

            self._bi = self._bi + read;
            count = count - read;
        }

        Some(value)
    }
}



