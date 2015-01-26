// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use core::prelude::*;
use io::prelude::*;

use io::{self, Error, ErrorKind, SeekPos, Seek};
use libc::{self, HANDLE};
use mem;
use path::{Path, GenericPath};
use ptr;
use string::String;
use sys::c;
use sys::handle::Handle;
use sys;
use vec::Vec;

pub struct File { handle: Handle }
pub struct FileAttr { data: c::WIN32_FILE_ATTRIBUTE_DATA }

pub struct ReadDir {
    handle: libc::HANDLE,
    root: Path,
    first: Option<libc::WIN32_FIND_DATAW>,
}

pub struct DirEntry { path: Path }

#[allow(bad_style)]
#[derive(Clone)]
pub struct OpenOptions {
    dwDesiredAccess: libc::DWORD,
    dwShareMode: libc::DWORD,
    dwCreationDisposition: libc::DWORD,
    dwFlagsAndAttributes: libc::DWORD,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FilePermission { attrs: libc::DWORD }

impl Iterator for ReadDir {
    type Item = io::Result<DirEntry>;
    fn next(&mut self) -> Option<io::Result<DirEntry>> {
        if let Some(first) = self.first.take() {
            if let Some(e) = DirEntry::new(&self.root, &first) {
                return Some(e);
            }
        }
        unsafe {
            let mut wfd = mem::zeroed();
            loop {
                match libc::FindNextFileW(self.handle, &mut wfd) {
                    0 => {
                        if libc::GetLastError() ==
                            c::ERROR_NO_MORE_FILES as libc::DWORD {
                            return None
                        } else {
                            return Some(Err(Error::last_os_error()))
                        }
                    }
                    _ => {}
                }
                if let Some(e) = DirEntry::new(&self.root, &wfd) {
                    return Some(e)
                }
            }
        }
    }
}

impl Drop for ReadDir {
    fn drop(&mut self) {
        let r = unsafe { libc::FindClose(self.handle) };
        debug_assert!(r != 0);
    }
}

impl DirEntry {
    fn new(root: &Path, wfd: &libc::WIN32_FIND_DATAW)
           -> Option<io::Result<DirEntry>> {
        match &wfd.cFileName[0..3] {
            // check for '.' and '..'
            [46, 0, ..] |
            [46, 46, 0, ..] => return None,
            _ => {}
        }

        // FIXME: once path reform lands then this should use OsString to read
        //        cFileName and convert it into a path which should never
        //        generate an error.
        let filename = super::truncate_utf16_at_nul(&wfd.cFileName);
        Some(match String::from_utf16(filename) {
            Ok(filename) => Ok(DirEntry { path: root.join(filename) }),
            Err(..) => {
                Err(Error::new(ErrorKind::InvalidInput,
                               "path was not valid UTF-16", None))
            }
        })
    }

    pub fn path(&self) -> Path {
        self.path.clone()
    }
}

impl OpenOptions {
    pub fn new() -> OpenOptions {
        OpenOptions {
            dwDesiredAccess: 0,
            dwCreationDisposition: libc::OPEN_EXISTING,

            dwFlagsAndAttributes: libc::FILE_ATTRIBUTE_NORMAL,

            // libuv has a good comment about this, but the basic idea is that
            // we try to emulate unix semantics by enabling all sharing by
            // allowing things such as deleting a file while it's still open.
            dwShareMode: libc::FILE_SHARE_READ | libc::FILE_SHARE_WRITE |
                         libc::FILE_SHARE_DELETE,
        }
    }

    pub fn read(&mut self, read: bool) {
        flag(&mut self.dwDesiredAccess, libc::FILE_GENERIC_READ, read);
    }

    pub fn write(&mut self, write: bool) {
        flag(&mut self.dwDesiredAccess, libc::FILE_GENERIC_WRITE, write);
    }

    pub fn append(&mut self, append: bool) {
        if append {
            flag(&mut self.dwDesiredAccess, libc::FILE_WRITE_DATA, false);
        }
        flag(&mut self.dwDesiredAccess, libc::FILE_APPEND_DATA, append);
    }

    pub fn truncate(&mut self, truncate: bool) {
        // CREATE_NEW    -- already truncates
        // CREATE_ALWAYS -- already truncates
        // OPEN_ALWAYS   -- already truncates
        self.dwCreationDisposition = if truncate {
            match self.dwCreationDisposition {
                libc::OPEN_EXISTING => {
                    if self.dwDesiredAccess & libc::FILE_WRITE_DATA != 0 {
                        libc::CREATE_ALWAYS
                    } else {
                        libc::TRUNCATE_EXISTING
                    }
                }
                n => n,
            }
        } else {
            match self.dwCreationDisposition {
                libc::TRUNCATE_EXISTING => libc::OPEN_EXISTING,
                libc::CREATE_ALWAYS => libc::OPEN_ALWAYS,
                n => n,
            }
        };
    }

    pub fn create(&mut self, create: bool) {
        // CREATE_NEW    -- already creates
        // OPEN_ALWAYS   -- already creates
        self.dwCreationDisposition = if create {
            match self.dwCreationDisposition {
                libc::TRUNCATE_EXISTING |
                libc::OPEN_EXISTING => libc::CREATE_ALWAYS,
                n => n,
            }
        } else {
            // FIXME: this means that .truncate(true).create(true).create(false)
            //        is not technically correct.
            match self.dwCreationDisposition {
                libc::CREATE_ALWAYS => libc::OPEN_EXISTING,
                n => n,
            }
        };
    }
}

fn flag(slot: &mut libc::DWORD, val: libc::DWORD, on: bool) {
    if on {
        *slot |= val;
    } else {
        *slot &= !val;
    }
}

impl File {
    pub fn open(path: &Path, opts: &OpenOptions) -> io::Result<File> {
        let path = try!(to_utf16(path));
        let handle = unsafe {
            libc::CreateFileW(path.as_ptr(),
                              opts.dwDesiredAccess,
                              opts.dwShareMode,
                              ptr::null_mut(),
                              opts.dwCreationDisposition,
                              opts.dwFlagsAndAttributes,
                              ptr::null_mut())
        };
        if handle == libc::INVALID_HANDLE_VALUE {
            Err(Error::last_os_error())
        } else {
            Ok(File { handle: Handle::new(handle) })
        }
    }

//     pub fn fsync(&mut self) -> IoResult<()> {
//         super::mkerr_winbool(unsafe {
//             libc::FlushFileBuffers(self.handle)
//         })
//     }
//
//     pub fn datasync(&mut self) -> IoResult<()> { return self.fsync(); }
//
//     pub fn truncate(&mut self, offset: i64) -> IoResult<()> {
//         let orig_pos = try!(self.tell());
//         let _ = try!(self.seek(offset, SeekSet));
//         let ret = unsafe {
//             match libc::SetEndOfFile(self.handle) {
//                 0 => Err(super::last_error()),
//                 _ => Ok(())
//             }
//         };
//         let _ = self.seek(orig_pos as i64, SeekSet);
//         return ret;
//     }

    pub fn file_attr(&self) -> io::Result<FileAttr> {
        unsafe {
            let mut info: c::BY_HANDLE_FILE_INFORMATION = mem::zeroed();
            try!(call!(c::GetFileInformationByHandle(self.handle.raw(),
                                                     &mut info)));
            Ok(FileAttr {
                data: c::WIN32_FILE_ATTRIBUTE_DATA {
                    dwFileAttributes: info.dwFileAttributes,
                    ftCreationTime: info.ftCreationTime,
                    ftLastAccessTime: info.ftLastAccessTime,
                    ftLastWriteTime: info.ftLastWriteTime,
                    nFileSizeHigh: info.nFileSizeHigh,
                    nFileSizeLow: info.nFileSizeLow,
                }
            })
        }
    }
}

impl Read for File {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut read = 0;
        try!(call!(unsafe {
            libc::ReadFile(self.handle.raw(),
                           buf.as_ptr() as libc::LPVOID,
                           buf.len() as libc::DWORD,
                           &mut read,
                           ptr::null_mut())
        }));
        Ok(read as usize)
    }
}
impl Write for File {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut amt = 0;
        try!(call!(unsafe {
            libc::WriteFile(self.handle.raw(),
                            buf.as_ptr() as libc::LPVOID,
                            buf.len() as libc::DWORD,
                            &mut amt,
                            ptr::null_mut())
        }));
        Ok(amt as usize)
    }
}
impl Seek for File {
    fn seek(&mut self, pos: SeekPos) -> io::Result<u64> {
        let (whence, pos) = match pos {
            SeekPos::FromStart(n) => (libc::FILE_BEGIN, n as i64),
            SeekPos::FromEnd(n) => (libc::FILE_END, n),
            SeekPos::FromCur(n) => (libc::FILE_CURRENT, n),
        };
        let pos = pos as libc::LARGE_INTEGER;
        let mut newpos = 0;
        try!(call!(unsafe {
            libc::SetFilePointerEx(self.handle.raw(), pos,
                                   &mut newpos, whence)
        }));
        Ok(newpos as u64)
    }
}

pub fn to_utf16(s: &Path) -> io::Result<Vec<u16>> {
    sys::to_utf16(s.as_str())
}

impl FileAttr {
    pub fn is_dir(&self) -> bool {
        self.data.dwFileAttributes & c::FILE_ATTRIBUTE_DIRECTORY != 0
    }
    pub fn is_file(&self) -> bool {
        // TODO: verify that this is correct
        !self.is_dir()
    }
    pub fn size(&self) -> u64 {
        ((self.data.nFileSizeHigh as u64) << 32) | (self.data.nFileSizeLow as u64)
    }
    pub fn perm(&self) -> FilePermission {
        FilePermission { attrs: self.data.dwFileAttributes }
    }
}

impl FilePermission {
    pub fn readonly(&self) -> bool {
        self.attrs & c::FILE_ATTRIBUTE_READONLY != 0
    }

    pub fn set_readonly(&mut self, readonly: bool) {
        flag(&mut self.attrs, c::FILE_ATTRIBUTE_READONLY, readonly);
    }
}

pub fn mkdir(p: &Path) -> io::Result<()> {
    let p = try!(to_utf16(p));
    try!(call!(unsafe {
        libc::CreateDirectoryW(p.as_ptr(), ptr::null_mut())
    }));
    Ok(())
}

pub fn readdir(p: &Path) -> io::Result<ReadDir> {
    let root = p.clone();
    let star = p.join("*");
    let path = try!(to_utf16(&star));

    unsafe {
        let mut wfd = mem::zeroed();
        let find_handle = libc::FindFirstFileW(path.as_ptr(), &mut wfd);
        if find_handle != libc::INVALID_HANDLE_VALUE {
            Ok(ReadDir { handle: find_handle, root: root, first: Some(wfd) })
        } else {
            Err(Error::last_os_error())
        }
    }
}

pub fn unlink(p: &Path) -> io::Result<()> {
    fn do_unlink(p_utf16: &Vec<u16>) -> io::Result<()> {
        try!(call!(unsafe { libc::DeleteFileW(p_utf16.as_ptr()) }));
        Ok(())
    }

    let p_utf16 = try!(to_utf16(p));
    let res = do_unlink(&p_utf16);
    let e = match res {
        Ok(()) => return Ok(()),
        Err(e) => e,
    };

    // On unix, a readonly file can be successfully removed. On windows,
    // however, it cannot. To keep the two platforms in line with
    // respect to their behavior, catch this case on windows, attempt to
    // change it to read-write, and then remove the file.
    if e.kind() != ErrorKind::PermissionDenied {
        return Err(e)
    }
    unsafe {
        let perm = c::GetFileAttributesW(p_utf16.as_ptr());
        if perm & c::FILE_ATTRIBUTE_READONLY == 0 { return Err(e) }
        let perm2 = perm & !c::FILE_ATTRIBUTE_READONLY;
        match call!(c::SetFileAttributesW(p_utf16.as_ptr(), perm2)) {
            Ok(..) => {}
            Err(..) => return Err(e),
        }
        match do_unlink(&p_utf16) {
            Ok(()) => return Ok(()),
            Err(..) => {}
        }
        // Oops, try to put things back the way we found it
        let _ = c::SetFileAttributesW(p_utf16.as_ptr(), perm);
    }
    Err(e)
}

pub fn rename(old: &Path, new: &Path) -> io::Result<()> {
    let old = try!(to_utf16(old));
    let new = try!(to_utf16(new));
    try!(call!(unsafe {
        libc::MoveFileExW(old.as_ptr(), new.as_ptr(),
                          libc::MOVEFILE_REPLACE_EXISTING)
    }));
    Ok(())
}

pub fn rmdir(p: &Path) -> io::Result<()> {
    let p = try!(to_utf16(p));
    try!(call!(unsafe { c::RemoveDirectoryW(p.as_ptr()) }));
    Ok(())
}

pub fn readlink(p: &Path) -> io::Result<Path> {
    use sys::c::compat::kernel32::GetFinalPathNameByHandleW;
    let mut opts = OpenOptions::new();
    opts.read(true);
    let file = try!(File::open(p, &opts));;

    // Specify (sz - 1) because the documentation states that it's the size
    // without the null pointer
    //
    // FIXME: I have a feeling that this reads intermediate symlinks as well.
    let ret = try!(super::fill_utf16_buf_and_decode(|buf, sz| unsafe {
        GetFinalPathNameByHandleW(file.handle.raw(),
                                  buf as *const u16,
                                  sz - 1,
                                  libc::VOLUME_NAME_DOS)
    }));
    // TODO: don't unwrap here
    let s = String::from_utf16(ret.as_slice()).unwrap();
    if s.starts_with(r"\\?\") {
        Ok(Path::new(&s[4..]))
    } else {
        Ok(Path::new(s))
    }
}

pub fn symlink(src: &Path, dst: &Path) -> io::Result<()> {
    use sys::c::compat::kernel32::CreateSymbolicLinkW;
    let src = try!(to_utf16(src));
    let dst = try!(to_utf16(dst));
    try!(call!(unsafe {
        CreateSymbolicLinkW(dst.as_ptr(), src.as_ptr(), 0) as libc::BOOL
    }));
    Ok(())
}

pub fn link(src: &Path, dst: &Path) -> io::Result<()> {
    let src = try!(to_utf16(src));
    let dst = try!(to_utf16(dst));
    try!(call!(unsafe {
        libc::CreateHardLinkW(dst.as_ptr(), src.as_ptr(), ptr::null_mut())
    }));
    Ok(())
}

pub fn stat(p: &Path) -> io::Result<FileAttr> {
    let p = try!(to_utf16(p));
    unsafe {
        let mut attr: FileAttr = mem::zeroed();
        try!(call!(c::GetFileAttributesExW(p.as_ptr(),
                                           c::GetFileExInfoStandard,
                                           &mut attr.data as *mut _ as *mut _)));
        Ok(attr)
    }
}

pub fn set_perm(p: &Path, perm: FilePermission) -> io::Result<()> {
    let p = try!(to_utf16(p));
    unsafe {
        try!(call!(c::SetFileAttributesW(p.as_ptr(), perm.attrs)));
        Ok(())
    }
}

// // FIXME: move this to platform-specific modules (for now)?
// pub fn lstat(_p: &Path) -> IoResult<FileStat> {
//     // FIXME: implementation is missing
//     Err(super::unimpl())
// }
//
// pub fn utime(p: &Path, atime: u64, mtime: u64) -> IoResult<()> {
//     let mut buf = libc::utimbuf {
//         actime: atime as libc::time64_t,
//         modtime: mtime as libc::time64_t,
//     };
//     let p = try!(to_utf16(p));
//     mkerr_libc(unsafe {
//         libc::wutime(p.as_ptr(), &mut buf)
//     })
// }
