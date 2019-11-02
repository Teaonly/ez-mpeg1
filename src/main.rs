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

    let mut buffer = bytebuf::ByteBuffer::new(1024*1024*4);
    let mut ps = pkt::MpegPS::new();

    /*
    buffer.push(&data[0..1024]);
    let mut offset:usize = 0;

    let pack_pkt = ps.get_packet(buffer.get(0)).unwrap();
    offset = offset + pack_pkt.pos();
    let system_pkt = ps.get_packet(buffer.get(offset )).unwrap();
    offset = offset + system_pkt.pos();

    println!("==============={:?}", pack_pkt);
    println!("==============={:?}", system_pkt);

    let pes_pkt = ps.get_packet(buffer.get(offset));
    println!("{:?}", pes_pkt);
    */

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
}
