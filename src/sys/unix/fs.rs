// Copyright 2013-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Blocking posix-based file I/O

use core::prelude::*;
use io::prelude::*;

use ffi::{self, CString};
use io::{self, Error, Seek, SeekPos};
use libc::{self, c_int, c_void, size_t, off_t, c_char, mode_t};
use mem;
use path::{Path, GenericPath};
use ptr;
use rc::Rc;
use sys::fd::FileDesc;
use vec::Vec;

pub struct File(FileDesc);

pub struct FileAttr {
    stat: libc::stat,
}

pub struct ReadDir {
    dirp: *mut libc::DIR,
    root: Rc<Path>,
}

pub struct DirEntry {
    buf: Vec<u8>,
    dirent: *mut libc::dirent_t,
    root: Rc<Path>,
}

#[derive(Clone)]
pub struct OpenOptions {
    flags: c_int,
    read: bool,
    write: bool,
    mode: mode_t,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FilePermission { mode: mode_t }

impl FileAttr {
    pub fn is_dir(&self) -> bool {
        (self.stat.st_mode as mode_t) & libc::S_IFMT == libc::S_IFDIR
    }
    pub fn is_file(&self) -> bool {
        (self.stat.st_mode as mode_t) & libc::S_IFMT == libc::S_IFREG
    }
    pub fn size(&self) -> u64 { self.stat.st_size as u64 }
    pub fn perm(&self) -> FilePermission {
        FilePermission { mode: (self.stat.st_mode as mode_t) & 0o777 }
    }

// fn mkstat(stat: &libc::stat) -> FileStat {
//     // FileStat times are in milliseconds
//     fn mktime(secs: u64, nsecs: u64) -> u64 { secs * 1000 + nsecs / 1000000 }
//
//     #[cfg(not(any(target_os = "linux", target_os = "android")))]
//     fn flags(stat: &libc::stat) -> u64 { stat.st_flags as u64 }
//     #[cfg(any(target_os = "linux", target_os = "android"))]
//     fn flags(_stat: &libc::stat) -> u64 { 0 }
//
//     #[cfg(not(any(target_os = "linux", target_os = "android")))]
//     fn gen(stat: &libc::stat) -> u64 { stat.st_gen as u64 }
//     #[cfg(any(target_os = "linux", target_os = "android"))]
//     fn gen(_stat: &libc::stat) -> u64 { 0 }
//
//     FileStat {
//         size: stat.st_size as u64,
//         kind: match (stat.st_mode as libc::mode_t) & libc::S_IFMT {
//             libc::S_IFREG => old_io::FileType::RegularFile,
//             libc::S_IFDIR => old_io::FileType::Directory,
//             libc::S_IFIFO => old_io::FileType::NamedPipe,
//             libc::S_IFBLK => old_io::FileType::BlockSpecial,
//             libc::S_IFLNK => old_io::FileType::Symlink,
//             _ => old_io::FileType::Unknown,
//         },
//         perm: FilePermission::from_bits_truncate(stat.st_mode as u32),
//         created: mktime(stat.st_ctime as u64, stat.st_ctime_nsec as u64),
//         modified: mktime(stat.st_mtime as u64, stat.st_mtime_nsec as u64),
//         accessed: mktime(stat.st_atime as u64, stat.st_atime_nsec as u64),
//         unstable: UnstableFileStat {
//             device: stat.st_dev as u64,
//             inode: stat.st_ino as u64,
//             rdev: stat.st_rdev as u64,
//             nlink: stat.st_nlink as u64,
//             uid: stat.st_uid as u64,
//             gid: stat.st_gid as u64,
//             blksize: stat.st_blksize as u64,
//             blocks: stat.st_blocks as u64,
//             flags: flags(stat),
//             gen: gen(stat),
//         },
//     }
// }
}

impl FilePermission {
    pub fn readonly(&self) -> bool { self.mode & 0o222 == 0 }
    pub fn set_readonly(&mut self, readonly: bool) {
        if readonly {
            self.mode &= !0o222;
        } else {
            self.mode |= 0o222;
        }
    }
}

impl Iterator for ReadDir {
    type Item = io::Result<DirEntry>;

    fn next(&mut self) -> Option<io::Result<DirEntry>> {
        extern {
            fn rust_dirent_t_size() -> c_int;
        }

        let mut buf: Vec<u8> = Vec::with_capacity(unsafe {
            rust_dirent_t_size() as usize
        });
        let ptr = buf.as_mut_ptr() as *mut libc::dirent_t;

        let mut entry_ptr = ptr::null_mut();
        loop {
            if unsafe { libc::readdir_r(self.dirp, ptr, &mut entry_ptr) != 0 } {
                return Some(Err(Error::last_os_error()))
            }
            if entry_ptr.is_null() {
                return None
            }

            let entry = DirEntry {
                buf: buf,
                dirent: entry_ptr,
                root: self.root.clone()
            };
            if entry.name_bytes() == b"." || entry.name_bytes() == b".." {
                buf = entry.buf;
            } else {
                return Some(Ok(entry))
            }
        }
    }
}

impl Drop for ReadDir {
    fn drop(&mut self) {
        let r = unsafe { libc::closedir(self.dirp) };
        debug_assert_eq!(r, 0);
    }
}

impl DirEntry {
    pub fn path(&self) -> Path {
        self.root.join(self.name_bytes())
    }

    fn name_bytes(&self) -> &[u8] {
        extern {
            fn rust_list_dir_val(ptr: *mut libc::dirent_t) -> *const c_char;
        }
        unsafe {
            let ptr = rust_list_dir_val(self.dirent);
            ffi::c_str_to_bytes(mem::copy_lifetime(self, &ptr))
        }
    }
}

impl OpenOptions {
    pub fn new() -> OpenOptions {
        OpenOptions {
            flags: 0,
            read: false,
            write: false,
            mode: libc::S_IRUSR | libc::S_IWUSR,
        }
    }

    pub fn read(&mut self, read: bool) {
        self.read = read;
    }

    pub fn write(&mut self, write: bool) {
        self.write = write;
    }

    pub fn append(&mut self, append: bool) {
        self.flag(libc::O_APPEND, append);
    }

    pub fn truncate(&mut self, truncate: bool) {
        self.flag(libc::O_TRUNC, truncate);
    }

    pub fn create(&mut self, create: bool) {
        self.flag(libc::O_CREAT, create);
    }

    fn flag(&mut self, bit: c_int, on: bool) {
        if on {
            self.flags |= bit;
        } else {
            self.flags &= !bit;
        }
    }
}

impl File {
    pub fn open(path: &Path, opts: &OpenOptions) -> io::Result<File> {
        let flags = opts.flags | match (opts.read, opts.write) {
            (true, true) => libc::O_RDWR,
            (false, true) => libc::O_WRONLY,
            (true, false) |
            (false, false) => libc::O_RDONLY,
        };
        let path = cstr(path);
        // TODO: retry this?
        let fd = try!(call!(unsafe {
            libc::open(path.as_ptr(), flags, opts.mode)
        }));
        Ok(File(FileDesc::new(fd)))
    }

    pub fn file_attr(&self) -> io::Result<FileAttr> {
        let mut stat: libc::stat = unsafe { mem::zeroed() };
        try!(call!(unsafe { libc::fstat(self.0.raw(), &mut stat) }));
        Ok(FileAttr { stat: stat })
    }
//
//     pub fn fsync(&self) -> IoResult<()> {
//         mkerr_libc(retry(|| unsafe { libc::fsync(self.fd()) }))
//     }
//
//     pub fn datasync(&self) -> IoResult<()> {
//         return mkerr_libc(os_datasync(self.fd()));
//
//         #[cfg(any(target_os = "macos", target_os = "ios"))]
//         fn os_datasync(fd: c_int) -> c_int {
//             unsafe { libc::fcntl(fd, libc::F_FULLFSYNC) }
//         }
//         #[cfg(target_os = "linux")]
//         fn os_datasync(fd: c_int) -> c_int {
//             retry(|| unsafe { libc::fdatasync(fd) })
//         }
//         #[cfg(not(any(target_os = "macos", target_os = "ios", target_os = "linux")))]
//         fn os_datasync(fd: c_int) -> c_int {
//             retry(|| unsafe { libc::fsync(fd) })
//         }
//     }
//
//     pub fn truncate(&self, offset: i64) -> IoResult<()> {
//         mkerr_libc(retry(|| unsafe {
//             libc::ftruncate(self.fd(), offset as libc::off_t)
//         }))
//     }
//
//     pub fn fstat(&self) -> IoResult<FileStat> {
//         let mut stat: libc::stat = unsafe { mem::zeroed() };
//         match unsafe { libc::fstat(self.fd(), &mut stat) } {
//             0 => Ok(mkstat(&stat)),
//             _ => Err(super::last_error()),
//         }
//     }
//
}

impl Read for File {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf)
    }
}
impl Write for File {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }
}
impl Seek for File {
    fn seek(&mut self, pos: SeekPos) -> io::Result<u64> {
        let (whence, pos) = match pos {
            SeekPos::FromStart(off) => (libc::SEEK_SET, off as off_t),
            SeekPos::FromEnd(off) => (libc::SEEK_END, off as off_t),
            SeekPos::FromCur(off) => (libc::SEEK_CUR, off as off_t),
        };
        let n = try!(call!(unsafe { libc::lseek(self.0.raw(), pos, whence) }));
        Ok(n as u64)
    }
}

fn cstr(path: &Path) -> CString {
    CString::from_slice(path.as_vec())
}

pub fn mkdir(p: &Path) -> io::Result<()> {
    let p = cstr(p);
    try!(call!(unsafe { libc::mkdir(p.as_ptr(), 0o777) }));
    Ok(())
}

pub fn readdir(p: &Path) -> io::Result<ReadDir> {
    let root = Rc::new(p.clone());
    let p = cstr(p);
    unsafe {
        let ptr = libc::opendir(p.as_ptr());
        if ptr.is_null() {
            Err(Error::last_os_error())
        } else {
            Ok(ReadDir { dirp: ptr, root: root })
        }
    }
}

pub fn unlink(p: &Path) -> io::Result<()> {
    let p = cstr(p);
    try!(call!(unsafe { libc::unlink(p.as_ptr()) }));
    Ok(())
}

pub fn rename(old: &Path, new: &Path) -> io::Result<()> {
    let old = cstr(old);
    let new = cstr(new);
    try!(call!(unsafe { libc::rename(old.as_ptr(), new.as_ptr()) }));
    Ok(())
}

pub fn set_perm(p: &Path, perm: FilePermission) -> io::Result<()> {
    let p = cstr(p);
    try!(call!(unsafe { libc::chmod(p.as_ptr(), perm.mode) }));
    Ok(())
}

pub fn rmdir(p: &Path) -> io::Result<()> {
    let p = cstr(p);
    try!(call!(unsafe { libc::rmdir(p.as_ptr()) }));
    Ok(())
}

// pub fn chown(p: &Path, uid: int, gid: int) -> IoResult<()> {
//     let p = cstr(p);
//     mkerr_libc(retry(|| unsafe {
//         libc::chown(p.as_ptr(), uid as libc::uid_t, gid as libc::gid_t)
//     }))
// }

pub fn readlink(p: &Path) -> io::Result<Path> {
    let c_path = cstr(p);
    let p = c_path.as_ptr();
    let mut len = unsafe { libc::pathconf(p as *mut _, libc::_PC_NAME_MAX) };
    if len < 0 {
        len = 1024; // FIXME: read PATH_MAX from C ffi?
    }
    let mut buf: Vec<u8> = Vec::with_capacity(len as usize);
    unsafe {
        let n = try!(call!({
            libc::readlink(p, buf.as_ptr() as *mut c_char, len as size_t)
        }));
        buf.set_len(n as usize);
        Ok(Path::new(buf))
    }
}

pub fn symlink(src: &Path, dst: &Path) -> io::Result<()> {
    let src = cstr(src);
    let dst = cstr(dst);
    try!(call!(unsafe { libc::symlink(src.as_ptr(), dst.as_ptr()) }));
    Ok(())
}

pub fn link(src: &Path, dst: &Path) -> io::Result<()> {
    let src = cstr(src);
    let dst = cstr(dst);
    try!(call!(unsafe { libc::link(src.as_ptr(), dst.as_ptr()) }));
    Ok(())
}

pub fn stat(p: &Path) -> io::Result<FileAttr> {
    let p = cstr(p);
    let mut stat: libc::stat = unsafe { mem::zeroed() };
    try!(call!(unsafe { libc::stat(p.as_ptr(), &mut stat) }));
    Ok(FileAttr { stat: stat })
}

pub fn lstat(p: &Path) -> io::Result<FileAttr> {
    let p = cstr(p);
    let mut stat: libc::stat = unsafe { mem::zeroed() };
    try!(call!(unsafe { libc::lstat(p.as_ptr(), &mut stat) }));
    Ok(FileAttr { stat: stat })
}

// pub fn utime(p: &Path, atime: u64, mtime: u64) -> IoResult<()> {
//     let p = cstr(p);
//     let buf = libc::utimbuf {
//         actime: (atime / 1000) as libc::time_t,
//         modtime: (mtime / 1000) as libc::time_t,
//     };
//     mkerr_libc(unsafe { libc::utime(p.as_ptr(), &buf) })
// }
//
// #[cfg(test)]
// mod tests {
//     use super::FileDesc;
//     use libc;
//     use os;
//     use prelude::v1::*;
//
//     #[cfg_attr(target_os = "freebsd", ignore)] // hmm, maybe pipes have a tiny buffer
//     #[test]
//     fn test_file_desc() {
//         // Run this test with some pipes so we don't have to mess around with
//         // opening or closing files.
//         let os::Pipe { reader, writer } = unsafe { os::pipe().unwrap() };
//         let mut reader = FileDesc::new(reader, true);
//         let mut writer = FileDesc::new(writer, true);
//
//         writer.write(b"test").ok().unwrap();
//         let mut buf = [0u8; 4];
//         match reader.read(&mut buf) {
//             Ok(4) => {
//                 assert_eq!(buf[0], 't' as u8);
//                 assert_eq!(buf[1], 'e' as u8);
//                 assert_eq!(buf[2], 's' as u8);
//                 assert_eq!(buf[3], 't' as u8);
//             }
//             r => panic!("invalid read: {:?}", r),
//         }
//
//         assert!(writer.read(&mut buf).is_err());
//         assert!(reader.write(&buf).is_err());
//     }
// }
