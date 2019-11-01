#[derive(Debug, Clone)]
struct BitBuffer {
    _buf: Vec<u8>,

};

impl BitBuffer {
    pub fn new(blk_num: i32) {
        let buf:Vec<u8> = Vec::new();
        buf.resize( BitBuffer::BLOCK_SIZE * blk_num as usize, 0);

        BitBuffer {
            _buf:   buf,
            _w_index: 0,
            _w_size: 0,
            _rby_index: -1,
            _rbi_index: 0,
        }
    }

    pub fn capacity(&self) -> u64{
        self._buf.len()
    }

    pub fn len(&self) -> u64{

    }

    pub fn push_bytes(data: &[u8]) -> Option<String> {

    }
}
