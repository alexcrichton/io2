// Copyright 2012-2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Higher-level interfaces to libc::* functions and operating system services.
//!
//! In general these take and return rust types, use rust idioms (enums,
//! closures, vectors) rather than C idioms, and do more extensive safety
//! checks.
//!
//! This module is not meant to only contain 1:1 mappings to libc entries; any
//! os-interface code that is reasonably useful and broadly applicable can go
//! here. Including utility routines that merely build on other os code.
//!
//! We assume the general case is that users do not care, and do not want to be
//! made to care, which operating system they are on. While they may want to
//! special case various special cases -- and so we will not _hide_ the facts of
//! which OS the user is on -- they should be given the opportunity to write
//! OS-ignorant code by default.

use prelude::v1::*;

use error::Error;
use ffi::{OsString, AsOsStr};
use fmt;
use io;
use sync::atomic::{AtomicIsize, ATOMIC_ISIZE_INIT, Ordering};
use sync::{StaticMutex, MUTEX_INIT};
use sys::os as os_imp;

/// Returns the current working directory as a `Path`.
///
/// # Errors
///
/// Returns an `Err` if the current working directory value is invalid.
/// Possible cases:
///
/// * Current directory does not exist.
/// * There are insufficient permissions to access the current directory.
/// * The internal buffer is not large enough to hold the path.
///
/// # Example
///
/// ```rust
/// use std::env;
///
/// // We assume that we are in a valid directory.
/// let p = env::current_dir().unwrap();
/// println!("The current directory is {}", p.display());
/// ```
pub fn current_dir() -> io::Result<Path> {
    os_imp::getcwd()

}
/// Changes the current working directory to the specified path, returning
/// whether the change was completed successfully or not.
///
/// # Example
///
/// ```rust
/// use std::env;
/// use std::path::Path;
///
/// let root = Path::new("/");
/// assert!(env::set_current_dir(&root).is_ok());
/// println!("Successfully changed working directory to {}!", root.display());
/// ```
pub fn set_current_dir(p: &Path) -> io::Result<()> {
    os_imp::chdir(p)
}

static ENV_LOCK: StaticMutex = MUTEX_INIT;

/// An iterator over a snapshot of the environment variables of this process.
///
/// This iterator is created through `std::env::vars()` and yields `(OsString,
/// OsString)` pairs.
pub struct Vars { inner: os_imp::Env }

/// Returns an iterator of (variable, value) pairs, for all the environment
/// variables of the current process.
///
/// The returned iterator contains a snapshot of the process's environment
/// variables at the time of this invocation, modifications to environment
/// variables afterwards will not be reflected in the returned iterator.
///
/// # Example
///
/// ```rust
/// use std::env;
///
/// // We will iterate through the references to the element returned by
/// // env::vars();
/// for (key, value) in env::vars() {
///     println!("{:?}: {:?}", key, value);
/// }
/// ```
pub fn vars() -> Vars {
    let _g = ENV_LOCK.lock();
    Vars { inner: os_imp::env() }
}

impl Iterator for Vars {
    type Item = (OsString, OsString);
    fn next(&mut self) -> Option<(OsString, OsString)> { self.inner.next() }
    fn size_hint(&self) -> (usize, Option<usize>) { self.inner.size_hint() }
}

/// Fetches the environment variable `n` from the current process, returning
/// None if the variable isn't set.
///
/// # Example
///
/// ```rust
/// use std::env;
///
/// let key = "HOME";
/// match env::var(key) {
///     Some(val) => println!("{}: {:?}", key, val),
///     None => println!("{} is not defined in the environment.", key)
/// }
/// ```
pub fn var<K: ?Sized>(key: &K) -> Option<OsString> where K: AsOsStr {
    let _g = ENV_LOCK.lock();
    os_imp::getenv(key.as_os_str())
}

/// Sets the environment variable `n` to the value `v` for the currently running
/// process.
///
/// # Example
///
/// ```rust
/// use std::env;
///
/// let key = "KEY";
/// env::set_var(key, "VALUE");
/// match os::var(key) {
///     Some(ref val) => println!("{}: {:?}", key, val),
///     None => println!("{} is not defined in the environment.", key)
/// }
/// ```
pub fn set_var<K: ?Sized, V: ?Sized>(k: &K, v: &V)
    where K: AsOsStr, V: AsOsStr
{
    let _g = ENV_LOCK.lock();
    os_imp::setenv(k.as_os_str(), v.as_os_str())
}

/// Remove a variable from the environment entirely.
pub fn remove_var<K: ?Sized>(k: &K) where K: AsOsStr {
    let _g = ENV_LOCK.lock();
    os_imp::unsetenv(k.as_os_str())
}

/// An iterator over `Path` instances for parsing an environment variable
/// according to platform-specific conventions.
///
/// This structure is returned from `std::env::split_paths`.
pub struct SplitPaths<'a> { inner: os_imp::SplitPaths<'a> }

/// Parses input according to platform conventions for the `PATH`
/// environment variable.
///
/// Returns an iterator over the paths contained in `unparsed`.
///
/// # Example
///
/// ```rust
/// use std::env;
///
/// let key = "PATH";
/// match env::var(key) {
///     Some(paths) => {
///         for path in env::split_paths(&paths) {
///             println!("'{}'", path.display());
///         }
///     }
///     None => println!("{} is not defined in the environment.", key)
/// }
/// ```
pub fn split_paths<T: AsOsStr + ?Sized>(unparsed: &T) -> SplitPaths {
    SplitPaths { inner: os_imp::split_paths(unparsed.as_os_str()) }
}

impl<'a> Iterator for SplitPaths<'a> {
    type Item = Path;
    fn next(&mut self) -> Option<Path> { self.inner.next() }
    fn size_hint(&self) -> (usize, Option<usize>) { self.inner.size_hint() }
}

/// Error type returned from `std::env::join_paths` when paths fail to be
/// joined.
#[derive(Show)]
pub struct JoinPathsError {
    inner: os_imp::JoinPathsError
}

/// Joins a collection of `Path`s appropriately for the `PATH`
/// environment variable.
///
/// Returns an `OsString` on success.
///
/// Returns an `Err` (containing an error message) if one of the input
/// `Path`s contains an invalid character for constructing the `PATH`
/// variable (a double quote on Windows or a colon on Unix).
///
/// # Example
///
/// ```rust
/// use std::env;
///
/// let key = "PATH";
/// let mut paths = env::var(key).map_or(Vec::new(), env::split_paths);
/// paths.push(Path::new("/home/xyz/bin"));
/// env::setenv(key, env::join_paths(paths.as_slice()).unwrap());
/// ```
pub fn join_paths<'a, I, T: ?Sized>(paths: I) -> Result<OsString, JoinPathsError>
    where I: Iterator<Item=&'a T>, T: AsOsStr
{
    os_imp::join_paths(paths.map(|s| s.as_os_str())).map_err(|e| {
        JoinPathsError { inner: e }
    })
}

impl fmt::Display for JoinPathsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.inner.fmt(f)
    }
}

impl Error for JoinPathsError {
    fn description(&self) -> &str { self.inner.description() }
}

/// Optionally returns the path to the current user's home directory if known.
///
/// # Unix
///
/// Returns the value of the 'HOME' environment variable if it is set
/// and not equal to the empty string.
///
/// # Windows
///
/// Returns the value of the 'HOME' environment variable if it is
/// set and not equal to the empty string. Otherwise, returns the value of the
/// 'USERPROFILE' environment variable if it is set and not equal to the empty
/// string.
///
/// # Example
///
/// ```rust
/// use std::env;
///
/// match env::homedir() {
///     Some(ref p) => println!("{}", p.display()),
///     None => println!("Impossible to get your home dir!")
/// }
/// ```
pub fn home_dir() -> Option<Path> {
    os_imp::homedi()
}

/// Returns the path to a temporary directory.
///
/// On Unix, returns the value of the 'TMPDIR' environment variable if it is
/// set, otherwise for non-Android it returns '/tmp'. If Android, since there
/// is no global temporary folder (it is usually allocated per-app), we return
/// '/data/local/tmp'.
///
/// On Windows, returns the value of, in order, the 'TMP', 'TEMP',
/// 'USERPROFILE' environment variable  if any are set and not the empty
/// string. Otherwise, tmpdir returns the path to the Windows directory.
pub fn temp_dir() -> Path {
    if cfg!(unix) {
        env_path("TMPDIR").unwrap_or(if cfg!(target_os = "android") {
            Path::new("/data/local/tmp")
        } else {
            Path::new("/tmp")
        })
    } else {
        env_path("TMP").or(env_path("TEMP"))
                       .or(env_path("USERPROFILE"))
                       .or(env_path("WINDIR"))
                       .unwrap_or(Path::new("C:\\Windows"))
    }
}

fn env_path(key: &str) -> Option<Path> {
    var(key).and_then(|p| {
        // TODO: Path should be constructible from OsString
        let s = p.to_str().unwrap();
        if s.is_empty() {None} else {Some(Path::new(s))}
    })
}

/// Optionally returns the filesystem path to the current executable which is
/// running but with the executable name.
///
/// # Examples
///
/// ```rust
/// use std::env;
///
/// match env::current_exe() {
///     Some(exe_path) => println!("Path of this executable is: {}",
///                                exe_path.display()),
///     None => println!("Unable to get the path of this executable!")
/// };
/// ```
pub fn current_exe() -> Option<Path> {
    os_imp::current_exe()
}

static EXIT_STATUS: AtomicIsize = ATOMIC_ISIZE_INIT;

/// Sets the process exit code
///
/// Sets the exit code returned by the process if all supervised tasks
/// terminate successfully (without panicking). If the current root task panics
/// and is supervised by the scheduler then any user-specified exit status is
/// ignored and the process exits with the default panic status.
///
/// Note that this is not synchronized against modifications of other threads.
#[unstable]
pub fn set_exit_status(code: i32) {
    EXIT_STATUS.store(code as isize, Ordering::SeqCst)
}

/// Fetches the process's current exit code. This defaults to 0 and can change
/// by calling `set_exit_status`.
#[unstable]
pub fn get_exit_status() -> i32 {
    EXIT_STATUS.load(Ordering::SeqCst) as i32
}

/// An iterator over the arguments of a process, yielding an `OsString` value
/// for each argument.
///
/// This structure is created through the `std::env::args` method.
pub struct Args { inner: os_imp::Args }

/// Returns the arguments which this program was started with (normally passed
/// via the command line).
///
/// The first element is traditionally the path to the executable, but it can be
/// set to arbitrary text, and it may not even exist, so this property should
/// not be relied upon for security purposes.
///
/// # Example
///
/// ```rust
/// use std::env;
///
/// // Prints each argument on a separate line
/// for argument in env::args() {
///     println!("{:?}", argument);
/// }
/// ```
pub fn args() -> Args {
    Args { inner: os_imp::args() }
}

impl Iterator for Args {
    type Item = OsString;
    fn next(&mut self) -> Option<OsString> { self.inner.next() }
    fn size_hint(&self) -> (usize, Option<usize>) { self.inner.size_hint() }
}

/// Returns the page size of the current architecture in bytes.
pub fn page_size() -> usize {
    os_imp::page_size()
}

#[cfg(target_os = "linux")]
pub mod consts {
    pub use super::arch_consts::ARCH;

    pub const FAMILY: &'static str = "unix";

    /// A string describing the specific operating system in use: in this
    /// case, `linux`.
    pub const OS: &'static str = "linux";

    /// Specifies the filename prefix used for shared libraries on this
    /// platform: in this case, `lib`.
    pub const DLL_PREFIX: &'static str = "lib";

    /// Specifies the filename suffix used for shared libraries on this
    /// platform: in this case, `.so`.
    pub const DLL_SUFFIX: &'static str = ".so";

    /// Specifies the file extension used for shared libraries on this
    /// platform that goes after the dot: in this case, `so`.
    pub const DLL_EXTENSION: &'static str = "so";

    /// Specifies the filename suffix used for executable binaries on this
    /// platform: in this case, the empty string.
    pub const EXE_SUFFIX: &'static str = "";

    /// Specifies the file extension, if any, used for executable binaries
    /// on this platform: in this case, the empty string.
    pub const EXE_EXTENSION: &'static str = "";
}

#[cfg(target_os = "macos")]
pub mod consts {
    pub use super::arch_consts::ARCH;

    pub const FAMILY: &'static str = "unix";

    /// A string describing the specific operating system in use: in this
    /// case, `macos`.
    pub const OS: &'static str = "macos";

    /// Specifies the filename prefix used for shared libraries on this
    /// platform: in this case, `lib`.
    pub const DLL_PREFIX: &'static str = "lib";

    /// Specifies the filename suffix used for shared libraries on this
    /// platform: in this case, `.dylib`.
    pub const DLL_SUFFIX: &'static str = ".dylib";

    /// Specifies the file extension used for shared libraries on this
    /// platform that goes after the dot: in this case, `dylib`.
    pub const DLL_EXTENSION: &'static str = "dylib";

    /// Specifies the filename suffix used for executable binaries on this
    /// platform: in this case, the empty string.
    pub const EXE_SUFFIX: &'static str = "";

    /// Specifies the file extension, if any, used for executable binaries
    /// on this platform: in this case, the empty string.
    pub const EXE_EXTENSION: &'static str = "";
}

#[cfg(target_os = "ios")]
pub mod consts {
    pub use super::arch_consts::ARCH;

    pub const FAMILY: &'static str = "unix";

    /// A string describing the specific operating system in use: in this
    /// case, `ios`.
    pub const OS: &'static str = "ios";

    /// Specifies the filename suffix used for executable binaries on this
    /// platform: in this case, the empty string.
    pub const EXE_SUFFIX: &'static str = "";

    /// Specifies the file extension, if any, used for executable binaries
    /// on this platform: in this case, the empty string.
    pub const EXE_EXTENSION: &'static str = "";
}

#[cfg(target_os = "freebsd")]
pub mod consts {
    pub use super::arch_consts::ARCH;

    pub const FAMILY: &'static str = "unix";

    /// A string describing the specific operating system in use: in this
    /// case, `freebsd`.
    pub const OS: &'static str = "freebsd";

    /// Specifies the filename prefix used for shared libraries on this
    /// platform: in this case, `lib`.
    pub const DLL_PREFIX: &'static str = "lib";

    /// Specifies the filename suffix used for shared libraries on this
    /// platform: in this case, `.so`.
    pub const DLL_SUFFIX: &'static str = ".so";

    /// Specifies the file extension used for shared libraries on this
    /// platform that goes after the dot: in this case, `so`.
    pub const DLL_EXTENSION: &'static str = "so";

    /// Specifies the filename suffix used for executable binaries on this
    /// platform: in this case, the empty string.
    pub const EXE_SUFFIX: &'static str = "";

    /// Specifies the file extension, if any, used for executable binaries
    /// on this platform: in this case, the empty string.
    pub const EXE_EXTENSION: &'static str = "";
}

#[cfg(target_os = "dragonfly")]
pub mod consts {
    pub use super::arch_consts::ARCH;

    pub const FAMILY: &'static str = "unix";

    /// A string describing the specific operating system in use: in this
    /// case, `dragonfly`.
    pub const OS: &'static str = "dragonfly";

    /// Specifies the filename prefix used for shared libraries on this
    /// platform: in this case, `lib`.
    pub const DLL_PREFIX: &'static str = "lib";

    /// Specifies the filename suffix used for shared libraries on this
    /// platform: in this case, `.so`.
    pub const DLL_SUFFIX: &'static str = ".so";

    /// Specifies the file extension used for shared libraries on this
    /// platform that goes after the dot: in this case, `so`.
    pub const DLL_EXTENSION: &'static str = "so";

    /// Specifies the filename suffix used for executable binaries on this
    /// platform: in this case, the empty string.
    pub const EXE_SUFFIX: &'static str = "";

    /// Specifies the file extension, if any, used for executable binaries
    /// on this platform: in this case, the empty string.
    pub const EXE_EXTENSION: &'static str = "";
}

#[cfg(target_os = "android")]
pub mod consts {
    pub use super::arch_consts::ARCH;

    pub const FAMILY: &'static str = "unix";

    /// A string describing the specific operating system in use: in this
    /// case, `android`.
    pub const OS: &'static str = "android";

    /// Specifies the filename prefix used for shared libraries on this
    /// platform: in this case, `lib`.
    pub const DLL_PREFIX: &'static str = "lib";

    /// Specifies the filename suffix used for shared libraries on this
    /// platform: in this case, `.so`.
    pub const DLL_SUFFIX: &'static str = ".so";

    /// Specifies the file extension used for shared libraries on this
    /// platform that goes after the dot: in this case, `so`.
    pub const DLL_EXTENSION: &'static str = "so";

    /// Specifies the filename suffix used for executable binaries on this
    /// platform: in this case, the empty string.
    pub const EXE_SUFFIX: &'static str = "";

    /// Specifies the file extension, if any, used for executable binaries
    /// on this platform: in this case, the empty string.
    pub const EXE_EXTENSION: &'static str = "";
}

#[cfg(target_os = "windows")]
pub mod consts {
    pub use super::arch_consts::ARCH;

    pub const FAMILY: &'static str = "windows";

    /// A string describing the specific operating system in use: in this
    /// case, `windows`.
    pub const OS: &'static str = "windows";

    /// Specifies the filename prefix used for shared libraries on this
    /// platform: in this case, the empty string.
    pub const DLL_PREFIX: &'static str = "";

    /// Specifies the filename suffix used for shared libraries on this
    /// platform: in this case, `.dll`.
    pub const DLL_SUFFIX: &'static str = ".dll";

    /// Specifies the file extension used for shared libraries on this
    /// platform that goes after the dot: in this case, `dll`.
    pub const DLL_EXTENSION: &'static str = "dll";

    /// Specifies the filename suffix used for executable binaries on this
    /// platform: in this case, `.exe`.
    pub const EXE_SUFFIX: &'static str = ".exe";

    /// Specifies the file extension, if any, used for executable binaries
    /// on this platform: in this case, `exe`.
    pub const EXE_EXTENSION: &'static str = "exe";
}

#[cfg(target_arch = "x86")]
mod arch_consts {
    pub const ARCH: &'static str = "x86";
}

#[cfg(target_arch = "x86_64")]
mod arch_consts {
    pub const ARCH: &'static str = "x86_64";
}

#[cfg(target_arch = "arm")]
mod arch_consts {
    pub const ARCH: &'static str = "arm";
}

#[cfg(target_arch = "aarch64")]
mod arch_consts {
    pub const ARCH: &'static str = "aarch64";
}

#[cfg(target_arch = "mips")]
mod arch_consts {
    pub const ARCH: &'static str = "mips";
}

#[cfg(target_arch = "mipsel")]
mod arch_consts {
    pub const ARCH: &'static str = "mipsel";
}

#[cfg(target_arch = "powerpc")]
mod arch_consts {
    pub const ARCH: &'static str = "powerpc";
}
