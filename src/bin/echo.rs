extern crate nix;

use std::str;
use std::os::unix::net::UnixDatagram;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 3 {
        eprintln!("Usage: ./echo <local_addr> <peer_addr>");
        std::process::exit(1);
    }

    let local_addr = &args[1];
    let peer_addr = &args[2];

    let sock = UnixDatagram::bind(local_addr).unwrap();

    let mut buf = [0u8; 1024];
    loop {
        let (count, addr) = sock.recv_from(&mut buf).unwrap();
        println!("Received {} from {:?}",
                 str::from_utf8(&buf[..count]).unwrap(), addr);

        sock.send_to(&buf, peer_addr).unwrap();
    }
}
