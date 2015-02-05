// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![allow(missing_docs)]
#![allow(non_camel_case_types)]
#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unused_unsafe)]
#![allow(unused_mut)]

use prelude::v1::*;

use ffi;
use io::{self, ErrorKind};
use libc;
use num::{Int, SignedInt};
use num;
use str;

macro_rules! call {
    ($e:expr) => {
        match $e {
            n if n < 0 => Err(::io::Error::last_os_error()),
            n => Ok(n),
        }
    }
}

// macro_rules! helper_init { (static $name:ident: Helper<$m:ty>) => (
//     static $name: Helper<$m> = Helper {
//         lock: ::sync::MUTEX_INIT,
//         cond: ::sync::CONDVAR_INIT,
//         chan: ::cell::UnsafeCell { value: 0 as *mut Sender<$m> },
//         signal: ::cell::UnsafeCell { value: 0 },
//         initialized: ::cell::UnsafeCell { value: false },
//         shutdown: ::cell::UnsafeCell { value: false },
//     };
// ) }
//
pub mod c;
pub mod ext;
pub mod fd;
pub mod fs;
// pub mod helper_signal;
pub mod os;
pub mod net;
// pub mod pipe;
// pub mod process;
// pub mod tcp;
// pub mod timer;
// pub mod tty;
// pub mod udp;
//
// pub mod addrinfo {
//     pub use sys_common::net::get_host_addresses;
//     pub use sys_common::net::get_address_name;
// }
//
// pub fn last_net_error() -> IoError {
//     last_error()
// }
//
// extern "system" {
//     fn gai_strerror(errcode: libc::c_int) -> *const libc::c_char;
// }
//
// pub fn last_gai_error(s: libc::c_int) -> IoError {
//
//     let mut err = decode_error(s);
//     err.detail = Some(unsafe {
//         str::from_utf8(ffi::c_str_to_bytes(&gai_strerror(s))).unwrap().to_string()
//     });
//     err
// }

pub fn decode_error_kind(errno: i32) -> ErrorKind {
    match errno as libc::c_int {
        libc::EOF => ErrorKind::EndOfFile,
        libc::ECONNREFUSED => ErrorKind::ConnectionRefused,
        libc::ECONNRESET => ErrorKind::ConnectionReset,
        libc::EPERM | libc::EACCES => ErrorKind::PermissionDenied,
        libc::EPIPE => ErrorKind::BrokenPipe,
        libc::ENOTCONN => ErrorKind::NotConnected,
        libc::ECONNABORTED => ErrorKind::ConnectionAborted,
        libc::EADDRNOTAVAIL => ErrorKind::ConnectionRefused,
        libc::EADDRINUSE => ErrorKind::ConnectionRefused,
        libc::ENOENT => ErrorKind::FileNotFound,
        libc::EISDIR => ErrorKind::InvalidInput,
        libc::EINVAL => ErrorKind::InvalidInput,
        libc::ENOTTY => ErrorKind::MismatchedFileTypeForOperation,
        libc::ETIMEDOUT => ErrorKind::TimedOut,
        libc::ECANCELED => ErrorKind::TimedOut,
        libc::consts::os::posix88::EEXIST => ErrorKind::PathAlreadyExists,

        // These two constants can have the same value on some systems,
        // but different values on others, so we can't use a match
        // clause
        x if x == libc::EAGAIN || x == libc::EWOULDBLOCK =>
            ErrorKind::ResourceUnavailable,

        _ => ErrorKind::Other,
    }
}

// pub fn retry<T, F> (mut f: F) -> T where
//     T: SignedInt,
//     F: FnMut() -> T,
// {
//     let one: T = Int::one();
//     loop {
//         let n = f();
//         if n == -one && os::errno() == libc::EINTR as int { }
//         else { return n }
//     }
// }

pub fn ms_to_timeval(ms: u64) -> libc::timeval {
    libc::timeval {
        tv_sec: (ms / 1000) as libc::time_t,
        tv_usec: ((ms % 1000) * 1000) as libc::suseconds_t,
    }
}

// pub fn wouldblock() -> bool {
//     let err = os::errno();
//     err == libc::EWOULDBLOCK as int || err == libc::EAGAIN as int
// }
//
// pub fn set_nonblocking(fd: sock_t, nb: bool) -> IoResult<()> {
//     let set = nb as libc::c_int;
//     mkerr_libc(retry(|| unsafe { c::ioctl(fd, c::FIONBIO, &set) }))
// }

pub fn cvt<T: SignedInt>(t: T) -> io::Result<T> {
    let one: T = Int::one();
    if t == -one {
        Err(io::Error::last_os_error())
    } else {
        Ok(t)
    }
}

pub fn cvt_r<T, F>(mut f: F) -> io::Result<T>
    where T: SignedInt, F: FnMut() -> T
{
    loop {
        match cvt(f()) {
            Err(ref e) if e.kind() == ErrorKind::Interrupted => {}
            other => return other,
        }
    }
}

