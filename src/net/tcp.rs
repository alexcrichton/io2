use prelude::v1::*;
use io::prelude::*;

use io;
use net::{ToSocketAddrs, SocketAddr, Shutdown};
use sys_common::net as net_imp;

pub struct TcpStream(net_imp::TcpStream);
pub struct TcpListener(net_imp::TcpListener);
pub struct Incoming<'a> { listener: &'a TcpListener }

impl TcpStream {
    pub fn connect<A: ToSocketAddrs + ?Sized>(addr: &A) -> io::Result<TcpStream> {
        super::each_addr(addr, net_imp::TcpStream::connect).map(TcpStream)
    }

    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        self.0.peer_addr()
    }

    pub fn socket_addr(&self) -> io::Result<SocketAddr> {
        self.0.socket_addr()
    }

    pub fn shutdown(&self, how: Shutdown) -> io::Result<()> {
        self.0.shutdown(how)
    }

    pub fn duplicate(&self) -> io::Result<TcpStream> {
        self.0.duplicate().map(TcpStream)
    }

    pub fn set_nodelay(&self, nodelay: bool) -> io::Result<()> {
        self.0.set_nodelay(nodelay)
    }

    pub fn set_keepalive(&self, seconds: Option<u32>) -> io::Result<()> {
        self.0.set_keepalive(seconds)
    }
}

impl Read for TcpStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> { self.0.read(buf) }
}
impl Write for TcpStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> { self.0.write(buf) }
}
impl<'a> Read for &'a TcpStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> { self.0.read(buf) }
}
impl<'a> Write for &'a TcpStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> { self.0.write(buf) }
}

impl TcpListener {
    pub fn bind<A: ToSocketAddrs + ?Sized>(addr: &A) -> io::Result<TcpListener> {
        super::each_addr(addr, net_imp::TcpListener::bind).map(TcpListener)
    }

    pub fn socket_addr(&self) -> io::Result<SocketAddr> {
        self.0.socket_addr()
    }

    pub fn duplicate(&self) -> io::Result<TcpListener> {
        self.0.duplicate().map(TcpListener)
    }

    pub fn accept(&self) -> io::Result<(TcpStream, SocketAddr)> {
        self.0.accept().map(|(a, b)| (TcpStream(a), b))
    }

    pub fn incoming(&self) -> Incoming {
        Incoming { listener: self }
    }
}


impl<'a> Iterator for Incoming<'a> {
    type Item = io::Result<TcpStream>;
    fn next(&mut self) -> Option<io::Result<TcpStream>> {
        Some(self.listener.accept().map(|p| p.0))
    }
}
