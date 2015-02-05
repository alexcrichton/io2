// Copyright 2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Traits, helpers, and type definitions for core I/O functionality.
//!
//! > **NOTE**: This module is very much a work in progress and is under active
//! > development. At this time it is still recommended to use the `old_io`
//! > module while the details of this module shake out.

#![unstable = "this new I/O module is still under active deveopment and APIs \
               are subject to tweaks fairly regularly"]

use borrow::ByRef;
use cmp;
use unicode::str as core_str;
use error::Error as StdError;
use fmt;
use iter::Iterator;
use marker::Sized;
use mem as stdmem;
use option::Option::{self, Some, None};
use ptr::PtrExt;
use result::Result::{Ok, Err};
use result;
use slice::{self, SliceExt};
use str::{self, StrExt};
use vec::Vec;

pub use self::util::{copy, sink, Sink, empty, Empty, repeat, Repeat};
pub use self::mem::Cursor;
pub use self::error::{Result, Error, ErrorKind};

pub mod prelude;
mod error;
mod impls;
mod mem;
mod util;

const DEFAULT_BUF_SIZE: usize = 64 * 1024;

/// A trait for objects which are byte-oriented sources.
///
/// Readers are defined by one method, `read`. Each call to `read` will attempt
/// to pull bytes from this source into a provided buffer.
///
/// Readers are intended to be composable with one another. Many objects
/// throughout the I/O and related libraries take and provide types which
/// implement the `Read` trait.
pub trait Read {
    /// Pull some bytes from this source into the specified buffer, returning
    /// how many bytes were read.
    ///
    /// This function does not provide any guarantees about whether it blocks
    /// waiting for data, but if an object needs to block for a read but cannot
    /// it will typically signal this via an `Err` return value.
    ///
    /// If the return value of this method is `Ok(n)`, then it must be
    /// guaranteed that `0 <= n <= buf.len()`. If `n` is `0` then it indicates
    /// that this reader as reached "end of file" and will likely no longer be
    /// able to produce more bytes, but this is not guaranteed, however. A
    /// nonzero `n` value indicates that the buffer `buf` has been filled in
    /// with `n` bytes of data from this source.
    ///
    /// No guarantees are provided about the contents of `buf` when this
    /// function is called, implementations cannot rely on any property of the
    /// contents of `buf` being true.
    ///
    /// # Errors
    ///
    /// If this function encounters any form of I/O or other error, an error
    /// variant will be returned. If an error is returned then it is guaranteed
    /// that no bytes were read successfully.
    fn read(&mut self, buf: &mut [u8]) -> Result<usize>;
}

/// Extension methods for all instances of `Read`, typically imported through
/// `std::io::prelude::*`.
pub trait ReadExt: Read + Sized {
    /// Read all remaining bytes in this source, placing them into `buf`.
    ///
    /// This function will continuously invoke `read` until `Ok(0)` or an error
    /// is reached, at which point this function will immediately return.
    ///
    /// This function will only successfully return if an invocation of `Ok(0)`
    /// succeeds at which point all bytes have been read into `buf`.
    ///
    /// # Errors
    ///
    /// If a read error is encountered then this function immediately returns.
    /// Any bytes which have already been read will be present in `buf`.
    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<()> {
        loop {
            let cur_len = buf.len();
            if buf.capacity() == cur_len {
                buf.reserve(DEFAULT_BUF_SIZE);
            }
            let n = {
                let buf = unsafe {
                    let base = buf.as_mut_ptr().offset(cur_len as isize);
                    let len = buf.capacity() - cur_len;
                    slice::from_raw_mut_buf(stdmem::copy_lifetime(buf, &base), len)
                };
                // Our buffer we're passing down to `read` is uninitialized data
                // (the end of a `Vec`) but the read operation will be *much*
                // faster if we don't have to zero it out. In order to prevent
                // LLVM from generating an `undef` value by reading from this
                // uninitialized memory, we force LLVM to think it's initialized
                // by sending it through a black box. This should prevent actual
                // undefined behavior after optimizations.
                black_box(&buf);
                try!(self.read(buf))
            };
            if n == 0 { return Ok(()) }
            unsafe { buf.set_len(cur_len + n) }
        }

        // Semi-hack used to prevent LLVM from retaining any assumptions about
        // `dummy` over this function call
        fn black_box<T>(dummy: T) {
            unsafe { asm!("" : : "r"(&dummy)) }
        }
    }

    /// Create a "by reference" adaptor for this instance of `Read`.
    ///
    /// The returned adaptor also implements `Read` and will simply borrow this
    /// current reader.
    fn by_ref(&mut self) -> ByRef<Self> {
        ByRef { inner: self }
    }

    /// Transform this `Read` instance to an `Iterator` over its bytes.
    ///
    /// The returned type implements `Iterator` where the `Item` is `Result<u8,
    /// R::Err>`.  The yielded item is `Ok` if a byte was successfully read and
    /// `Err` otherwise for I/O errors. EOF is mapped to returning `None` for
    /// this iterator.
    fn bytes(self) -> Bytes<Self> {
        Bytes { inner: self }
    }

    /// Transform this `Read` instance to an `Iterator` over `char`s.
    ///
    /// This adaptor will attempt to interpret this reader as an UTF-8 encoded
    /// sequence of characters. The returned iterator will return `None` once
    /// EOF is reached for this reader (and it's not in the middle of decoding a
    /// character). Otherwise each element yielded will be a `Result<char, E>`
    /// where `E` may contain information about what I/O error occurred or where
    /// decoding failed.
    ///
    /// Currently this adaptor will discard intermediate data read, and should
    /// be avoided if this is not desired.
    #[unstable = "the error semantics of the returned structure are uncertain"]
    fn chars(self) -> Chars<Self> {
        Chars { inner: self }
    }

    /// Create an adaptor which will chain this stream with another.
    ///
    /// The returned instance of `Read` will yield all this object's bytes
    /// until EOF is reached. Afterwards the bytes of `next` will be yielded
    /// infinitely.
    fn chain<R: Read>(self, next: R) -> Chain<Self, R> {
        Chain { first: self, second: next, done_first: false }
    }

    /// Create an adaptor which will read at most `limit` bytes from it.
    ///
    /// This function returns a new instance of `Read` which will read at most
    /// `limit` bytes, after which it will always return EOF (`Ok(0)`). Any
    /// read errors will not count towards the number of bytes read and future
    /// calls to `read` may succeed.
    fn take(self, limit: u64) -> Take<Self> {
        Take { inner: self, limit: limit }
    }

    /// Creates a reader adaptor which will write all read data into the given
    /// output stream.
    ///
    /// Whenever the returned `Read` instance is read it will write the read
    /// data to `out`. The current semantics of this implementation imply that
    /// a `write` error will not report how much data was initially read.
    #[unstable = "the error semantics of the returned structure are uncertain"]
    fn tee<W: Write>(self, out: W) -> Tee<Self, W> {
        Tee { reader: self, writer: out }
    }
}

impl<T: Read> ReadExt for T {}

/// A trait for objects which are byte-oriented sink.
///
/// Writers are defined by one method, `write`. This function will attempt to
/// write some data into the object, returning how many bytes were successfully
/// written.
///
/// Another commonly overridden method is the `flush` method for writers such as
/// buffered writers.
///
/// Writers are intended to be composable with one another. Many objects
/// throughout the I/O and related libraries take and provide types which
/// implement the `Write` trait.
pub trait Write {
    /// Write a buffer into this object, returning how many bytes were read.
    ///
    /// This function will attempt to write the entire contents of `buf`, but
    /// the entire write may not succeed, or the write may also generate an
    /// error. A call to `write` represents *at most one* attempt to write to
    /// any wrapped object.
    ///
    /// Calls to `write` are not guaranteed to block waiting for data to be
    /// written, and a write which would otherwise block is indicated through an
    /// `Err` variant.
    ///
    /// If the return value is `Ok(n)` then it must be guaranteed that
    /// `0 <= n <= buf.len()`. A return value of `0` typically means that the
    /// underlying object is no longer able to accept bytes and will likely not
    /// be able to in the future as well, or that the buffer provided is empty.
    ///
    /// # Errors
    ///
    /// Each call to `write` may generate an I/O error indicating that the
    /// operation could not be completed. If an error is returned then no bytes
    /// in the buffer were successfully written to this writer.
    ///
    /// It is **not** considered an error if the entire buffer could not be
    /// written to this writer.
    fn write(&mut self, buf: &[u8]) -> Result<usize>;

    /// Flush this output stream, ensuring that all intermediately buffered
    /// contents reach their destination.
    ///
    /// This is by default a no-op and implementers of the `Write` trait should
    /// decide whether their stream needs to be buffered or not.
    fn flush(&mut self) -> Result<()> { Ok(()) }
}

/// Extension methods for all instances of `Write`, typically imported through
/// `std::io::prelude::*`.
pub trait WriteExt: Write + Sized {
    /// Attempts to write an entire buffer into this write.
    ///
    /// This method will continuously call `write` while there is more data to
    /// write. This method will not return until the entire buffer has been
    /// successfully written or an error occurs. The first error generated from
    /// this method will be returned.
    ///
    /// # Errors
    ///
    /// This function will return the first error that `write` returns.
    #[unstable = "this function loses information about intermediate writes"]
    fn write_all(&mut self, mut buf: &[u8]) -> Result<()> {
        while buf.len() > 0 {
            let n = try!(self.write(buf));
            if n == 0 {
                return Err(Error::new(ErrorKind::EndOfFile,
                                      "failed to write whole buffer: eof reached",
                                      None))
            }
            buf = &buf[n..];
        }
        Ok(())
    }

    /// Writes a formatted string into this writer, returning any error
    /// encountered.
    ///
    /// This method is primarily used to interface with the `format_args!`
    /// macro, but it is rare that this should explicitly be called. The
    /// `write!` macro should be favored to invoke this method instead.
    ///
    /// This function internally uses the `write_all` method on this trait and
    /// hence will continuously write data so long as no errors are received.
    /// This also means that partial writes are not indicated in this signature.
    ///
    /// # Errors
    ///
    /// This function will return any I/O error reported while formatting.
    #[unstable = "this function loses information about intermediate writes"]
    fn write_fmt(&mut self, fmt: fmt::Arguments) -> Result<()> {
        // Create a shim which translates a Writer to a fmt::Writer and saves
        // off I/O errors. instead of discarding them
        struct Adaptor<'a, T: 'a> {
            inner: &'a mut T,
            error: Result<()>,
        }

        impl<'a, T: Write> fmt::Writer for Adaptor<'a, T> {
            fn write_str(&mut self, s: &str) -> fmt::Result {
                match self.inner.write_all(s.as_bytes()) {
                    Ok(()) => Ok(()),
                    Err(e) => {
                        self.error = Err(e);
                        Err(fmt::Error)
                    }
                }
            }
        }

        let mut output = Adaptor { inner: self, error: Ok(()) };
        match fmt::write(&mut output, fmt) {
            Ok(()) => Ok(()),
            Err(..) => output.error
        }
    }

    /// Create a "by reference" adaptor for this instance of `Write`.
    ///
    /// The returned adaptor also implements `Write` and will simply borrow this
    /// current writer.
    fn by_ref(&mut self) -> ByRef<Self> {
        ByRef { inner: self }
    }

    /// Creates a new writer which will write all data to both this writer and
    /// another writer.
    ///
    /// All data written to the returned writer will both be written to `self`
    /// as well as `other`. Note that the error semantics of the current
    /// implementation do not precisely track where errors happen. For example
    /// an error on the second call to `write` will not report that the first
    /// call to `write` succeeded.
    #[unstable = "the error semantics of the returned structure are uncertain"]
    fn broadcast<W: Write>(self, other: W) -> Broadcast<Self, W> {
        Broadcast { first: self, second: other }
    }
}

impl<T: Write> WriteExt for T {}

/// An object implementing `Seek` internally has some form of cursor which can
/// be moved within a stream of bytes.
///
/// The stream typically has a fixed size, allowing seeking relative to either
/// end or the current offset.
pub trait Seek {
    /// Seek to an offset, in bytes, in a stream
    ///
    /// A seek beyond the end of a stream is allowed, but seeking before offset
    /// 0 is an error.
    ///
    /// Seeking past the end of the stream does not modify the underlying
    /// stream, but the next write may cause the previous data to be filled in
    /// with a bit pattern.
    ///
    /// This method returns the new position within the stream if the seek
    /// operation completed successfully.
    ///
    /// # Errors
    ///
    /// Seeking to a negative offset is considered an error
    fn seek(&mut self, pos: SeekPos) -> Result<u64>;
}

/// Enumeration of possible methods to seek within an I/O object.
#[derive(Copy, PartialEq, Eq, Clone, Debug)]
pub enum SeekPos {
    /// Set the offset to the provided number of bytes.
    FromStart(u64),

    /// Set the offset to the size of this object plus the specified number of
    /// bytes.
    ///
    /// It is possible to seek beyond the end of an object, but is an error to
    /// seek before byte 0.
    FromEnd(i64),

    /// Set the offset to the current position plus the specified number of
    /// bytes.
    ///
    /// It is possible to seek beyond the end of an object, but is an error to
    /// seek before byte 0.
    FromCur(i64),
}

/// A Buffer is a type of reader which has some form of internal buffering to
/// allow certain kinds of reading operations to be more optimized than others.
///
/// This type extends the `Read` trait with a few methods that are not
/// possible to reasonably implement with purely a read interface.
pub trait BufferedRead: Read {
    /// Fills the internal buffer of this object, returning the buffer contents.
    ///
    /// Note that none of the contents will be "read" in the sense that later
    /// calling `read` may return the same contents.
    ///
    /// The `consume` function must be called with the number of bytes that are
    /// consumed from this buffer returned to ensure that the bytes are never
    /// returned twice.
    ///
    /// An empty buffer returned indicates that the stream has reached EOF.
    ///
    /// # Errors
    ///
    /// This function will return an I/O error if the underlying reader was
    /// read, but returned an error.
    fn fill_buf(&mut self) -> Result<&[u8]>;

    /// Tells this buffer that `amt` bytes have been consumed from the buffer,
    /// so they should no longer be returned in calls to `read`.
    fn consume(&mut self, amt: usize);
}

/// A `Write` adaptor which will write data to multiple locations.
///
/// For more information, see `WriteExt::broadcast`.
pub struct Broadcast<T, U> {
    first: T,
    second: U,
}

impl<T: Write, U: Write> Write for Broadcast<T, U> {
    fn write(&mut self, data: &[u8]) -> Result<usize> {
        let n = try!(self.first.write(data));
        // TODO: what if the write fails? (we wrote something)
        try!(self.second.write_all(&data[..n]));
        Ok(n)
    }

    fn flush(&mut self) -> Result<()> {
        self.first.flush().and(self.second.flush())
    }
}

/// Adaptor to chain together two instances of `Read`.
///
/// For more information, see `ReadExt::chain`.
pub struct Chain<T, U> {
    first: T,
    second: U,
    done_first: bool,
}

impl<T: Read, U: Read> Read for Chain<T, U> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        if !self.done_first {
            match try!(self.first.read(buf)) {
                0 => { self.done_first = true; }
                n => return Ok(n),
            }
        }
        self.second.read(buf)
    }
}

/// Reader adaptor which limits the bytes read from an underlying reader.
///
/// For more information, see `ReadExt::take`.
pub struct Take<T> {
    inner: T,
    limit: u64,
}

impl<T: Read> Read for Take<T> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let max = cmp::min(buf.len() as u64, self.limit) as usize;
        let n = try!(self.inner.read(&mut buf[..max]));
        self.limit -= n as u64;
        Ok(n)
    }
}

/// An adaptor which will emit all read data to a specified writer as well.
///
/// For more information see `ReadExt::tee`
pub struct Tee<R, W> {
    reader: R,
    writer: W,
}

impl<R: Read, W: Write> Read for Tee<R, W> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let n = try!(self.reader.read(buf));
        // TODO: what if the write fails? (we read something)
        try!(self.writer.write_all(&buf[..n]));
        Ok(n)
    }
}

/// A bridge from implementations of `Read` to an `Iterator` of `u8`.
///
/// See `ReadExt::bytes` for more information.
pub struct Bytes<R> {
    inner: R,
}

impl<R: Read> Iterator for Bytes<R> {
    type Item = Result<u8>;

    fn next(&mut self) -> Option<Result<u8>> {
        let mut buf = [0];
        match self.inner.read(&mut buf) {
            Ok(0) => None,
            Ok(..) => Some(Ok(buf[0])),
            Err(e) => Some(Err(e)),
        }
    }
}

/// A bridge from implementations of `Read` to an `Iterator` of `char`.
///
/// See `ReadExt::chars` for more information.
pub struct Chars<R> {
    inner: R,
}

/// An enumeration of possible errors that can be generated from the `Chars`
/// adapter.
#[derive(PartialEq, Clone, Debug)]
pub enum CharsError {
    /// Variant representing that the underlying stream was read successfully
    /// but it did not contain valid utf8 data.
    NotUtf8,

    /// Variant representing that an I/O error occurred.
    Other(Error),
}

impl<R: Read> Iterator for Chars<R> {
    type Item = result::Result<char, CharsError>;

    fn next(&mut self) -> Option<result::Result<char, CharsError>> {
        let mut buf = [0];
        let first_byte = match self.inner.read(&mut buf) {
            Ok(0) => return None,
            Ok(..) => buf[0],
            Err(e) => return Some(Err(CharsError::Other(e))),
        };
        let width = core_str::utf8_char_width(first_byte);
        if width == 1 { return Some(Ok(first_byte as char)) }
        if width == 0 { return Some(Err(CharsError::NotUtf8)) }
        let mut buf = [first_byte, 0, 0, 0];
        {
            let mut start = 1;
            while start < width {
                match self.inner.read(&mut buf[start..width]) {
                    Ok(0) => return Some(Err(CharsError::NotUtf8)),
                    Ok(n) if n == width - start => break,
                    Ok(n) => start += n,
                    Err(e) => return Some(Err(CharsError::Other(e))),
                }
            }
        }
        Some(match str::from_utf8(&buf[..width]).ok() {
            Some(s) => Ok(s.char_at(0)),
            None => Err(CharsError::NotUtf8),
        })
    }
}

impl StdError for CharsError {
    fn description(&self) -> &str {
        match *self {
            CharsError::NotUtf8 => "invalid utf8 encoding",
            CharsError::Other(ref e) => e.description(),
        }
    }
    fn cause(&self) -> Option<&StdError> {
        match *self {
            CharsError::NotUtf8 => None,
            CharsError::Other(ref e) => e.cause(),
        }
    }
}

impl fmt::Display for CharsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            CharsError::NotUtf8 => {
                "byte stream did not contain valid utf8".fmt(f)
            }
            CharsError::Other(ref e) => e.fmt(f),
        }
    }
}
