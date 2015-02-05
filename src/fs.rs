// Copyright 2013-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use core::prelude::*;

use io::{self, Error, ErrorKind, Read, Write, Seek, SeekPos};
use path::{Path, GenericPath};
use sys::fs as fs_imp;
use vec::Vec;

/// Unconstrained file access type that exposes read and write operations
///
/// Can be constructed via `File::open()`, `File::create()`, and
/// `File::open_opts()`.
///
/// # Error
///
/// This type will return errors as an `io::Result<T>` if operations are
/// attempted against it for which its underlying file descriptor was not
/// configured at creation time, via the `FileAccess` parameter to
/// `File::open_mode()`.
pub struct File {
    inner: fs_imp::File,
    path: Path,
}

pub struct FileAttr(fs_imp::FileAttr);
pub struct ReadDir(fs_imp::ReadDir);
pub struct DirEntry(fs_imp::DirEntry);
#[derive(Clone)]
pub struct OpenOptions(fs_imp::OpenOptions);
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FilePermission(fs_imp::FilePermission);

impl File {
    /// Open a file at `path` in the mode specified by the `mode` and `access`
    /// arguments
    ///
    /// # Example
    ///
    /// ```rust,should_fail
    /// use std::old_io::{File, Open, ReadWrite};
    ///
    /// let p = Path::new("/some/file/path.txt");
    ///
    /// let file = match File::open_mode(&p, Open, ReadWrite) {
    ///     Ok(f) => f,
    ///     Err(e) => panic!("file error: {}", e),
    /// };
    /// // do some stuff with that file
    ///
    /// // the file will be closed at the end of this block
    /// ```
    ///
    /// `FileMode` and `FileAccess` provide information about the permissions
    /// context in which a given stream is created. More information about them
    /// can be found in `std::io`'s docs. If a file is opened with `Write`
    /// or `ReadWrite` access, then it will be created if it does not already
    /// exist.
    ///
    /// Note that, with this function, a `File` is returned regardless of the
    /// access-limitations indicated by `FileAccess` (e.g. calling `write` on a
    /// `File` opened as `Read` will return an error at runtime).
    ///
    /// # Error
    ///
    /// This function will return an error under a number of different
    /// circumstances, to include but not limited to:
    ///
    /// * Opening a file that does not exist with `Read` access.
    /// * Attempting to open a file with a `FileAccess` that the user lacks
    ///   permissions for
    /// * Filesystem-level errors (full disk, etc)
    pub fn open_opts(path: &Path,
                     opts: &OpenOptions) -> io::Result<File> {
        let inner = try!(fs_imp::File::open(path, &opts.0));

        // On *BSD systems, we can open a directory as a file and read from
        // it: fd=open("/tmp", O_RDONLY); read(fd, buf, N); due to an old
        // tradition before the introduction of opendir(3).  We explicitly
        // reject it because there are few use cases.
        if cfg!(not(any(target_os = "linux", target_os = "android"))) &&
           try!(inner.file_attr()).is_dir() {
            Err(Error::new(ErrorKind::InvalidInput, "is a directory", None))
        } else {
            Ok(File { path: path.clone(), inner: inner })
        }
    }

    /// Attempts to open a file in read-only mode. This function is equivalent to
    /// `File::open_mode(path, Open, Read)`, and will raise all of the same
    /// errors that `File::open_mode` does.
    ///
    /// For more information, see the `File::open_mode` function.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::old_io::File;
    ///
    /// let contents = File::open(&Path::new("foo.txt")).read_to_end();
    /// ```
    pub fn open(path: &Path) -> io::Result<File> {
        File::open_opts(path, OpenOptions::new().read(true))
    }

    /// Attempts to create a file in write-only mode. This function is
    /// equivalent to `File::open_mode(path, Truncate, Write)`, and will
    /// raise all of the same errors that `File::open_mode` does.
    ///
    /// For more information, see the `File::open_mode` function.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #![allow(unused_must_use)]
    /// use std::old_io::File;
    ///
    /// let mut f = File::create(&Path::new("foo.txt"));
    /// f.write(b"This is a sample file");
    /// # drop(f);
    /// # ::std::old_io::fs::unlink(&Path::new("foo.txt"));
    /// ```
    pub fn create(path: &Path) -> io::Result<File> {
        File::open_opts(path, OpenOptions::new().write(true).create(true)
                                                .truncate(true))
    }

    /// Returns the original path that was used to open this file.
    pub fn path<'a>(&'a self) -> &'a Path {
        &self.path
    }

    /// This function is similar to `flush`, except that it may not synchronize
    /// file metadata to the filesystem. This is intended for use cases that
    /// must synchronize content, but don't need the metadata on disk. The goal
    /// of this method is to reduce disk operations.
    pub fn flush_data(&mut self) -> io::Result<()> {
        self.inner.datasync()
    }

    /// Either truncates or extends the underlying file, updating the size of
    /// this file to become `size`. This is equivalent to unix's `truncate`
    /// function.
    ///
    /// If the `size` is less than the current file's size, then the file will
    /// be shrunk. If it is greater than the current file's size, then the file
    /// will be extended to `size` and have all of the intermediate data filled
    /// in with 0s.
    pub fn truncate(&mut self, size: u64) -> io::Result<()> {
        self.inner.truncate(size)
    }

    /// Queries information about the underlying file.
    pub fn file_attr(&self) -> io::Result<FileAttr> {
        self.inner.file_attr().map(FileAttr)
    }
}

impl Read for File {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }
}
impl Write for File {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.inner.fsync()
    }
}
impl Seek for File {
    fn seek(&mut self, pos: SeekPos) -> io::Result<u64> {
        self.inner.seek(pos)
    }
}

impl OpenOptions {
    pub fn new() -> OpenOptions {
        OpenOptions(fs_imp::OpenOptions::new())
    }

    pub fn read(&mut self, read: bool) -> &mut OpenOptions {
        self.0.read(read); self
    }

    pub fn write(&mut self, write: bool) -> &mut OpenOptions {
        self.0.write(write); self
    }

    pub fn append(&mut self, append: bool) -> &mut OpenOptions {
        self.0.append(append); self
    }

    pub fn truncate(&mut self, truncate: bool) -> &mut OpenOptions {
        self.0.truncate(truncate); self
    }

    pub fn create(&mut self, create: bool) -> &mut OpenOptions {
        self.0.create(create); self
    }
}

impl FileAttr {
    pub fn is_dir(&self) -> bool { self.0.is_dir() }
    pub fn is_file(&self) -> bool { self.0.is_file() }
    pub fn size(&self) -> u64 { self.0.size() }
    pub fn perm(&self) -> FilePermission { FilePermission(self.0.perm()) }

    #[unstable = "return type may change (as well as name)"]
    pub fn accessed(&self) -> u64 { self.0.accessed() }
    #[unstable = "return type may change (as well as name)"]
    pub fn modified(&self) -> u64 { self.0.modified() }
}

impl FilePermission {
    pub fn readonly(&self) -> bool { self.0.readonly() }
    pub fn set_readonly(&mut self, readonly: bool) {
        self.0.set_readonly(readonly)
    }
}

impl Iterator for ReadDir {
    type Item = io::Result<DirEntry>;

    fn next(&mut self) -> Option<io::Result<DirEntry>> {
        self.0.next().map(|entry| entry.map(DirEntry))
    }
}

impl DirEntry {
    pub fn path(&self) -> Path { self.0.path() }
}

/// Unlink a file from the underlying filesystem.
///
/// # Example
///
/// ```rust
/// # #![allow(unused_must_use)]
/// use std::old_io::fs;
///
/// let p = Path::new("/some/file/path.txt");
/// fs::unlink(&p);
/// ```
///
/// Note that, just because an unlink call was successful, it is not
/// guaranteed that a file is immediately deleted (e.g. depending on
/// platform, other open file descriptors may prevent immediate removal)
///
/// # Error
///
/// This function will return an error if `path` points to a directory, if the
/// user lacks permissions to remove the file, or if some other filesystem-level
/// error occurs.
pub fn remove_file(path: &Path) -> io::Result<()> {
    fs_imp::unlink(path)
}

/// Given a path, query the file system to get information about a file,
/// directory, etc. This function will traverse symlinks to query
/// information about the destination file.
///
/// # Example
///
/// ```rust
/// use std::old_io::fs;
///
/// let p = Path::new("/some/file/path.txt");
/// match fs::stat(&p) {
///     Ok(stat) => { /* ... */ }
///     Err(e) => { /* handle error */ }
/// }
/// ```
///
/// # Error
///
/// This function will return an error if the user lacks the requisite permissions
/// to perform a `stat` call on the given `path` or if there is no entry in the
/// filesystem at the provided path.
pub fn file_attr(path: &Path) -> io::Result<FileAttr> {
    fs_imp::stat(path).map(FileAttr)
}

/// Rename a file or directory to a new name.
///
/// # Example
///
/// ```rust
/// # #![allow(unused_must_use)]
/// use std::old_io::fs;
///
/// fs::rename(&Path::new("foo"), &Path::new("bar"));
/// ```
///
/// # Error
///
/// This function will return an error if the provided `from` doesn't exist, if
/// the process lacks permissions to view the contents, or if some other
/// intermittent I/O error occurs.
pub fn rename(from: &Path, to: &Path) -> io::Result<()> {
    fs_imp::rename(from, to)
}

/// Copies the contents of one file to another. This function will also
/// copy the permission bits of the original file to the destination file.
///
/// Note that if `from` and `to` both point to the same file, then the file
/// will likely get truncated by this operation.
///
/// # Example
///
/// ```rust
/// # #![allow(unused_must_use)]
/// use std::old_io::fs;
///
/// fs::copy(&Path::new("foo.txt"), &Path::new("bar.txt"));
/// ```
///
/// # Error
///
/// This function will return an error in the following situations, but is not
/// limited to just these cases:
///
/// * The `from` path is not a file
/// * The `from` file does not exist
/// * The current process does not have the permission rights to access
///   `from` or write `to`
///
/// Note that this copy is not atomic in that once the destination is
/// ensured to not exist, there is nothing preventing the destination from
/// being created and then destroyed by this operation.
pub fn copy(from: &Path, to: &Path) -> io::Result<u64> {
    if !from.is_file() {
        return Err(Error::new(ErrorKind::MismatchedFileTypeForOperation,
                              "the source path is not an existing file",
                              None))
    }

    let mut reader = try!(File::open(from));
    let mut writer = try!(File::create(to));
    let perm = try!(reader.file_attr()).perm();

    let ret = try!(io::copy(&mut reader, &mut writer));
    try!(set_perm(to, perm));
    Ok(ret)
}

/// Creates a new hard link on the filesystem. The `dst` path will be a
/// link pointing to the `src` path. Note that systems often require these
/// two paths to both be located on the same filesystem.
pub fn hard_link(src: &Path, dst: &Path) -> io::Result<()> {
    fs_imp::link(src, dst)
}

/// Creates a new symbolic link on the filesystem. The `dst` path will be a
/// symlink pointing to the `src` path.
pub fn sym_link(src: &Path, dst: &Path) -> io::Result<()> {
    fs_imp::symlink(src, dst)
}

/// Reads a symlink, returning the file that the symlink points to.
///
/// # Error
///
/// This function will return an error on failure. Failure conditions include
/// reading a file that does not exist or reading a file that is not a symlink.
pub fn read_link(path: &Path) -> io::Result<Path> {
    fs_imp::readlink(path)
}

/// Create a new, empty directory at the provided path
///
/// # Example
///
/// ```rust
/// # #![allow(unused_must_use)]
/// use std::old_io;
/// use std::old_io::fs;
///
/// let p = Path::new("/some/dir");
/// fs::make_dir(&p);
/// ```
///
/// # Error
///
/// This function will return an error if the user lacks permissions to make a
/// new directory at the provided `path`, or if the directory already exists.
pub fn make_dir(path: &Path) -> io::Result<()> {
    fs_imp::mkdir(path)
}

/// Recursively create a directory and all of its parent components if they
/// are missing.
///
/// # Error
///
/// See `fs::mkdir`.
pub fn make_dir_all(path: &Path) -> io::Result<()> {
    if path.is_dir() { return Ok(()) }
    try!(make_dir_all(&path.dir_path()));
    make_dir(path)
}

/// Remove an existing, empty directory
///
/// # Example
///
/// ```rust
/// # #![allow(unused_must_use)]
/// use std::old_io::fs;
///
/// let p = Path::new("/some/dir");
/// fs::rmdir(&p);
/// ```
///
/// # Error
///
/// This function will return an error if the user lacks permissions to remove
/// the directory at the provided `path`, or if the directory isn't empty.
pub fn remove_dir(path: &Path) -> io::Result<()> {
    fs_imp::rmdir(path)
}

/// Removes a directory at this path, after removing all its contents. Use
/// carefully!
///
/// # Error
///
/// See `file::unlink` and `fs::readdir`
pub fn remove_dir_all(path: &Path) -> io::Result<()> {
    for child in try!(read_dir(path)) {
        let child = try!(child).path();
        let stat = try!(lstat(&child));
        if stat.is_dir() {
            try!(remove_dir_all(&child));
        } else {
            try!(remove_file(&child));
        }
    }
    return remove_dir(path);

    #[cfg(unix)]
    fn lstat(path: &Path) -> io::Result<fs_imp::FileAttr> { fs_imp::lstat(path) }
    #[cfg(windows)]
    fn lstat(path: &Path) -> io::Result<fs_imp::FileAttr> { fs_imp::stat(path) }
}

/// Retrieve a vector containing all entries within a provided directory
///
/// # Example
///
/// ```rust
/// use std::old_io::fs::PathExtensions;
/// use std::old_io::fs;
/// use std::old_io;
///
/// // one possible implementation of fs::walk_dir only visiting files
/// fn visit_dirs<F>(dir: &Path, cb: &mut F) -> io::Result<()> where
///     F: FnMut(&Path),
/// {
///     if dir.is_dir() {
///         let contents = try!(fs::readdir(dir));
///         for entry in contents.iter() {
///             if entry.is_dir() {
///                 try!(visit_dirs(entry, cb));
///             } else {
///                 (*cb)(entry);
///             }
///         }
///         Ok(())
///     } else {
///         Err(old_io::standard_error(old_io::InvalidInput))
///     }
/// }
/// ```
///
/// # Error
///
/// This function will return an error if the provided `path` doesn't exist, if
/// the process lacks permissions to view the contents or if the `path` points
/// at a non-directory file
pub fn read_dir(path: &Path) -> io::Result<ReadDir> {
    fs_imp::readdir(path).map(ReadDir)
}

/// Returns an iterator that will recursively walk the directory structure
/// rooted at `path`. The path given will not be iterated over, and this will
/// perform iteration in some top-down order.  The contents of unreadable
/// subdirectories are ignored.
pub fn walk_dir(path: &Path) -> io::Result<WalkDir> {
    let start = try!(read_dir(path));
    Ok(WalkDir { cur: Some(start), stack: Vec::new() })
}

/// An iterator that walks over a directory
pub struct WalkDir {
    cur: Option<ReadDir>,
    stack: Vec<io::Result<ReadDir>>,
}

impl Iterator for WalkDir {
    type Item = io::Result<DirEntry>;

    fn next(&mut self) -> Option<io::Result<DirEntry>> {
        loop {
            if let Some(ref mut cur) = self.cur {
                match cur.next() {
                    Some(Err(e)) => return Some(Err(e)),
                    Some(Ok(next)) => {
                        let path = next.path();
                        if path.is_dir() {
                            self.stack.push(read_dir(&path));
                        }
                        return Some(Ok(next))
                    }
                    None => {}
                }
            }
            self.cur = None;
            match self.stack.pop() {
                Some(Err(e)) => return Some(Err(e)),
                Some(Ok(next)) => self.cur = Some(next),
                None => return None,
            }
        }
    }
}

/// Utility methods for paths.
pub trait PathExt {
    /// Get information on the file, directory, etc at this path.
    ///
    /// Consult the `fs::stat` documentation for more info.
    ///
    /// This call preserves identical runtime/error semantics with `file::stat`.
    fn file_attr(&self) -> io::Result<FileAttr>;

    /// Boolean value indicator whether the underlying file exists on the local
    /// filesystem. Returns false in exactly the cases where `fs::stat` fails.
    fn exists(&self) -> bool;

    /// Whether the underlying implementation (be it a file path, or something
    /// else) points at a "regular file" on the FS. Will return false for paths
    /// to non-existent locations or directories or other non-regular files
    /// (named pipes, etc). Follows links when making this determination.
    fn is_file(&self) -> bool;

    /// Whether the underlying implementation (be it a file path, or something
    /// else) is pointing at a directory in the underlying FS. Will return
    /// false for paths to non-existent locations or if the item is not a
    /// directory (eg files, named pipes, etc). Follows links when making this
    /// determination.
    fn is_dir(&self) -> bool;
}

impl PathExt for Path {
    fn file_attr(&self) -> io::Result<FileAttr> { file_attr(self) }

    fn exists(&self) -> bool { file_attr(self).is_ok() }

    fn is_file(&self) -> bool {
        file_attr(self).map(|s| s.is_file()).unwrap_or(false)
    }
    fn is_dir(&self) -> bool {
        file_attr(self).map(|s| s.is_dir()).unwrap_or(false)
    }
}

/// Changes the timestamps for a file's last modification and access time.
/// The file at the path specified will have its last access time set to
/// `atime` and its modification time set to `mtime`. The times specified should
/// be in milliseconds.
#[unstable = "argument types and argument counts may change"]
pub fn change_file_times(path: &Path, atime: u64, mtime: u64) -> io::Result<()> {
    fs_imp::utimes(path, atime, mtime)
}

/// Changes the permission mode bits found on a file or a directory. This
/// function takes a mask from the `io` module
///
/// # Example
///
/// ```rust
/// # #![allow(unused_must_use)]
/// use std::old_io;
/// use std::old_io::fs;
///
/// fs::chmod(&Path::new("file.txt"), old_io::USER_FILE);
/// fs::chmod(&Path::new("file.txt"), old_io::USER_READ | old_io::USER_WRITE);
/// fs::chmod(&Path::new("dir"),      old_io::USER_DIR);
/// fs::chmod(&Path::new("file.exe"), old_io::USER_EXEC);
/// ```
///
/// # Error
///
/// This function will return an error if the provided `path` doesn't exist, if
/// the process lacks permissions to change the attributes of the file, or if
/// some other I/O error is encountered.
pub fn set_perm(path: &Path, perm: FilePermission) -> io::Result<()> {
    fs_imp::set_perm(path, perm.0)
}
