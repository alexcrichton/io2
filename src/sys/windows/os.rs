// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Implementation of `std::os` functionality for Windows

#![allow(bad_style)]

use prelude::v1::*;
use os::windows::*;

use error::Error as StdError;
use ffi::{OsString, OsStr};
use fmt;
use io::{self, Error};
use iter::Range;
use libc::{self, c_int, c_void};
use libc::types::os::arch::extra::LPWCH;
use mem;
use ptr;
use slice;
use sys::c;

use libc::funcs::extra::kernel32::{
    GetEnvironmentStringsW,
    FreeEnvironmentStringsW
};

pub fn errno() -> i32 {
    unsafe { libc::GetLastError() as i32 }
}

/// Get a detailed string description for the given error number
pub fn error_string(errnum: i32) -> String {
    use libc::types::os::arch::extra::DWORD;
    use libc::types::os::arch::extra::LPWSTR;
    use libc::types::os::arch::extra::LPVOID;
    use libc::types::os::arch::extra::WCHAR;

    #[link_name = "kernel32"]
    extern "system" {
        fn FormatMessageW(flags: DWORD,
                          lpSrc: LPVOID,
                          msgId: DWORD,
                          langId: DWORD,
                          buf: LPWSTR,
                          nsize: DWORD,
                          args: *const c_void)
                          -> DWORD;
    }

    static FORMAT_MESSAGE_FROM_SYSTEM: DWORD = 0x00001000;
    static FORMAT_MESSAGE_IGNORE_INSERTS: DWORD = 0x00000200;

    // This value is calculated from the macro
    // MAKELANGID(LANG_SYSTEM_DEFAULT, SUBLANG_SYS_DEFAULT)
    let langId = 0x0800 as DWORD;

    let mut buf = [0 as WCHAR; 2048];

    unsafe {
        let res = FormatMessageW(FORMAT_MESSAGE_FROM_SYSTEM |
                                 FORMAT_MESSAGE_IGNORE_INSERTS,
                                 ptr::null_mut(),
                                 errnum as DWORD,
                                 langId,
                                 buf.as_mut_ptr(),
                                 buf.len() as DWORD,
                                 ptr::null());
        if res == 0 {
            // Sometimes FormatMessageW can fail e.g. system doesn't like langId,
            let fm_err = errno();
            return format!("OS Error {} (FormatMessageW() returned error {})",
                           errnum, fm_err);
        }

        let b = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
        let msg = String::from_utf16(&buf[..b]);
        match msg {
            Ok(msg) => msg,
            Err(..) => format!("OS Error {} (FormatMessageW() returned \
                                invalid UTF-16)", errnum),
        }
    }
}

pub struct Env {
    base: LPWCH,
    cur: LPWCH,
}

impl Iterator for Env {
    type Item = (OsString, OsString);

    fn next(&mut self) -> Option<(OsString, OsString)> {
        unsafe {
            if *self.cur == 0 { return None }
            let p = &*self.cur;
            let mut len = 0;
            while *(p as *const _).offset(len) != 0 {
                len += 1;
            }
            let p = p as *const u16;
            let s = slice::from_raw_buf(&p, len as usize);
            self.cur = self.cur.offset(len + 1);

            let (k, v) = match s.iter().position(|&b| b == '=' as u16) {
                Some(n) => (&s[..n], &s[n+1..]),
                None => (s, &[][]),
            };
            Some((OsStringExt::from_wide(k), OsStringExt::from_wide(v)))
        }
    }
}

impl Drop for Env {
    fn drop(&mut self) {
        unsafe { FreeEnvironmentStringsW(self.base); }
    }
}

pub fn env() -> Env {
    unsafe {
        let ch = GetEnvironmentStringsW();
        if ch as usize == 0 {
            panic!("failure getting env string from OS: {}",
                   Error::last_os_error());
        }
        Env { base: ch, cur: ch }
    }
}

pub struct SplitPaths<'a> {
    data: EncodeWide<'a>,
    must_yield: bool,
}

pub fn split_paths(unparsed: &OsStr) -> SplitPaths {
    SplitPaths {
        data: unparsed.encode_wide(),
        must_yield: true,
    }
}

impl<'a> Iterator for SplitPaths<'a> {
    type Item = Path;
    fn next(&mut self) -> Option<Path> {
        // On Windows, the PATH environment variable is semicolon separated.
        // Double quotes are used as a way of introducing literal semicolons
        // (since c:\some;dir is a valid Windows path). Double quotes are not
        // themselves permitted in path names, so there is no way to escape a
        // double quote.  Quoted regions can appear in arbitrary locations, so
        //
        //   c:\foo;c:\som"e;di"r;c:\bar
        //
        // Should parse as [c:\foo, c:\some;dir, c:\bar].
        //
        // (The above is based on testing; there is no clear reference available
        // for the grammar.)


        let must_yield = self.must_yield;
        self.must_yield = false;

        let mut in_progress = Vec::new();
        let mut in_quote = false;
        for b in self.data {
            if b == '"' as u16 {
                in_quote = !in_quote;
            } else if b == ';' as u16 && !in_quote {
                self.must_yield = true;
                break
            } else {
                in_progress.push(b)
            }
        }

        if !must_yield && in_progress.is_empty() {
            None
        } else {
            // TODO: OsString => Path
            let s = String::from_utf16(&*in_progress).unwrap();
            Some(Path::new(s))
        }
    }
}

#[derive(Show)]
pub struct JoinPathsError;

pub fn join_paths<'a, I>(paths: I) -> Result<OsString, JoinPathsError>
    where I: Iterator<Item=&'a OsStr>
{
    let mut joined = Vec::new();
    let sep = b';' as u16;

    for (i, path) in paths.enumerate() {
        if i > 0 { joined.push(sep) }
        let v = path.encode_wide().collect::<Vec<u16>>();
        if v.contains(&(b'"' as u16)) {
            return Err(JoinPathsError)
        } else if v.contains(&sep) {
            joined.push(b'"' as u16);
            joined.push_all(&v[]);
            joined.push(b'"' as u16);
        } else {
            joined.push_all(&v[]);
        }
    }

    Ok(OsStringExt::from_wide(&joined[]))
}

impl fmt::Display for JoinPathsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        "path segment contains `\"`".fmt(f)
    }
}

impl StdError for JoinPathsError {
    fn description(&self) -> &str { "failed to join paths" }
}

pub fn current_exe() -> Option<Path> {
    super::fill_utf16_buf_and_decode(|buf, sz| unsafe {
        libc::GetModuleFileNameW(ptr::null_mut(), buf, sz)
    }).map(|s| {
        // TODO: OsString -> Path
        Path::new(String::from_utf16(&*s).unwrap())
    }).ok()
}

pub fn getcwd() -> io::Result<Path> {
    let buf = try!(super::fill_utf16_buf_and_decode(|buf, sz| unsafe {
        libc::GetCurrentDirectoryW(sz, buf)
    }));

    // TODO: OsString -> Path
    Ok(Path::new(String::from_utf16(&buf[]).unwrap()))
}

pub fn chdir(p: &Path) -> io::Result<()> {
    // TODO: Path => OsString
    let mut p = p.as_str().unwrap().utf16_units().collect::<Vec<u16>>();
    p.push(0);

    unsafe {
        match libc::SetCurrentDirectoryW(p.as_ptr()) != (0 as libc::BOOL) {
            true => Ok(()),
            false => Err(Error::last_os_error()),
        }
    }
}

pub fn getenv(k: &OsStr) -> Option<OsString> {
    let k = super::to_utf16_os(k);
    super::fill_utf16_buf_and_decode(|buf, sz| unsafe {
        libc::GetEnvironmentVariableW(k.as_ptr(), buf, sz)
    }).ok().map(|buf| {
        OsStringExt::from_wide(&*buf)
    })
}

pub fn setenv(k: &OsStr, v: &OsStr) {
    let k = super::to_utf16_os(k);
    let v = super::to_utf16_os(v);

    unsafe {
        if libc::SetEnvironmentVariableW(k.as_ptr(), v.as_ptr()) == 0 {
            panic!("failed to set env: {}", Error::last_os_error());
        }
    }
}

pub fn unsetenv(n: &OsStr) {
    let v = super::to_utf16_os(n);
    unsafe {
        if libc::SetEnvironmentVariableW(v.as_ptr(), ptr::null()) == 0 {
            panic!("failed to unset env: {}", Error::last_os_error());
        }
    }
}

pub struct Args {
    range: Range<isize>,
    cur: *mut *mut u16,
}

impl Iterator for Args {
    type Item = OsString;
    fn next(&mut self) -> Option<OsString> {
        self.range.next().map(|i| unsafe {
            let ptr = *self.cur.offset(i);
            let mut len = 0;
            while *ptr.offset(len) != 0 { len += 1; }

            // Push it onto the list.
            let ptr = ptr as *const u16;
            let buf = slice::from_raw_buf(&ptr, len as usize);
            OsStringExt::from_wide(buf)
        })
    }
    fn size_hint(&self) -> (usize, Option<usize>) { self.range.size_hint() }
}

impl Drop for Args {
    fn drop(&mut self) {
        unsafe { c::LocalFree(self.cur as *mut c_void); }
    }
}

pub fn args() -> Args {
    unsafe {
        let mut nArgs: c_int = 0;
        let lpArgCount: *mut c_int = &mut nArgs;
        let lpCmdLine = c::GetCommandLineW();
        let szArgList = c::CommandLineToArgvW(lpCmdLine, lpArgCount);

        Args { cur: szArgList, range: range(0, lpArgCount as isize) }
    }
}

pub fn page_size() -> usize {
    unsafe {
        let mut info = mem::zeroed();
        libc::GetSystemInfo(&mut info);
        return info.dwPageSize as usize;
    }
}
