// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// pub mod helper_thread;
pub mod net;

// common error constructors

// pub fn timeout(desc: &'static str) -> IoError {
//     IoError {
//         kind: old_io::TimedOut,
//         desc: desc,
//         detail: None,
//     }
// }
//
// pub fn short_write(n: uint, desc: &'static str) -> IoError {
//     IoError {
//         kind: if n == 0 { old_io::TimedOut } else { old_io::ShortWrite(n) },
//         desc: desc,
//         detail: None,
//     }
// }

// pub fn unimpl() -> IoError {
//     IoError {
//         kind: old_io::IoUnavailable,
//         desc: "operations not yet supported",
//         detail: None,
//     }
// }

// A trait for extracting representations from std::io types
pub trait AsInner<Inner> {
    fn as_inner(&self) -> &Inner;
}

pub trait FromInner<Inner> {
    fn from_inner(inner: Inner) -> Self;
}

// pub trait ProcessConfig<K: BytesContainer, V: BytesContainer> {
//     fn program(&self) -> &CString;
//     fn args(&self) -> &[CString];
//     fn env(&self) -> Option<&collections::HashMap<K, V>>;
//     fn cwd(&self) -> Option<&CString>;
//     fn uid(&self) -> Option<uint>;
//     fn gid(&self) -> Option<uint>;
//     fn detach(&self) -> bool;
// }
