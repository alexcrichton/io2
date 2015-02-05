// Copyright 2013-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use prelude::v1::*;

use ffi::CString;
use io::{self, Error, ErrorKind};
use libc::{self, c_int, c_char, c_void, socklen_t};
use mem;
use net::{IpAddr, SocketAddr, Shutdown};
use num::Int;
use sys::c;
use sys::net::{cvt, cvt_r, cvt_gai, Socket, init, wrlen_t};
use sys_common::AsInner;

////////////////////////////////////////////////////////////////////////////////
// sockaddr and misc bindings
////////////////////////////////////////////////////////////////////////////////

fn hton<I: Int>(i: I) -> I { i.to_be() }
fn ntoh<I: Int>(i: I) -> I { Int::from_be(i) }

enum InAddr {
    V4(libc::in_addr),
    V6(libc::in6_addr),
}

fn ip_to_inaddr(ip: &IpAddr) -> InAddr {
    match *ip {
        IpAddr::V4(ref ip) => {
            let ip = ((ip.octets()[0] as u32) << 24) |
                     ((ip.octets()[1] as u32) << 16) |
                     ((ip.octets()[2] as u32) <<  8) |
                     ((ip.octets()[3] as u32) <<  0);
            InAddr::V4(libc::in_addr { s_addr: hton(ip) })
        }
        IpAddr::V6(ref ip) => {
            InAddr::V6(libc::in6_addr {
                s6_addr: [
                    hton(ip.segments()[0]),
                    hton(ip.segments()[1]),
                    hton(ip.segments()[2]),
                    hton(ip.segments()[3]),
                    hton(ip.segments()[4]),
                    hton(ip.segments()[5]),
                    hton(ip.segments()[6]),
                    hton(ip.segments()[7]),
                ]
            })
        }
    }
}

fn addr_to_sockaddr(addr: &SocketAddr,
                    storage: &libc::sockaddr_storage)
                    -> socklen_t {
    unsafe {
        let len = match ip_to_inaddr(&addr.ip) {
            InAddr::V4(inaddr) => {
                let storage = storage as *const _ as *mut libc::sockaddr_in;
                (*storage).sin_family = libc::AF_INET as libc::sa_family_t;
                (*storage).sin_port = hton(addr.port);
                (*storage).sin_addr = inaddr;
                mem::size_of::<libc::sockaddr_in>()
            }
            InAddr::V6(inaddr) => {
                let storage = storage as *const _ as *mut libc::sockaddr_in6;
                (*storage).sin6_family = libc::AF_INET6 as libc::sa_family_t;
                (*storage).sin6_port = hton(addr.port);
                (*storage).sin6_addr = inaddr;
                mem::size_of::<libc::sockaddr_in6>()
            }
        };
        return len as socklen_t;
    }
}

fn setsockopt<T>(sock: &Socket, opt: c_int, val: c_int,
                     payload: T) -> io::Result<()> {
    unsafe {
        let payload = &payload as *const T as *const c_void;
        try!(cvt(libc::setsockopt(*sock.as_inner(), opt, val, payload,
                                  mem::size_of::<T>() as socklen_t)));
        Ok(())
    }
}

#[allow(dead_code)]
fn getsockopt<T: Copy>(sock: &Socket, opt: c_int,
                           val: c_int) -> io::Result<T> {
    unsafe {
        let mut slot: T = mem::zeroed();
        let mut len = mem::size_of::<T>() as socklen_t;
        let ret = try!(cvt(c::getsockopt(*sock.as_inner(), opt, val,
                                         &mut slot as *mut _ as *mut _,
                                         &mut len)));
        assert_eq!(ret as usize, mem::size_of::<T>());
        Ok(slot)
    }
}

fn sockname<F>(f: F) -> io::Result<SocketAddr>
    where F: FnOnce(*mut libc::sockaddr, *mut socklen_t) -> c_int
{
    unsafe {
        let mut storage: libc::sockaddr_storage = mem::zeroed();
        let mut len = mem::size_of_val(&storage) as socklen_t;
        try!(cvt(f(&mut storage as *mut _ as *mut _, &mut len)));
        sockaddr_to_addr(&storage, len as usize)
    }
}

fn sockaddr_to_addr(storage: &libc::sockaddr_storage,
                    len: usize) -> io::Result<SocketAddr> {
    match storage.ss_family as libc::c_int {
        libc::AF_INET => {
            assert!(len as usize >= mem::size_of::<libc::sockaddr_in>());
            let storage: &libc::sockaddr_in = unsafe {
                mem::transmute(storage)
            };
            let ip = (storage.sin_addr.s_addr as u32).to_be();
            let a = (ip >> 24) as u8;
            let b = (ip >> 16) as u8;
            let c = (ip >>  8) as u8;
            let d = (ip >>  0) as u8;
            Ok(SocketAddr {
                ip: IpAddr::new_v4(a, b, c, d),
                port: ntoh(storage.sin_port),
            })
        }
        libc::AF_INET6 => {
            assert!(len as usize >= mem::size_of::<libc::sockaddr_in6>());
            let storage: &libc::sockaddr_in6 = unsafe {
                mem::transmute(storage)
            };
            let a = ntoh(storage.sin6_addr.s6_addr[0]);
            let b = ntoh(storage.sin6_addr.s6_addr[1]);
            let c = ntoh(storage.sin6_addr.s6_addr[2]);
            let d = ntoh(storage.sin6_addr.s6_addr[3]);
            let e = ntoh(storage.sin6_addr.s6_addr[4]);
            let f = ntoh(storage.sin6_addr.s6_addr[5]);
            let g = ntoh(storage.sin6_addr.s6_addr[6]);
            let h = ntoh(storage.sin6_addr.s6_addr[7]);
            Ok(SocketAddr {
                ip: IpAddr::new_v6(a, b, c, d, e, f, g, h),
                port: ntoh(storage.sin6_port),
            })
        }
        _ => {
            Err(Error::new(ErrorKind::InvalidInput, "invalid argument", None))
        }
    }
}

////////////////////////////////////////////////////////////////////////////////
// get_host_addresses
////////////////////////////////////////////////////////////////////////////////

extern "system" {
    fn getaddrinfo(node: *const c_char, service: *const c_char,
                   hints: *const libc::addrinfo,
                   res: *mut *mut libc::addrinfo) -> c_int;
    fn freeaddrinfo(res: *mut libc::addrinfo);
}

pub struct LookupHost {
    original: *mut libc::addrinfo,
    cur: *mut libc::addrinfo,
}

impl Iterator for LookupHost {
    type Item = io::Result<SocketAddr>;
    fn next(&mut self) -> Option<io::Result<SocketAddr>> {
        unsafe {
            if self.cur.is_null() { return None }
            let ret = sockaddr_to_addr(mem::transmute((*self.cur).ai_addr),
                                       (*self.cur).ai_addrlen as usize);
            self.cur = (*self.cur).ai_next as *mut libc::addrinfo;
            Some(ret)
        }
    }
}

impl Drop for LookupHost {
    fn drop(&mut self) {
        unsafe { freeaddrinfo(self.original) }
    }
}

pub fn lookup_host(host: &str) -> io::Result<LookupHost> {
    init();

    let c_host = CString::from_slice(host.as_bytes());
    let mut res = 0 as *mut _;
    unsafe {
        try!(cvt_gai(getaddrinfo(c_host.as_ptr(), 0 as *const _, 0 as *const _,
                                 &mut res)));
        Ok(LookupHost { original: res, cur: res })
    }
}

// ////////////////////////////////////////////////////////////////////////////////
// // get_address_name
// ////////////////////////////////////////////////////////////////////////////////
//
// extern "system" {
//     fn getnameinfo(sa: *const libc::sockaddr, salen: libc::socklen_t,
//         host: *mut c_char, hostlen: libc::size_t,
//         serv: *mut c_char, servlen: libc::size_t,
//         flags: c_int) -> c_int;
// }
//
// const NI_MAXHOST: uint = 1025;
//
// pub fn get_address_name(addr: IpAddr) -> Result<String, IoError> {
//     let addr = SocketAddr{ip: addr, port: 0};
//
//     let mut storage: libc::sockaddr_storage = unsafe { mem::zeroed() };
//     let len = addr_to_sockaddr(addr, &mut storage);
//
//     let mut hostbuf = [0 as c_char; NI_MAXHOST];
//
//     let res = unsafe {
//         getnameinfo(&storage as *const _ as *const libc::sockaddr, len,
//             hostbuf.as_mut_ptr(), NI_MAXHOST as libc::size_t,
//             ptr::null_mut(), 0,
//             0)
//     };
//
//     if res != 0 {
//         return Err(last_gai_error(res));
//     }
//
//     unsafe {
//         Ok(str::from_utf8(ffi::c_str_to_bytes(&hostbuf.as_ptr()))
//                .unwrap().to_string())
//     }
// }

////////////////////////////////////////////////////////////////////////////////
// TCP streams
////////////////////////////////////////////////////////////////////////////////

pub struct TcpStream {
    inner: Socket,
}

impl TcpStream {
    pub fn connect(addr: &SocketAddr) -> io::Result<TcpStream> {
        init();

        let sock = try!(Socket::new(addr, libc::SOCK_STREAM));

        let mut storage = unsafe { mem::zeroed() };
        let len = addr_to_sockaddr(addr, &mut storage);
        let addrp = &storage as *const _ as *const libc::sockaddr;

        try!(cvt_r(|| unsafe { libc::connect(*sock.as_inner(), addrp, len) }));
        Ok(TcpStream { inner: sock })
    }

    pub fn set_nodelay(&self, nodelay: bool) -> io::Result<()> {
        setsockopt(&self.inner, libc::IPPROTO_TCP, libc::TCP_NODELAY,
                   nodelay as c_int)
    }

    pub fn set_keepalive(&self, seconds: Option<u32>) -> io::Result<()> {
        let ret = setsockopt(&self.inner, libc::SOL_SOCKET, libc::SO_KEEPALIVE,
                             seconds.is_some() as c_int);
        match seconds {
            Some(n) => ret.and_then(|()| self.set_tcp_keepalive(n)),
            None => ret,
        }
    }

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    fn set_tcp_keepalive(&self, seconds: u32) -> io::Result<()> {
        setsockopt(&self.inner, libc::IPPROTO_TCP, libc::TCP_KEEPALIVE,
                   seconds as c_int)
    }
    #[cfg(any(target_os = "freebsd", target_os = "dragonfly"))]
    fn set_tcp_keepalive(&self, seconds: u32) -> io::Result<()> {
        setsockopt(&self.inner, libc::IPPROTO_TCP, libc::TCP_KEEPIDLE,
                   seconds as c_int)
    }
    #[cfg(not(any(target_os = "macos",
                  target_os = "ios",
                  target_os = "freebsd",
                  target_os = "dragonfly")))]
    fn set_tcp_keepalive(&self, _seconds: u32) -> io::Result<()> {
        Ok(())
    }

    pub fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }

    pub fn write(&self, buf: &[u8]) -> io::Result<usize> {
        let ret = try!(cvt(unsafe {
            libc::send(*self.inner.as_inner(),
                       buf.as_ptr() as *const c_void,
                       buf.len() as wrlen_t,
                       0)
        }));
        Ok(ret as usize)
    }

    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        sockname(|buf, len| unsafe {
            libc::getpeername(*self.inner.as_inner(), buf, len)
        })
    }

    pub fn socket_addr(&self) -> io::Result<SocketAddr> {
        sockname(|buf, len| unsafe {
            libc::getsockname(*self.inner.as_inner(), buf, len)
        })
    }

    pub fn shutdown(&self, how: Shutdown) -> io::Result<()> {
        use libc::consts::os::bsd44::SHUT_RDWR;

        let how = match how {
            Shutdown::Write => libc::SHUT_WR,
            Shutdown::Read => libc::SHUT_RD,
            Shutdown::Both => SHUT_RDWR,
        };
        try!(cvt(unsafe { libc::shutdown(*self.inner.as_inner(), how) }));
        Ok(())
    }

    pub fn duplicate(&self) -> io::Result<TcpStream> {
        self.inner.duplicate().map(|s| TcpStream { inner: s })
    }
}

////////////////////////////////////////////////////////////////////////////////
// TCP listeners
////////////////////////////////////////////////////////////////////////////////

pub struct TcpListener {
    inner: Socket,
}

impl TcpListener {
    pub fn bind(addr: &SocketAddr) -> io::Result<TcpListener> {
        init();

        let sock = try!(Socket::new(addr, libc::SOCK_STREAM));

        // On platforms with Berkeley-derived sockets, this allows
        // to quickly rebind a socket, without needing to wait for
        // the OS to clean up the previous one.
        if !cfg!(windows) {
            try!(setsockopt(&sock, libc::SOL_SOCKET, libc::SO_REUSEADDR,
                            1 as c_int));
        }

        // Bind our new socket
        let mut storage = unsafe { mem::zeroed() };
        let len = addr_to_sockaddr(addr, &mut storage);
        let addrp = &storage as *const _ as *const libc::sockaddr;
        try!(cvt(unsafe { libc::bind(*sock.as_inner(), addrp, len) }));

        // Start listening
        try!(cvt(unsafe { libc::listen(*sock.as_inner(), 128 as c_int) }));
        Ok(TcpListener { inner: sock })
    }

    pub fn socket_addr(&self) -> io::Result<SocketAddr> {
        sockname(|buf, len| unsafe {
            libc::getsockname(*self.inner.as_inner(), buf, len)
        })
    }

    pub fn accept(&self) -> io::Result<(TcpStream, SocketAddr)> {
        let mut storage: libc::sockaddr_storage = unsafe { mem::zeroed() };
        let mut len = mem::size_of_val(&storage) as socklen_t;
        let sock = try!(self.inner.accept(&mut storage as *mut _ as *mut _,
                                          &mut len));
        let addr = try!(sockaddr_to_addr(&storage, len as usize));
        Ok((TcpStream { inner: sock, }, addr))
    }

    pub fn duplicate(&self) -> io::Result<TcpListener> {
        self.inner.duplicate().map(|s| TcpListener { inner: s })
    }
}

////////////////////////////////////////////////////////////////////////////////
// UDP
////////////////////////////////////////////////////////////////////////////////

pub struct UdpSocket {
    inner: Socket,
}

impl UdpSocket {
    pub fn bind(addr: &SocketAddr) -> io::Result<UdpSocket> {
        init();

        let sock = try!(Socket::new(addr, libc::SOCK_DGRAM));

        let mut storage = unsafe { mem::zeroed() };
        let len = addr_to_sockaddr(addr, &mut storage);
        let addrp = &storage as *const _ as *const libc::sockaddr;

        try!(cvt(unsafe { libc::bind(*sock.as_inner(), addrp, len) }));
        Ok(UdpSocket { inner: sock })
    }

    pub fn socket_addr(&self) -> io::Result<SocketAddr> {
        sockname(|buf, len| unsafe {
            libc::getsockname(*self.inner.as_inner(), buf, len)
        })
    }

    pub fn recv_from(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        let mut storage: libc::sockaddr_storage = unsafe { mem::zeroed() };
        let mut addrlen = mem::size_of_val(&storage) as socklen_t;

        let n = try!(cvt(unsafe {
            libc::recvfrom(*self.inner.as_inner(),
                           buf.as_mut_ptr() as *mut c_void,
                           buf.len() as wrlen_t, 0,
                           &mut storage as *mut _ as *mut _, &mut addrlen)
        }));
        Ok((n as usize, try!(sockaddr_to_addr(&storage, addrlen as usize))))
    }

    pub fn send_to(&self, buf: &[u8], dst: &SocketAddr) -> io::Result<usize> {
        let mut storage = unsafe { mem::zeroed() };
        let dstlen = addr_to_sockaddr(dst, &mut storage);
        let dstp = &storage as *const _ as *const libc::sockaddr;

        let ret = try!(cvt(unsafe {
            libc::sendto(*self.inner.as_inner(),
                         buf.as_ptr() as *const c_void, buf.len() as wrlen_t,
                         0, dstp, dstlen)
        }));
        Ok(ret as usize)
    }

    pub fn set_broadcast(&self, on: bool) -> io::Result<()> {
        setsockopt(&self.inner, libc::SOL_SOCKET, libc::SO_BROADCAST,
                   on as c_int)
    }

    pub fn set_multicast_loop(&self, on: bool) -> io::Result<()> {
        setsockopt(&self.inner, libc::IPPROTO_IP,
                   libc::IP_MULTICAST_LOOP, on as c_int)
    }

    pub fn join_multicast(&self, multi: &IpAddr) -> io::Result<()> {
        match *multi {
            IpAddr::V4(..) => {
                self.set_membership(multi, libc::IP_ADD_MEMBERSHIP)
            }
            IpAddr::V6(..) => {
                self.set_membership(multi, libc::IPV6_ADD_MEMBERSHIP)
            }
        }
    }
    pub fn leave_multicast(&self, multi: &IpAddr) -> io::Result<()> {
        match *multi {
            IpAddr::V4(..) => {
                self.set_membership(multi, libc::IP_DROP_MEMBERSHIP)
            }
            IpAddr::V6(..) => {
                self.set_membership(multi, libc::IPV6_DROP_MEMBERSHIP)
            }
        }
    }
    fn set_membership(&self, addr: &IpAddr, opt: c_int) -> io::Result<()> {
        match ip_to_inaddr(addr) {
            InAddr::V4(addr) => {
                let mreq = libc::ip_mreq {
                    imr_multiaddr: addr,
                    // interface == INADDR_ANY
                    imr_interface: libc::in_addr { s_addr: 0x0 },
                };
                setsockopt(&self.inner, libc::IPPROTO_IP, opt, mreq)
            }
            InAddr::V6(addr) => {
                let mreq = libc::ip6_mreq {
                    ipv6mr_multiaddr: addr,
                    ipv6mr_interface: 0,
                };
                setsockopt(&self.inner, libc::IPPROTO_IPV6, opt, mreq)
            }
        }
    }

    pub fn multicast_time_to_live(&self, ttl: i32) -> io::Result<()> {
        setsockopt(&self.inner, libc::IPPROTO_IP, libc::IP_MULTICAST_TTL,
                   ttl as c_int)
    }

    pub fn time_to_live(&self, ttl: i32) -> io::Result<()> {
        setsockopt(&self.inner, libc::IPPROTO_IP, libc::IP_TTL, ttl as c_int)
    }

    pub fn duplicate(&self) -> io::Result<UdpSocket> {
        self.inner.duplicate().map(|s| UdpSocket { inner: s })
    }
}
