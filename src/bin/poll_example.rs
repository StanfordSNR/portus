extern crate nix;

use std::{str, thread, time};
use std::os::unix::net::UnixDatagram;
use std::os::unix::io::AsRawFd;
use nix::poll::{POLLIN, POLLOUT, PollFd, poll};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 3 {
        eprintln!("Usage: ./poll_example <local_addr> <peer_addr>");
        std::process::exit(1);
    }

    let local_addr = &args[1];
    let peer_addr = &args[2];

    let sock = UnixDatagram::bind(local_addr).unwrap();
    sock.set_nonblocking(true).unwrap();

    let mut fds = [PollFd::new(sock.as_raw_fd(), POLLIN),
                   PollFd::new(sock.as_raw_fd(), POLLOUT)];

    loop {
        poll(&mut fds, -1).unwrap();

        if fds[0].revents().unwrap().contains(POLLIN) {
            println!("POLLIN event");

            let mut recv_buf = [0u8; 1024];
            let (count, addr) = sock.recv_from(&mut recv_buf).unwrap();
            println!("Received {} from {:?}",
                     str::from_utf8(&recv_buf[..count]).unwrap(), addr);
        }

        if fds[1].revents().unwrap().contains(POLLOUT) {
            println!("POLLOUT event");

            let send_buf = b"Hello world!";
            sock.send_to(send_buf, peer_addr).unwrap();
            println!("Sent {} to {:?}",
                     str::from_utf8(send_buf).unwrap(), peer_addr);
        }

        // pause for 1 second
        thread::sleep(time::Duration::from_secs(1));
    }
}
