use prelude::v1::*;

use ffi;
use io;
use libc::{self, c_int, size_t};
use str;
use sys::c;
use net::{SocketAddr, IpAddr};
use sys::fd::FileDesc;
use sys_common::AsInner;

pub use sys::{cvt, cvt_r};

pub type wrlen_t = size_t;

pub struct Socket(FileDesc);

pub fn init() {}

pub fn cvt_gai(err: c_int) -> io::Result<()> {
    if err == 0 { return Ok(()) }

    let detail = unsafe {
        str::from_utf8(ffi::c_str_to_bytes(&c::gai_strerror(err))).unwrap()
            .to_string()
    };
    Err(io::Error::new(io::ErrorKind::Other,
                       "failed to lookup address information", Some(detail)))
}

impl Socket {
    pub fn new(addr: &SocketAddr, ty: c_int) -> io::Result<Socket> {
        let fam = match addr.ip {
            IpAddr::V4(..) => libc::AF_INET,
            IpAddr::V6(..) => libc::AF_INET6,
        };
        unsafe {
            let fd = try!(cvt(libc::socket(fam, ty, 0)));
            Ok(Socket(FileDesc::new(fd)))
        }
    }

    pub fn accept(&self, storage: *mut libc::sockaddr,
                  len: *mut libc::socklen_t) -> io::Result<Socket> {
        let fd = try!(cvt_r(|| unsafe {
            libc::accept(self.0.raw(), storage, len)
        }));
        Ok(Socket(FileDesc::new(fd)))
    }

    pub fn duplicate(&self) -> io::Result<Socket> {
        cvt(unsafe { libc::dup(self.0.raw()) }).map(|fd| {
            Socket(FileDesc::new(fd))
        })
    }

    pub fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf)
    }
}

impl AsInner<c_int> for Socket {
    fn as_inner(&self) -> &c_int { self.0.as_inner() }
}
