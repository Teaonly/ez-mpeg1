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

