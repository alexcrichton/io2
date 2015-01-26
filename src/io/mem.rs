// Copyright 2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![allow(missing_copy_implementations)]

use core::prelude::*;

use io::{self, Read, BufferedRead, Write, SeekPos, Error, ErrorKind};
use iter::repeat;
use slice;
use vec::Vec;

/// A `Cursor` is a type which wraps another I/O object to provide a `Seek`
/// implementation.
///
/// Cursors are currently typically used with memory buffer objects in order to
/// allow `Seek` plus `Read` and `Write` implementations. For example, common
/// cursor types include:
///
/// * `Cursor<Vec<u8>>`
/// * `Cursor<&[u8]>`
///
/// Cursors are not currently generic over the type contained within, but may
/// become so.
pub struct Cursor<T> {
    pos: u64,
    inner: T,
}

impl<T> Cursor<T> {
    /// Create a new cursor wrapping the provided underlying I/O object.
    pub fn new(inner: T) -> Cursor<T> {
        Cursor { pos: 0, inner: inner }
    }

    /// Consume this cursor, returning the underlying value.
    pub fn into_inner(self) -> T { self.inner }

    /// Get a reference to the underlying value in this cursor.
    pub fn get_ref(&self) -> &T { &self.inner }

    /// Get a mutable reference to the underlying value in this cursor.
    ///
    /// Care should be taken to avoid modifying the internal I/O state of the
    /// underlying value as it may corrupt this cursor's position.
    pub fn get_mut(&mut self) -> &mut T { &mut self.inner }

    /// Returns the current value of this cursor
    pub fn position(&self) -> u64 { self.pos }

    /// Sets the value of this cursor
    pub fn set_position(&mut self, pos: u64) { self.pos = pos; }
}

macro_rules! seek {
    () => {
        fn seek(&mut self, style: SeekPos) -> io::Result<u64> {
            let pos = match style {
                SeekPos::FromStart(n) => n as i64,
                SeekPos::FromEnd(n) => self.inner.len() as i64 + n,
                SeekPos::FromCur(n) => self.pos as i64 + n,
            };

            if pos < 0 {
                Err(Error::new(ErrorKind::InvalidInput,
                               "invalid seek to a negative position",
                               None))
            } else {
                self.pos = pos as u64;
                Ok(self.pos)
            }
        }
    }
}

impl<'a> io::Seek for Cursor<&'a [u8]> { seek!(); }
impl<'a> io::Seek for Cursor<&'a mut [u8]> { seek!(); }
impl io::Seek for Cursor<Vec<u8>> { seek!(); }

macro_rules! read {
    () => {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            if self.pos > self.inner.len() as u64 { return Ok(0) }
            let mut slice = &self.inner[(self.pos as usize)..];
            let n = try!(slice.read(buf));
            self.pos += n as u64;
            Ok(n)
        }
    }
}

impl<'a> Read for Cursor<&'a [u8]> { read!(); }
impl<'a> Read for Cursor<&'a mut [u8]> { read!(); }
impl Read for Cursor<Vec<u8>> { read!(); }

macro_rules! buffer {
    () => {
        fn fill_buf(&mut self) -> io::Result<&[u8]> {
            if self.pos < (self.inner.len() as u64) {
                Ok(&self.inner[(self.pos as usize)..])
            } else {
                Ok(&[])
            }
        }
        fn consume(&mut self, amt: usize) { self.pos += amt as u64; }
    }
}

impl<'a> BufferedRead for Cursor<&'a [u8]> { buffer!(); }
impl<'a> BufferedRead for Cursor<&'a mut [u8]> { buffer!(); }
impl<'a> BufferedRead for Cursor<Vec<u8>> { buffer!(); }

impl<'a> Write for Cursor<&'a mut [u8]> {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        if self.pos >= self.inner.len() as u64 { return Ok(0) }

        let amt = {
            let mut s = &mut self.inner[(self.pos as usize)..];
            try!(s.write(data))
        };
        self.pos += amt as u64;
        Ok(amt)
    }
}
impl Write for Cursor<Vec<u8>> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let pos = self.position();
        let mut len = self.get_ref().len();
        if pos == len as u64 {
            self.get_mut().push_all(buf)
        } else {
            // Make sure the internal buffer is as least as big as where we
            // currently are
            let difference = pos as i64 - len as i64;
            if difference > 0 {
                self.get_mut().extend(repeat(0).take(difference as usize));
                len += difference as usize;
            }

            // Figure out what bytes will be used to overwrite what's currently
            // there (left), and what will be appended on the end (right)
            let cap = len - (pos as usize);
            let (left, right) = if cap <= buf.len() {
                (&buf[..cap], &buf[cap..])
            } else {
                let result: (_, &[_]) = (buf, &[]);
                result
            };

            // Do the necessary writes
            if left.len() > 0 {
                let dst = &mut self.get_mut()[(pos as usize)..];
                slice::bytes::copy_memory(dst, left);
            }
            if right.len() > 0 {
                self.get_mut().push_all(right);
            }
        }

        // Bump us forward
        self.set_position(pos + buf.len() as u64);
        Ok(buf.len())
    }
}
