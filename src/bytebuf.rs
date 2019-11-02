use std::ptr;
pub struct ByteBuffer {
    _buf:   Vec<u8>,
    _len:   usize,
}

impl ByteBuffer {
    pub fn new(capacity: usize) -> Self {
        let mut _buf:Vec<u8> = Vec::new();
        _buf.resize(capacity, 0);
        let _len:usize = 0;
        ByteBuffer{_buf, _len}
    }

    pub fn len(&self) -> usize {
        self._len
    }

    pub fn capacity(&self) -> usize {
        self._buf.len()
    }

    pub fn remain(&self) -> usize {
        self._buf.len() - self._len
    }

    pub fn get<'a>(&'a self, offset: usize) ->&'a [u8]{
        &self._buf[offset..self._len]
    }

    pub fn rewind(&mut self, offset: usize) {
        if offset >= self._len {
            return;
        }

        unsafe {
            let dst: *mut u8 = self._buf.as_ptr() as *mut u8;
            let src: *const u8 = self._buf.as_ptr().add(offset);
            ptr::copy(src, dst, self._len - offset);
        }
        self._len = self._len - offset;
    }

    pub fn push(&mut self, data: &[u8]) {
        let mut len = data.len();

        let mut copy_len = self._buf.len() - self._len;
        if copy_len > len {
            copy_len = len;
        }
        unsafe {
            let dst: *mut u8 = self._buf.as_ptr().add(self._len) as *mut u8;
            let src: *const u8 = data.as_ptr();
            ptr::copy(src, dst, copy_len);
        }
        self._len = self._len + copy_len;

        len = len - copy_len;
        if len > 0 {
            self._buf.extend_from_slice(&data[copy_len..]);
            self._len = self._buf.len()
        }
    }
}
