use prelude::v1::*;

use io::{self, Error, ErrorKind};
use net::{ToSocketAddrs, SocketAddr, IpAddr};
use sys_common::net as net_imp;

pub struct UdpSocket(net_imp::UdpSocket);

impl UdpSocket {
    pub fn bind<A: ToSocketAddrs>(addr: &A) -> io::Result<UdpSocket> {
        super::each_addr(addr, net_imp::UdpSocket::bind).map(UdpSocket)
    }

    pub fn recv_from(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        self.0.recv_from(buf)
    }

    pub fn send_to<A: ToSocketAddrs + ?Sized>(&self, buf: &[u8], addr: &A)
                                              -> io::Result<usize> {
        let addrs = try!(addr.to_socket_addrs());
        match addrs.get(0) {
            Some(addr) => self.0.send_to(buf, addr),
            None => Err(Error::new(ErrorKind::InvalidInput,
                                   "no addresses to send data to", None)),
        }
    }

    pub fn socket_addr(&self) -> io::Result<SocketAddr> {
        self.0.socket_addr()
    }

    pub fn duplicate(&self) -> io::Result<UdpSocket> {
        self.0.duplicate().map(UdpSocket)
    }

    pub fn set_broadcast(&self, on: bool) -> io::Result<()> {
        self.0.set_broadcast(on)
    }

    pub fn set_multicast_loop(&self, on: bool) -> io::Result<()> {
        self.0.set_multicast_loop(on)
    }

    pub fn join_multicast(&self, multi: &IpAddr) -> io::Result<()> {
        self.0.join_multicast(multi)
    }

    pub fn leave_multicast(&self, multi: &IpAddr) -> io::Result<()> {
        self.0.leave_multicast(multi)
    }

    pub fn multicast_time_to_live(&self, ttl: i32) -> io::Result<()> {
        self.0.multicast_time_to_live(ttl)
    }

    pub fn time_to_live(&self, ttl: i32) -> io::Result<()> {
        self.0.time_to_live(ttl)
    }
}
