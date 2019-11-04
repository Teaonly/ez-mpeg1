use std::env;
use std::fs::File;
use std::io::Read;

mod bitbuf;
mod bytebuf;
mod pkt;

fn main() {
    // file to [u8]
    let args: Vec<String> = env::args().collect();
    let mut file = File::open(&args[1]).unwrap();
    let mut data:Vec<u8> = Vec::new();
    let _result = file.read_to_end(&mut data);

    let mut ps = pkt::MpegPS::new();
    /*
    let mut buffer = bytebuf::ByteBuffer::new(1024*1024*4);
    buffer.push(&data[0..1280]);

    let mut offset:usize = 0;
    let mut index:usize = 1280;

    while index < data.len() {
        let pkt_result = ps.get_packet(buffer.get(offset));
        if let Ok(ref pkt) = pkt_result {
            println!("===={:?}", pkt);
            offset = offset + pkt.pos();
        }
        if let Err(e) = pkt_result {
            if let pkt::PacketError::OUT_LENGTH(more) = e {
                let mut push_size = more + 1280;
                if push_size > data.len() - index {
                    push_size = data.len() - index;
                }
                if push_size <= buffer.remain() {
                    buffer.push( &data[index..(index+push_size)] );
                    index = index + push_size;
                } else {
                    buffer.rewind(offset);
                    offset = 0;
                }
            } else {
                println!("Can't handle Error: {:?}", e);
                break;
            }
        }
    }
    */
}
