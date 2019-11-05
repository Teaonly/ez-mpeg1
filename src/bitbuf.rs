#[derive(Debug)]
pub struct RingBitBuffer {
    buffer_: Vec<u8>,
    rbi_:    usize,
    wi_:     usize,
    cap_:    usize,
}

impl RingBitBuffer {
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
        if (self.rbi_ >> 3) == ((self.wi_ + 1) % self.cap_)  {
            return true;
        }
        return false;
    }

    pub fn push(&mut self, data: &[u8]) -> usize {
        let mut wlen = 0;
        while wlen < data.len() {
            if self.full() {
                break;
            }
            self.buffer_[self.wi_] = data[wlen];
            wlen += 1;
            self.wi_ = (self.wi_ + 1) % self.cap_;
        }
        wlen
    }

    pub fn has(&self, count: usize) -> bool {
        if self.empty() {
            return false;
        }
        let mut has_bytes:usize = (self.rbi_ + self.cap_ - (self.rbi_ >> 3) ) % self.cap_;
        let has_bits = has_bytes * 8 - (self.rbi_ & 0x07);
        if has_bits >= count {
            return true;
        }
        return false;
    }

    pub fn skip(&mut self, count: usize) -> usize {
        if self.has(count) {
            self.rbi_ = (self.rbi_ + count) % (self.cap_ * 8);
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

        Some(value)
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



