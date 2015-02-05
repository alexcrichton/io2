use prelude::v1::*;

use io::{self, Error, ErrorKind};
use sys_common::net as net_imp;

pub use self::ip::{IpAddr, Ipv4Addr, Ipv6Addr, Ipv6MulticastScope};
pub use self::addr::{SocketAddr, ToSocketAddrs};
pub use self::tcp::{TcpStream, TcpListener};
pub use self::udp::UdpSocket;

mod ip;
mod addr;
mod tcp;
mod udp;
mod parser;

#[derive(Copy, Clone, PartialEq)]
pub enum Shutdown {
    Read, Write, Both
}

fn each_addr<A: ToSocketAddrs + ?Sized, F, T>(addr: &A, mut f: F) -> io::Result<T>
    where F: FnMut(&SocketAddr) -> io::Result<T>
{
    let mut last_err = None;
    for addr in try!(addr.to_socket_addrs()).iter() {
        match f(addr) {
            Ok(l) => return Ok(l),
            Err(e) => last_err = Some(e),
        }
    }
    Err(last_err.unwrap_or_else(|| {
        Error::new(ErrorKind::InvalidInput,
                   "could not resolve to any addresses", None)
    }))
}

pub struct LookupHost(net_imp::LookupHost);

impl Iterator for LookupHost {
    type Item = io::Result<SocketAddr>;
    fn next(&mut self) -> Option<io::Result<SocketAddr>> { self.0.next() }
}

pub fn lookup_host(host: &str) -> io::Result<LookupHost> {
    net_imp::lookup_host(host).map(LookupHost)
}
