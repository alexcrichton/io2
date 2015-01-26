
// /// A memory mapped file or chunk of memory. This is a very system-specific
// /// interface to the OS's memory mapping facilities (`mmap` on POSIX,
// /// `VirtualAlloc`/`CreateFileMapping` on Windows). It makes no attempt at
// /// abstracting platform differences, besides in error values returned. Consider
// /// yourself warned.
// ///
// /// The memory map is released (unmapped) when the destructor is run, so don't
// /// let it leave scope by accident if you want it to stick around.
// #[allow(missing_copy_implementations)]
// pub struct MemoryMap {
//     data: *mut u8,
//     len: uint,
//     kind: MemoryMapKind,
// }
//
// /// Type of memory map
// pub enum MemoryMapKind {
//     /// Virtual memory map. Usually used to change the permissions of a given
//     /// chunk of memory.  Corresponds to `VirtualAlloc` on Windows.
//     MapFile(*const u8),
//     /// Virtual memory map. Usually used to change the permissions of a given
//     /// chunk of memory, or for allocation. Corresponds to `VirtualAlloc` on
//     /// Windows.
//     MapVirtual
// }
//
// impl Copy for MemoryMapKind {}
//
// /// Options the memory map is created with
// pub enum MapOption {
//     /// The memory should be readable
//     MapReadable,
//     /// The memory should be writable
//     MapWritable,
//     /// The memory should be executable
//     MapExecutable,
//     /// Create a map for a specific address range. Corresponds to `MAP_FIXED` on
//     /// POSIX.
//     MapAddr(*const u8),
//     /// Create a memory mapping for a file with a given HANDLE.
//     #[cfg(windows)]
//     MapFd(libc::HANDLE),
//     /// Create a memory mapping for a file with a given fd.
//     #[cfg(not(windows))]
//     MapFd(c_int),
//     /// When using `MapFd`, the start of the map is `uint` bytes from the start
//     /// of the file.
//     MapOffset(uint),
//     /// On POSIX, this can be used to specify the default flags passed to
//     /// `mmap`. By default it uses `MAP_PRIVATE` and, if not using `MapFd`,
//     /// `MAP_ANON`. This will override both of those. This is platform-specific
//     /// (the exact values used) and ignored on Windows.
//     MapNonStandardFlags(c_int),
// }
//
// impl Copy for MapOption {}
//
// /// Possible errors when creating a map.
// #[derive(Copy, Show)]
// pub enum MapError {
//     /// # The following are POSIX-specific
//     ///
//     /// fd was not open for reading or, if using `MapWritable`, was not open for
//     /// writing.
//     ErrFdNotAvail,
//     /// fd was not valid
//     ErrInvalidFd,
//     /// Either the address given by `MapAddr` or offset given by `MapOffset` was
//     /// not a multiple of `MemoryMap::granularity` (unaligned to page size).
//     ErrUnaligned,
//     /// With `MapFd`, the fd does not support mapping.
//     ErrNoMapSupport,
//     /// If using `MapAddr`, the address + `min_len` was outside of the process's
//     /// address space. If using `MapFd`, the target of the fd didn't have enough
//     /// resources to fulfill the request.
//     ErrNoMem,
//     /// A zero-length map was requested. This is invalid according to
//     /// [POSIX](http://pubs.opengroup.org/onlinepubs/9699919799/functions/mmap.html).
//     /// Not all platforms obey this, but this wrapper does.
//     ErrZeroLength,
//     /// Unrecognized error. The inner value is the unrecognized errno.
//     ErrUnknown(int),
//     /// # The following are Windows-specific
//     ///
//     /// Unsupported combination of protection flags
//     /// (`MapReadable`/`MapWritable`/`MapExecutable`).
//     ErrUnsupProt,
//     /// When using `MapFd`, `MapOffset` was given (Windows does not support this
//     /// at all)
//     ErrUnsupOffset,
//     /// When using `MapFd`, there was already a mapping to the file.
//     ErrAlreadyExists,
//     /// Unrecognized error from `VirtualAlloc`. The inner value is the return
//     /// value of GetLastError.
//     ErrVirtualAlloc(uint),
//     /// Unrecognized error from `CreateFileMapping`. The inner value is the
//     /// return value of `GetLastError`.
//     ErrCreateFileMappingW(uint),
//     /// Unrecognized error from `MapViewOfFile`. The inner value is the return
//     /// value of `GetLastError`.
//     ErrMapViewOfFile(uint)
// }
//
// #[stable]
// impl fmt::Display for MapError {
//     fn fmt(&self, out: &mut fmt::Formatter) -> fmt::Result {
//         let str = match *self {
//             ErrFdNotAvail => "fd not available for reading or writing",
//             ErrInvalidFd => "Invalid fd",
//             ErrUnaligned => {
//                 "Unaligned address, invalid flags, negative length or \
//                  unaligned offset"
//             }
//             ErrNoMapSupport=> "File doesn't support mapping",
//             ErrNoMem => "Invalid address, or not enough available memory",
//             ErrUnsupProt => "Protection mode unsupported",
//             ErrUnsupOffset => "Offset in virtual memory mode is unsupported",
//             ErrAlreadyExists => "File mapping for specified file already exists",
//             ErrZeroLength => "Zero-length mapping not allowed",
//             ErrUnknown(code) => {
//                 return write!(out, "Unknown error = {}", code)
//             },
//             ErrVirtualAlloc(code) => {
//                 return write!(out, "VirtualAlloc failure = {}", code)
//             },
//             ErrCreateFileMappingW(code) => {
//                 return write!(out, "CreateFileMappingW failure = {}", code)
//             },
//             ErrMapViewOfFile(code) => {
//                 return write!(out, "MapViewOfFile failure = {}", code)
//             }
//         };
//         write!(out, "{}", str)
//     }
// }
//
// impl Error for MapError {
//     fn description(&self) -> &str { "memory map error" }
// }
//
// // Round up `from` to be divisible by `to`
// fn round_up(from: uint, to: uint) -> uint {
//     let r = if from % to == 0 {
//         from
//     } else {
//         from + to - (from % to)
//     };
//     if r == 0 {
//         to
//     } else {
//         r
//     }
// }
//
// #[cfg(unix)]
// impl MemoryMap {
//     /// Create a new mapping with the given `options`, at least `min_len` bytes
//     /// long. `min_len` must be greater than zero; see the note on
//     /// `ErrZeroLength`.
//     pub fn new(min_len: uint, options: &[MapOption]) -> Result<MemoryMap, MapError> {
//         use libc::off_t;
//
//         if min_len == 0 {
//             return Err(ErrZeroLength)
//         }
//         let mut addr: *const u8 = ptr::null();
//         let mut prot = 0;
//         let mut flags = libc::MAP_PRIVATE;
//         let mut fd = -1;
//         let mut offset = 0;
//         let mut custom_flags = false;
//         let len = round_up(min_len, page_size());
//
//         for &o in options.iter() {
//             match o {
//                 MapReadable => { prot |= libc::PROT_READ; },
//                 MapWritable => { prot |= libc::PROT_WRITE; },
//                 MapExecutable => { prot |= libc::PROT_EXEC; },
//                 MapAddr(addr_) => {
//                     flags |= libc::MAP_FIXED;
//                     addr = addr_;
//                 },
//                 MapFd(fd_) => {
//                     flags |= libc::MAP_FILE;
//                     fd = fd_;
//                 },
//                 MapOffset(offset_) => { offset = offset_ as off_t; },
//                 MapNonStandardFlags(f) => { custom_flags = true; flags = f },
//             }
//         }
//         if fd == -1 && !custom_flags { flags |= libc::MAP_ANON; }
//
//         let r = unsafe {
//             libc::mmap(addr as *mut c_void, len as libc::size_t, prot, flags,
//                        fd, offset)
//         };
//         if r == libc::MAP_FAILED {
//             Err(match errno() as c_int {
//                 libc::EACCES => ErrFdNotAvail,
//                 libc::EBADF => ErrInvalidFd,
//                 libc::EINVAL => ErrUnaligned,
//                 libc::ENODEV => ErrNoMapSupport,
//                 libc::ENOMEM => ErrNoMem,
//                 code => ErrUnknown(code as int)
//             })
//         } else {
//             Ok(MemoryMap {
//                data: r as *mut u8,
//                len: len,
//                kind: if fd == -1 {
//                    MapVirtual
//                } else {
//                    MapFile(ptr::null())
//                }
//             })
//         }
//     }
//
//     /// Granularity that the offset or address must be for `MapOffset` and
//     /// `MapAddr` respectively.
//     pub fn granularity() -> uint {
//         page_size()
//     }
// }
//
// #[cfg(unix)]
// impl Drop for MemoryMap {
//     /// Unmap the mapping. Panics the task if `munmap` panics.
//     fn drop(&mut self) {
//         if self.len == 0 { /* workaround for dummy_stack */ return; }
//
//         unsafe {
//             // `munmap` only panics due to logic errors
//             libc::munmap(self.data as *mut c_void, self.len as libc::size_t);
//         }
//     }
// }
//
// #[cfg(windows)]
// impl MemoryMap {
//     /// Create a new mapping with the given `options`, at least `min_len` bytes long.
//     pub fn new(min_len: uint, options: &[MapOption]) -> Result<MemoryMap, MapError> {
//         use libc::types::os::arch::extra::{LPVOID, DWORD, SIZE_T, HANDLE};
//
//         let mut lpAddress: LPVOID = ptr::null_mut();
//         let mut readable = false;
//         let mut writable = false;
//         let mut executable = false;
//         let mut handle: HANDLE = libc::INVALID_HANDLE_VALUE;
//         let mut offset: uint = 0;
//         let len = round_up(min_len, page_size());
//
//         for &o in options.iter() {
//             match o {
//                 MapReadable => { readable = true; },
//                 MapWritable => { writable = true; },
//                 MapExecutable => { executable = true; }
//                 MapAddr(addr_) => { lpAddress = addr_ as LPVOID; },
//                 MapFd(handle_) => { handle = handle_; },
//                 MapOffset(offset_) => { offset = offset_; },
//                 MapNonStandardFlags(..) => {}
//             }
//         }
//
//         let flProtect = match (executable, readable, writable) {
//             (false, false, false) if handle == libc::INVALID_HANDLE_VALUE => libc::PAGE_NOACCESS,
//             (false, true, false) => libc::PAGE_READONLY,
//             (false, true, true) => libc::PAGE_READWRITE,
//             (true, false, false) if handle == libc::INVALID_HANDLE_VALUE => libc::PAGE_EXECUTE,
//             (true, true, false) => libc::PAGE_EXECUTE_READ,
//             (true, true, true) => libc::PAGE_EXECUTE_READWRITE,
//             _ => return Err(ErrUnsupProt)
//         };
//
//         if handle == libc::INVALID_HANDLE_VALUE {
//             if offset != 0 {
//                 return Err(ErrUnsupOffset);
//             }
//             let r = unsafe {
//                 libc::VirtualAlloc(lpAddress,
//                                    len as SIZE_T,
//                                    libc::MEM_COMMIT | libc::MEM_RESERVE,
//                                    flProtect)
//             };
//             match r as uint {
//                 0 => Err(ErrVirtualAlloc(errno())),
//                 _ => Ok(MemoryMap {
//                    data: r as *mut u8,
//                    len: len,
//                    kind: MapVirtual
//                 })
//             }
//         } else {
//             let dwDesiredAccess = match (executable, readable, writable) {
//                 (false, true, false) => libc::FILE_MAP_READ,
//                 (false, true, true) => libc::FILE_MAP_WRITE,
//                 (true, true, false) => libc::FILE_MAP_READ | libc::FILE_MAP_EXECUTE,
//                 (true, true, true) => libc::FILE_MAP_WRITE | libc::FILE_MAP_EXECUTE,
//                 _ => return Err(ErrUnsupProt) // Actually, because of the check above,
//                                               // we should never get here.
//             };
//             unsafe {
//                 let hFile = handle;
//                 let mapping = libc::CreateFileMappingW(hFile,
//                                                        ptr::null_mut(),
//                                                        flProtect,
//                                                        0,
//                                                        0,
//                                                        ptr::null());
//                 if mapping == ptr::null_mut() {
//                     return Err(ErrCreateFileMappingW(errno()));
//                 }
//                 if errno() as c_int == libc::ERROR_ALREADY_EXISTS {
//                     return Err(ErrAlreadyExists);
//                 }
//                 let r = libc::MapViewOfFile(mapping,
//                                             dwDesiredAccess,
//                                             ((len as u64) >> 32) as DWORD,
//                                             (offset & 0xffff_ffff) as DWORD,
//                                             0);
//                 match r as uint {
//                     0 => Err(ErrMapViewOfFile(errno())),
//                     _ => Ok(MemoryMap {
//                        data: r as *mut u8,
//                        len: len,
//                        kind: MapFile(mapping as *const u8)
//                     })
//                 }
//             }
//         }
//     }
//
//     /// Granularity of MapAddr() and MapOffset() parameter values.
//     /// This may be greater than the value returned by page_size().
//     pub fn granularity() -> uint {
//         use mem;
//         unsafe {
//             let mut info = mem::zeroed();
//             libc::GetSystemInfo(&mut info);
//
//             return info.dwAllocationGranularity as uint;
//         }
//     }
// }
//
// #[cfg(windows)]
// impl Drop for MemoryMap {
//     /// Unmap the mapping. Panics the task if any of `VirtualFree`,
//     /// `UnmapViewOfFile`, or `CloseHandle` fail.
//     fn drop(&mut self) {
//         use libc::types::os::arch::extra::{LPCVOID, HANDLE};
//         use libc::consts::os::extra::FALSE;
//         if self.len == 0 { return }
//
//         unsafe {
//             match self.kind {
//                 MapVirtual => {
//                     if libc::VirtualFree(self.data as *mut c_void, 0,
//                                          libc::MEM_RELEASE) == 0 {
//                         println!("VirtualFree failed: {}", errno());
//                     }
//                 },
//                 MapFile(mapping) => {
//                     if libc::UnmapViewOfFile(self.data as LPCVOID) == FALSE {
//                         println!("UnmapViewOfFile failed: {}", errno());
//                     }
//                     if libc::CloseHandle(mapping as HANDLE) == FALSE {
//                         println!("CloseHandle failed: {}", errno());
//                     }
//                 }
//             }
//         }
//     }
// }
//
// impl MemoryMap {
//     /// Returns the pointer to the memory created or modified by this map.
//     pub fn data(&self) -> *mut u8 { self.data }
//     /// Returns the number of bytes this map applies to.
//     pub fn len(&self) -> uint { self.len }
//     /// Returns the type of mapping this represents.
//     pub fn kind(&self) -> MemoryMapKind { self.kind }
// }

