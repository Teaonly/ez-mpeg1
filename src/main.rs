use std::env;
use std::fs::File;
use std::io::Read;

mod bitbuf;
mod pkt;
mod video;

fn main() {
    // file to [u8]
    let args: Vec<String> = env::args().collect();
    let mut file = File::open(&args[1]).unwrap();
    let mut data:Vec<u8> = Vec::new();
    let _result = file.read_to_end(&mut data);

    let mut vcodec = video::Mpeg1Video::new();

    let mut index:usize = 1400;
    let mut ps = pkt::MpegPS::new();
    ps.push(&data[0..index]);

    while index < data.len() {
        let pkt_result = ps.get();
        if let Ok(ref pkt) = pkt_result {
            println!("===={:?}", pkt);
            if pkt.pes_type == pkt::PacketType::PES_VIDEO {
                let payload = ps.payload(pkt);
                if vcodec.push(payload).is_none() {
                    panic!("Decoder's buffer is full,can't do any decoding");
                }
                if vcodec.include_one_frame() {
                    vcodec.decode();
                }
            }
        }
        if let Err(e) = pkt_result {
            if let pkt::PacketError::OUT_LENGTH(more) = e {
                let mut pushed = 1280 + more;
                if pushed > data.len() - index {
                    pushed = data.len() - index;
                }
                index = index + ps.push(&data[index..(index+pushed)]);
            } else if let pkt::PacketError::NO_START_CODE = e {
                let mut pushed = 1280;
                if pushed > data.len() - index {
                    pushed = data.len() - index;
                }
                index = index + ps.push(&data[index..(index+pushed)]);
            } else {
                println!("Can't handle Error: {:?}", e);
                break;
            }
        }
    }
}
