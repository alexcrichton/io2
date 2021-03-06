#![feature(libc, std_misc, core, path, unicode, collections, hash)]
#![no_std]
#![feature(asm)]

#[macro_use]
extern crate std;
extern crate core;
extern crate libc;
extern crate unicode;

pub use std::{slice, ptr, cmp, vec, iter, marker, mem, str, collections, path};
pub use std::{string, prelude, result, option, boxed, clone, error, fmt};
pub use std::{num, rc, sync, rt, hash, ops, ffi, usize};

mod borrow {
    use marker::Sized;
    pub struct ByRef<'a, T: ?Sized + 'a> {
        pub inner: &'a mut T,
    }
}

pub mod io;
pub mod fs;
// pub mod env;
pub mod os;
pub mod net;

#[cfg(unix)]    #[path = "sys/unix/mod.rs"]    mod sys;
#[cfg(windows)] #[path = "sys/windows/mod.rs"] mod sys;
#[path = "sys/common/mod.rs"] mod sys_common;
