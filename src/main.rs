use std::env;
use std::fs::File;
use std::io::Read;

mod bitbuf;
mod pkt;

fn main() {
    // file to [u8]
    let args: Vec<String> = env::args().collect();
    let mut file = File::open(&args[1]).unwrap();
    let mut data:Vec<u8> = Vec::new();
    let _result = file.read_to_end(&mut data);

    let mut index:usize = 1280;
    let mut ps = pkt::MpegPS::new();
    ps.push(&data[0..index]);

    while index < data.len() {
        let pkt_result = ps.get();
        if let Ok(ref pkt) = pkt_result {
            println!("===={:?}", pkt);
        }
        if let Err(e) = pkt_result {
            if let pkt::PacketError::OUT_LENGTH(_more) = e {
                let mut pushed = 1280;
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
