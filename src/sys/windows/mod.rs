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

use prelude::v1::*;

use io::{self, ErrorKind, Error};
use libc;
use iter::repeat;

// use mem;
// use num;
// use old_io::{self, IoResult, IoError};
// use sync::{Once, ONCE_INIT};
//
// macro_rules! helper_init { (static $name:ident: Helper<$m:ty>) => (
//     static $name: Helper<$m> = Helper {
//         lock: ::sync::MUTEX_INIT,
//         cond: ::sync::CONDVAR_INIT,
//         chan: ::cell::UnsafeCell { value: 0 as *mut ::sync::mpsc::Sender<$m> },
//         signal: ::cell::UnsafeCell { value: 0 },
//         initialized: ::cell::UnsafeCell { value: false },
//         shutdown: ::cell::UnsafeCell { value: false },
//     };
// ) }

macro_rules! call {
    ($e:expr) => {
        match $e {
            0 => Err(::io::Error::last_os_error()),
            n => Ok(n),
        }
    }
}

pub mod c;
// pub mod ext;
pub mod fs;
pub mod handle;
// pub mod helper_signal;
// pub mod os;
// pub mod pipe;
// pub mod process;
// pub mod tcp;
// pub mod timer;
// pub mod tty;
// pub mod udp;

// pub mod addrinfo {
//     pub use sys_common::net::get_host_addresses;
//     pub use sys_common::net::get_address_name;
// }
//
// pub fn last_error() -> IoError {
//     let errno = os::errno() as i32;
//     let mut err = decode_error(errno);
//     err.detail = Some(os::error_string(errno));
//     err
// }
//
// pub fn last_net_error() -> IoError {
//     let errno = unsafe { c::WSAGetLastError() as i32 };
//     let mut err = decode_error(errno);
//     err.detail = Some(os::error_string(errno));
//     err
// }

pub fn decode_error_kind(errno: u32) -> ErrorKind {
    match errno as libc::c_int {
        libc::EOF => ErrorKind::EndOfFile,

        libc::ERROR_ACCESS_DENIED => ErrorKind::PermissionDenied,
        libc::ERROR_ALREADY_EXISTS => ErrorKind::PathAlreadyExists,
        libc::ERROR_BROKEN_PIPE => ErrorKind::BrokenPipe,
        libc::ERROR_FILE_NOT_FOUND => ErrorKind::FileNotFound,
        libc::ERROR_INVALID_FUNCTION => ErrorKind::InvalidInput,
        libc::ERROR_INVALID_HANDLE => ErrorKind::MismatchedFileTypeForOperation,
        libc::ERROR_INVALID_NAME => ErrorKind::InvalidInput,
        libc::ERROR_NOTHING_TO_TERMINATE => ErrorKind::InvalidInput,
        libc::ERROR_NO_DATA => ErrorKind::BrokenPipe,
        libc::ERROR_OPERATION_ABORTED => ErrorKind::TimedOut,

        libc::WSAEACCES => ErrorKind::PermissionDenied,
        libc::WSAEADDRINUSE => ErrorKind::ConnectionRefused,
        libc::WSAEADDRNOTAVAIL => ErrorKind::ConnectionRefused,
        libc::WSAECONNABORTED => ErrorKind::ConnectionAborted,
        libc::WSAECONNREFUSED => ErrorKind::ConnectionRefused,
        libc::WSAECONNRESET => ErrorKind::ConnectionReset,
        libc::WSAEINVAL => ErrorKind::InvalidInput,
        libc::WSAENOTCONN => ErrorKind::NotConnected,
        libc::WSAEWOULDBLOCK => ErrorKind::ResourceUnavailable,

        _ => ErrorKind::Other,
    }
}

// pub fn ms_to_timeval(ms: u64) -> libc::timeval {
//     libc::timeval {
//         tv_sec: (ms / 1000) as libc::c_long,
//         tv_usec: ((ms % 1000) * 1000) as libc::c_long,
//     }
// }
//
// pub fn wouldblock() -> bool {
//     let err = os::errno();
//     err == libc::WSAEWOULDBLOCK as uint
// }
//
// pub fn set_nonblocking(fd: sock_t, nb: bool) -> IoResult<()> {
//     let mut set = nb as libc::c_ulong;
//     if unsafe { c::ioctlsocket(fd, c::FIONBIO, &mut set) != 0 } {
//         Err(last_error())
//     } else {
//         Ok(())
//     }
// }
//
// pub fn init_net() {
//     unsafe {
//         static START: Once = ONCE_INIT;
//
//         START.call_once(|| {
//             let mut data: c::WSADATA = mem::zeroed();
//             let ret = c::WSAStartup(0x202, // version 2.2
//                                     &mut data);
//             assert_eq!(ret, 0);
//         });
//     }
// }

fn to_utf16(s: Option<&str>) -> io::Result<Vec<u16>> {
    match s {
        Some(s) => Ok({
            let mut s = s.utf16_units().collect::<Vec<u16>>();
            s.push(0);
            s
        }),
        None => Err(Error::new(ErrorKind::InvalidInput,
                               "valid unicode input required", None)),
    }
}

fn fill_utf16_buf_and_decode<F>(mut f: F) -> io::Result<Vec<u16>> where
    F: FnMut(*mut u16, libc::DWORD) -> libc::DWORD,
{
    unsafe {
        let mut n = 128;
        loop {
            let mut buf: Vec<u16> = repeat(0u16).take(n as usize).collect();
            let k = try!(call!(f(buf.as_mut_ptr(), n)));
            if k == n && libc::GetLastError() ==
                            libc::ERROR_INSUFFICIENT_BUFFER as libc::DWORD {
                n *= 2;
            } else if k >= n {
                n = k;
            } else {
                buf.truncate(k as usize);
                return Ok(buf)
            }
        }
    }
}

fn truncate_utf16_at_nul<'a>(v: &'a [u16]) -> &'a [u16] {
    match v.iter().position(|c| *c == 0) {
        // don't include the 0
        Some(i) => &v[..i],
        None => v
    }
}

