use prelude::v1::*;

use fmt;
use io;
use net::{IpAddr, lookup_host};

#[derive(Copy, PartialEq, Eq, Clone, Hash, Debug)]
pub struct SocketAddr {
    pub ip: IpAddr,
    pub port: u16,
}

#[stable(feature = "rust1", since = "1.0.0")]
impl fmt::Display for SocketAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.ip {
            IpAddr::V4(_) => write!(f, "{}:{}", self.ip, self.port),
            IpAddr::V6(_) => write!(f, "[{}]:{}", self.ip, self.port),
        }
    }
}

/// A trait for objects which can be converted or resolved to one or more
/// `SocketAddr` values.
///
/// Implementing types minimally have to implement either `to_socket_addr` or
/// `to_socket_addr_all` method, and its trivial counterpart will be available
/// automatically.
///
/// This trait is used for generic address resolution when constructing network
/// objects.  By default it is implemented for the following types:
///
///  * `SocketAddr` - `to_socket_addr` is identity function.
///
///  * `(IpAddr, u16)` - `to_socket_addr` constructs `SocketAddr` trivially.
///
///  * `(&str, u16)` - the string should be either a string representation of an
///    IP address expected by `FromStr` implementation for `IpAddr` or a host
///    name.
///
///    For the former, `to_socket_addr_all` returns a vector with a single
///    element corresponding to that IP address joined with the given port.
///
///    For the latter, it tries to resolve the host name and returns a vector of
///    all IP addresses for the host name, each joined with the given port.
///
///  * `&str` - the string should be either a string representation of a
///    `SocketAddr` as expected by its `FromStr` implementation or a string like
///    `<host_name>:<port>` pair where `<port>` is a `u16` value.
///
///    For the former, `to_socket_addr_all` returns a vector with a single
///    element corresponding to that socket address.
///
///    For the latter, it tries to resolve the host name and returns a vector of
///    all IP addresses for the host name, each joined with the port.
///
///
/// This trait allows constructing network objects like `TcpStream` or
/// `UdpSocket` easily with values of various types for the bind/connection
/// address. It is needed because sometimes one type is more appropriate than
/// the other: for simple uses a string like `"localhost:12345"` is much nicer
/// than manual construction of the corresponding `SocketAddr`, but sometimes
/// `SocketAddr` value is *the* main source of the address, and converting it to
/// some other type (e.g. a string) just for it to be converted back to
/// `SocketAddr` in constructor methods is pointless.
///
/// Some examples:
///
/// ```rust,no_run
/// # #![allow(unused_must_use)]
///
/// use std::old_io::{TcpStream, TcpListener};
/// use std::old_io::net::udp::UdpSocket;
/// use std::old_io::net::ip::{Ipv4Addr, SocketAddr};
///
/// fn main() {
///     // The following lines are equivalent modulo possible "localhost" name resolution
///     // differences
///     let tcp_s = TcpStream::connect(SocketAddr { ip: Ipv4Addr(127, 0, 0, 1), port: 12345 });
///     let tcp_s = TcpStream::connect((Ipv4Addr(127, 0, 0, 1), 12345u16));
///     let tcp_s = TcpStream::connect(("127.0.0.1", 12345u16));
///     let tcp_s = TcpStream::connect(("localhost", 12345u16));
///     let tcp_s = TcpStream::connect("127.0.0.1:12345");
///     let tcp_s = TcpStream::connect("localhost:12345");
///
///     // TcpListener::bind(), UdpSocket::bind() and UdpSocket::send_to() behave similarly
///     let tcp_l = TcpListener::bind("localhost:12345");
///
///     let mut udp_s = UdpSocket::bind(("127.0.0.1", 23451u16)).unwrap();
///     udp_s.send_to([7u8, 7u8, 7u8].as_slice(), (Ipv4Addr(127, 0, 0, 1), 23451u16));
/// }
/// ```
pub trait ToSocketAddrs {
    /// Converts this object to single socket address value.
    ///
    /// If more than one value is available, this method returns the first one.
    /// If no values are available, this method returns an `IoError`.
    ///
    /// By default this method delegates to `to_socket_addr_all` method, taking
    /// the first item from its result.
    fn to_socket_addrs(&self) -> io::Result<Vec<SocketAddr>>;
}

impl ToSocketAddrs for SocketAddr {
    #[inline]
    fn to_socket_addrs(&self) -> io::Result<Vec<SocketAddr>> { Ok(vec![*self]) }
}

impl ToSocketAddrs for (IpAddr, u16) {
    #[inline]
    fn to_socket_addrs(&self) -> io::Result<Vec<SocketAddr>> {
        let (ip, port) = *self;
        Ok(vec![SocketAddr { ip: ip, port: port }])
    }
}

fn resolve_socket_addr(s: &str, p: u16) -> io::Result<Vec<SocketAddr>> {
    let ips = try!(lookup_host(s));
    ips.map(|a| a.map(|a| SocketAddr { ip: a.ip, port: p })).collect()
}

impl<'a> ToSocketAddrs for (&'a str, u16) {
    fn to_socket_addrs(&self) -> io::Result<Vec<SocketAddr>> {
        let (host, port) = *self;

        // try to parse the host as a regular IpAddr first
        match host.parse().ok() {
            Some(addr) => return Ok(vec![SocketAddr {
                ip: addr,
                port: port
            }]),
            None => {}
        }

        resolve_socket_addr(host, port)
    }
}

// accepts strings like 'localhost:12345'
impl ToSocketAddrs for str {
    fn to_socket_addrs(&self) -> io::Result<Vec<SocketAddr>> {
        // try to parse as a regular SocketAddr first
        match self.parse().ok() {
            Some(addr) => return Ok(vec![addr]),
            None => {}
        }

        macro_rules! try_opt {
            ($e:expr, $msg:expr) => (
                match $e {
                    Some(r) => r,
                    None => return Err(io::Error::new(io::ErrorKind::InvalidInput,
                                                      $msg, None)),
                }
            )
        }

        // split the string by ':' and convert the second part to u16
        let mut parts_iter = self.rsplitn(2, ':');
        let port_str = try_opt!(parts_iter.next(), "invalid socket address");
        let host = try_opt!(parts_iter.next(), "invalid socket address");
        let port: u16 = try_opt!(port_str.parse().ok(), "invalid port value");
        resolve_socket_addr(host, port)
    }
}


impl<'a, T: ToSocketAddrs + ?Sized> ToSocketAddrs for &'a T {
    fn to_socket_addrs(&self) -> io::Result<Vec<SocketAddr>> {
        (**self).to_socket_addrs()
    }
}
