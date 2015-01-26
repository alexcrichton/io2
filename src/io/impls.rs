// Copyright 2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use core::prelude::*;

use borrow::ByRef;
use cmp;
use io::{self, SeekPos, Read, Write, Seek, BufferedRead};
use ptr;
use slice;
use vec::Vec;

// =============================================================================
// Forwarding implementations

impl<'a, R: Read> Read for ByRef<'a, R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> { self.inner.read(buf) }
}
impl<'a, R: Read> Read for &'a mut R {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> { (**self).read(buf) }
}

impl<'a, W: Write> Write for ByRef<'a, W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> { self.inner.write(buf) }
    fn flush(&mut self) -> io::Result<()> { self.inner.flush() }
}
impl<'a, W: Write> Write for &'a mut W {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> { (**self).write(buf) }
    fn flush(&mut self) -> io::Result<()> { (**self).flush() }
}

impl<'a, S: Seek> Seek for ByRef<'a, S> {
    fn seek(&mut self, pos: SeekPos) -> io::Result<u64> { self.inner.seek(pos) }
}
impl<'a, S: Seek> Seek for &'a mut S {
    fn seek(&mut self, pos: SeekPos) -> io::Result<u64> { (**self).seek(pos) }
}

// =============================================================================
// In-memory buffer implementations

impl<'a> Read for &'a [u8] {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let amt = cmp::min(buf.len(), self.len());
        slice::bytes::copy_memory(buf, &self[..amt]);
        *self = &self[amt..];
        Ok(amt)
    }
}

impl<'a> BufferedRead for &'a [u8] {
    fn fill_buf(&mut self) -> io::Result<&[u8]> { Ok(*self) }
    fn consume(&mut self, amt: usize) { *self = &self[amt..]; }
}

impl<'a> Write for &'a mut [u8] {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        let amt = cmp::min(data.len(), self.len());
        slice::bytes::copy_memory(*self, &data[..amt]);
        // TODO: is this actually safe?
        unsafe {
            let other = ptr::read(self);
            *self = &mut other[amt..];
        }
        Ok(amt)
    }
}

impl Write for Vec<u8> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.push_all(buf);
        Ok(buf.len())
    }
}

