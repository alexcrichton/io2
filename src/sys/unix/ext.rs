// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Experimental extensions to `std` for Unix platforms.
//!
//! For now, this module is limited to extracting file descriptors,
//! but its functionality will grow over time.
//!
//! # Example
//!
//! ```rust,ignore
//! #![feature(globs)]
//!
//! use std::old_io::fs::File;
//! use std::os::unix::prelude::*;
//!
//! fn main() {
//!     let f = File::create(&Path::new("foo.txt")).unwrap();
//!     let fd = f.as_raw_fd();
//!
//!     // use fd with native unix bindings
//! }
//! ```

#![unstable]

use sys_common::AsInner;
use libc;

use old_io;

/// Raw file descriptors.
pub type Fd = libc::c_int;

/// Extract raw file descriptor
pub trait AsRawFd {
    /// Extract the raw file descriptor, without taking any ownership.
    fn as_raw_fd(&self) -> Fd;
}

impl AsRawFd for old_io::fs::File {
    fn as_raw_fd(&self) -> Fd {
        self.as_inner().fd()
    }
}

impl AsRawFd for old_io::pipe::PipeStream {
    fn as_raw_fd(&self) -> Fd {
        self.as_inner().fd()
    }
}

impl AsRawFd for old_io::net::pipe::UnixStream {
    fn as_raw_fd(&self) -> Fd {
        self.as_inner().fd()
    }
}

impl AsRawFd for old_io::net::pipe::UnixListener {
    fn as_raw_fd(&self) -> Fd {
        self.as_inner().fd()
    }
}

impl AsRawFd for old_io::net::pipe::UnixAcceptor {
    fn as_raw_fd(&self) -> Fd {
        self.as_inner().fd()
    }
}

impl AsRawFd for old_io::net::tcp::TcpStream {
    fn as_raw_fd(&self) -> Fd {
        self.as_inner().fd()
    }
}

impl AsRawFd for old_io::net::tcp::TcpListener {
    fn as_raw_fd(&self) -> Fd {
        self.as_inner().fd()
    }
}

impl AsRawFd for old_io::net::tcp::TcpAcceptor {
    fn as_raw_fd(&self) -> Fd {
        self.as_inner().fd()
    }
}

impl AsRawFd for old_io::net::udp::UdpSocket {
    fn as_raw_fd(&self) -> Fd {
        self.as_inner().fd()
    }
}

/// A prelude for conveniently writing platform-specific code.
///
/// Includes all extension traits, and some important type definitions.
pub mod prelude {
    pub use super::{Fd, AsRawFd};
}
