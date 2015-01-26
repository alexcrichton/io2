// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![allow(missing_copy_implementations)]

use prelude::v1::*;

use io::{self, Read, Write, WriteExt};

/// Copies the entire contents of a reader into a writer.
///
/// This function will continuously read data from `r` and then write it into
/// `w` in a streaming fashion until `r` returns EOF.
///
/// On success the total number of bytes that were copied from `r` to `w` is
/// returned.
///
/// # Errors
///
/// This function will return an error immediately if any call to `read` or
/// `write` returns an error.
#[unstable = "this function will discard intermediate data"]
pub fn copy<R: Read, W: Write>(r: &mut R, w: &mut W) -> io::Result<u64> {
    let mut buf = [0; 64 * 1024];
    let mut written = 0;
    loop {
        let len = match try!(r.read(&mut buf)) {
            0 => return Ok(written),
            len => len,
        };
        try!(w.write_all(&buf[..len]));
        written += len as u64;
    }
}

/// A reader which is always at EOF.
pub struct Empty { _priv: () }

/// Creates an instance of an empty reader.
///
/// All reads from the returned reader will return `Ok(0)`.
pub fn empty() -> Empty { Empty { _priv: () } }

impl Read for Empty {
    fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> { Ok(0) }
}

/// A reader which infinitely yields one byte.
pub struct Repeat { byte: u8 }

/// Creates an instance of a reader that infinitely repeats one byte.
///
/// All reads from this reader will succeed by filling the specified buffer with
/// the given byte.
pub fn repeat(byte: u8) -> Repeat { Repeat { byte: byte } }

impl Read for Repeat {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        for slot in buf.iter_mut() {
            *slot = self.byte;
        }
        Ok(buf.len())
    }
}

/// A writer which will move data into the void.
pub struct Sink { _priv: () }

/// Creates an instance of a writer which will successfully consume all data.
///
/// All calls to `write` on the returned instance will return `Ok(buf.len())`
/// and the contents of the buffer will not be inspected.
pub fn sink() -> Sink { Sink { _priv: () } }

impl Write for Sink {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> { Ok(buf.len()) }
}
