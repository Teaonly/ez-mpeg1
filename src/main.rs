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

    buffer.push(&data[0..1024]);
    let pack_pkt = ps.get_packet(buffer.get(0)).unwrap();
    let system_pkt = ps.get_packet(buffer.get( pack_pkt.len )).unwrap();

    println!("==============={:?}", pack_pkt);
    println!("==============={:?}", system_pkt);
}
