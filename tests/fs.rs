#![feature(rand, path, core, os)]
#![allow(warnings)]

extern crate io2;

use io2::io::prelude::*;
use io2::fs::{self, File, OpenOptions};
use io2::io::{ErrorKind, SeekPos};
use std::os;
use std::rand::{self, StdRng, Rng};
use std::str;

macro_rules! check { ($e:expr) => (
    match $e {
        Ok(t) => t,
        Err(e) => panic!("{} failed with: {}", stringify!($e), e),
    }
) }

macro_rules! error { ($e:expr, $s:expr) => (
    match $e {
        Ok(_) => panic!("Unexpected success. Should've been: {:?}", $s),
        Err(ref err) => assert!(err.to_string().contains($s.as_slice()),
                                format!("`{}` did not contain `{}`", err, $s))
    }
) }

pub struct TempDir(Path);

impl TempDir {
    fn join(&self, path: &str) -> Path {
        let TempDir(ref p) = *self;
        p.join(path)
    }

    fn path<'a>(&'a self) -> &'a Path {
        let TempDir(ref p) = *self;
        p
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        // Gee, seeing how we're testing the fs module I sure hope that we
        // at least implement this correctly!
        let TempDir(ref p) = *self;
        check!(fs::remove_dir_all(p));
    }
}

pub fn tmpdir() -> TempDir {
    let ret = os::tmpdir().join(format!("rust-{}", rand::random::<u32>()));
    check!(fs::make_dir(&ret));
    TempDir(ret)
}

#[test]
fn file_test_io_smoke_test() {
    let message = "it's alright. have a good time";
    let tmpdir = tmpdir();
    let filename = &tmpdir.join("file_rt_io_file_test.txt");
    {
        let mut write_stream = check!(File::create(filename));
        check!(write_stream.write(message.as_bytes()));
    }
    {
        let mut read_stream = check!(File::open(filename));
        let mut read_buf = [0; 1028];
        let read_str = match check!(read_stream.read(&mut read_buf)) {
            -1|0 => panic!("shouldn't happen"),
            n => str::from_utf8(&read_buf[..n]).unwrap().to_string()
        };
        assert_eq!(read_str.as_slice(), message);
    }
    check!(fs::remove_file(filename));
}

#[test]
fn invalid_path_raises() {
    let tmpdir = tmpdir();
    let filename = &tmpdir.join("file_that_does_not_exist.txt");
    let result = File::open(filename);

    if cfg!(unix) {
        error!(result, "o such file or directory");
    }
    // error!(result, "couldn't open path as file");
    // error!(result, format!("path={}; mode=open; access=read", filename.display()));
}

#[test]
fn file_test_iounlinking_invalid_path_should_raise_condition() {
    let tmpdir = tmpdir();
    let filename = &tmpdir.join("file_another_file_that_does_not_exist.txt");

    let result = fs::remove_file(filename);

    if cfg!(unix) {
        error!(result, "o such file or directory");
    }
    // error!(result, "couldn't unlink path");
    // error!(result, format!("path={}", filename.display()));
}

#[test]
fn file_test_io_non_positional_read() {
    let message: &str = "ten-four";
    let mut read_mem = [0; 8];
    let tmpdir = tmpdir();
    let filename = &tmpdir.join("file_rt_io_file_test_positional.txt");
    {
        let mut rw_stream = check!(File::create(filename));
        check!(rw_stream.write(message.as_bytes()));
    }
    {
        let mut read_stream = check!(File::open(filename));
        {
            let read_buf = &mut read_mem[0..4];
            check!(read_stream.read(read_buf));
        }
        {
            let read_buf = &mut read_mem[4..8];
            check!(read_stream.read(read_buf));
        }
    }
    check!(fs::remove_file(filename));
    let read_str = str::from_utf8(&read_mem).unwrap();
    assert_eq!(read_str, message);
}

#[test]
fn file_test_io_seek_and_tell_smoke_test() {
    let message = "ten-four";
    let mut read_mem = [0; 4];
    let set_cursor = 4 as u64;
    let mut tell_pos_pre_read;
    let mut tell_pos_post_read;
    let tmpdir = tmpdir();
    let filename = &tmpdir.join("file_rt_io_file_test_seeking.txt");
    {
        let mut rw_stream = check!(File::create(filename));
        check!(rw_stream.write(message.as_bytes()));
    }
    {
        let mut read_stream = check!(File::open(filename));
        check!(read_stream.seek(SeekPos::FromStart(set_cursor)));
        tell_pos_pre_read = check!(read_stream.seek(SeekPos::FromCur(0)));
        check!(read_stream.read(&mut read_mem));
        tell_pos_post_read = check!(read_stream.seek(SeekPos::FromCur(0)));
    }
    check!(fs::remove_file(filename));
    let read_str = str::from_utf8(&read_mem).unwrap();
    assert_eq!(read_str, &message[4..8]);
    assert_eq!(tell_pos_pre_read, set_cursor);
    assert_eq!(tell_pos_post_read, message.len() as u64);
}

#[test]
fn file_test_io_seek_and_write() {
    let initial_msg =   "food-is-yummy";
    let overwrite_msg =    "-the-bar!!";
    let final_msg =     "foo-the-bar!!";
    let seek_idx = 3;
    let mut read_mem = [0; 13];
    let tmpdir = tmpdir();
    let filename = &tmpdir.join("file_rt_io_file_test_seek_and_write.txt");
    {
        let mut rw_stream = check!(File::create(filename));
        check!(rw_stream.write(initial_msg.as_bytes()));
        check!(rw_stream.seek(SeekPos::FromStart(seek_idx)));
        check!(rw_stream.write(overwrite_msg.as_bytes()));
    }
    {
        let mut read_stream = check!(File::open(filename));
        check!(read_stream.read(&mut read_mem));
    }
    check!(fs::remove_file(filename));
    let read_str = str::from_utf8(&read_mem).unwrap();
    assert!(read_str == final_msg);
}

#[test]
fn file_test_io_seek_shakedown() {
    //                   01234567890123
    let initial_msg =   "qwer-asdf-zxcv";
    let chunk_one: &str = "qwer";
    let chunk_two: &str = "asdf";
    let chunk_three: &str = "zxcv";
    let mut read_mem = [0; 4];
    let tmpdir = tmpdir();
    let filename = &tmpdir.join("file_rt_io_file_test_seek_shakedown.txt");
    {
        let mut rw_stream = check!(File::create(filename));
        check!(rw_stream.write(initial_msg.as_bytes()));
    }
    {
        let mut read_stream = check!(File::open(filename));

        check!(read_stream.seek(SeekPos::FromEnd(-4)));
        check!(read_stream.read(&mut read_mem));
        assert_eq!(str::from_utf8(&read_mem).unwrap(), chunk_three);

        check!(read_stream.seek(SeekPos::FromCur(-9)));
        check!(read_stream.read(&mut read_mem));
        assert_eq!(str::from_utf8(&read_mem).unwrap(), chunk_two);

        check!(read_stream.seek(SeekPos::FromStart(0)));
        check!(read_stream.read(&mut read_mem));
        assert_eq!(str::from_utf8(&read_mem).unwrap(), chunk_one);
    }
    check!(fs::remove_file(filename));
}

#[test]
fn file_test_stat_is_correct_on_is_file() {
    let tmpdir = tmpdir();
    let filename = &tmpdir.join("file_stat_correct_on_is_file.txt");
    {
        let mut opts = OpenOptions::new();
        let mut fs = check!(File::open_opts(filename,
                                            opts.read(true).write(true)
                                                .create(true)));
        let msg = "hw";
        fs.write(msg.as_bytes()).unwrap();

        let fstat_res = check!(fs.file_attr());
        assert!(fstat_res.is_file());
    }
    let stat_res_fn = check!(fs::file_attr(filename));
    assert!(stat_res_fn.is_file());
    let stat_res_meth = check!(filename.file_attr());
    assert!(stat_res_meth.is_file());
    check!(fs::remove_file(filename));
}

#[test]
fn file_test_stat_is_correct_on_is_dir() {
    let tmpdir = tmpdir();
    let filename = &tmpdir.join("file_stat_correct_on_is_dir");
    check!(fs::make_dir(filename));
    let stat_res_fn = check!(fs::file_attr(filename));
    assert!(stat_res_fn.is_dir());
    let stat_res_meth = check!(filename.file_attr());
    assert!(stat_res_meth.is_dir());
    check!(fs::remove_dir(filename));
}

#[test]
fn file_test_fileinfo_false_when_checking_is_file_on_a_directory() {
    let tmpdir = tmpdir();
    let dir = &tmpdir.join("fileinfo_false_on_dir");
    check!(fs::make_dir(dir));
    assert!(dir.is_file() == false);
    check!(fs::remove_dir(dir));
}

#[test]
fn file_test_fileinfo_check_exists_before_and_after_file_creation() {
    let tmpdir = tmpdir();
    let file = &tmpdir.join("fileinfo_check_exists_b_and_a.txt");
    check!(check!(File::create(file)).write(b"foo"));
    assert!(file.exists());
    check!(fs::remove_file(file));
    assert!(!file.exists());
}

#[test]
fn file_test_directoryinfo_check_exists_before_and_after_mkdir() {
    let tmpdir = tmpdir();
    let dir = &tmpdir.join("before_and_after_dir");
    assert!(!dir.exists());
    check!(fs::make_dir(dir));
    assert!(dir.exists());
    assert!(dir.is_dir());
    check!(fs::remove_dir(dir));
    assert!(!dir.exists());
}

#[test]
fn file_test_directoryinfo_readdir() {
    let tmpdir = tmpdir();
    let dir = &tmpdir.join("di_readdir");
    check!(fs::make_dir(dir));
    let prefix = "foo";
    for n in range(0, 3) {
        let f = dir.join(format!("{}.txt", n));
        let mut w = check!(File::create(&f));
        let msg_str = format!("{}{}", prefix, n.to_string());
        let msg = msg_str.as_bytes();
        check!(w.write(msg));
    }
    let mut files = check!(fs::read_dir(dir));
    let mut mem = [0u8; 4];
    for f in files {
        let f = f.unwrap().path();
        {
            let n = f.filestem_str();
            check!(check!(File::open(&f)).read(&mut mem));
            let read_str = str::from_utf8(&mem).unwrap();
            let expected = match n {
                None|Some("") => panic!("really shouldn't happen.."),
                Some(n) => format!("{}{}", prefix, n),
            };
            assert_eq!(expected.as_slice(), read_str);
        }
        check!(fs::remove_file(&f));
    }
    check!(fs::remove_dir(dir));
}

#[test]
fn file_test_walk_dir() {
    let tmpdir = tmpdir();
    let dir = &tmpdir.join("walk_dir");
    check!(fs::make_dir(dir));

    let dir1 = &dir.join("01/02/03");
    check!(fs::make_dir_all(dir1));
    check!(File::create(&dir1.join("04")));

    let dir2 = &dir.join("11/12/13");
    check!(fs::make_dir_all(dir2));
    check!(File::create(&dir2.join("14")));

    let mut files = check!(fs::walk_dir(dir));
    let mut cur = [0u8; 2];
    for f in files {
        let f = f.unwrap().path();
        let stem = f.filestem_str().unwrap();
        let root = stem.as_bytes()[0] - b'0';
        let name = stem.as_bytes()[1] - b'0';
        assert!(cur[root as usize] < name);
        cur[root as usize] = name;
    }

    check!(fs::remove_dir_all(dir));
}

#[test]
fn mkdir_path_already_exists_error() {
    let tmpdir = tmpdir();
    let dir = &tmpdir.join("mkdir_error_twice");
    check!(fs::make_dir(dir));
    let e = fs::make_dir(dir).err().unwrap();
    assert_eq!(e.kind(), ErrorKind::PathAlreadyExists);
}

#[test]
fn recursive_mkdir() {
    let tmpdir = tmpdir();
    let dir = tmpdir.join("d1/d2");
    check!(fs::make_dir_all(&dir));
    assert!(dir.is_dir())
}

#[test]
fn recursive_mkdir_failure() {
    let tmpdir = tmpdir();
    let dir = tmpdir.join("d1");
    let file = dir.join("f1");

    check!(fs::make_dir_all(&dir));
    check!(File::create(&file));

    let result = fs::make_dir_all(&file);

    assert!(result.is_err());
    // error!(result, "couldn't recursively mkdir");
    // error!(result, "couldn't create directory");
    // error!(result, "mode=0700");
    // error!(result, format!("path={}", file.display()));
}

#[test]
fn recursive_mkdir_slash() {
    check!(fs::make_dir_all(&Path::new("/")));
}

// FIXME(#12795) depends on lstat to work on windows
#[cfg(not(windows))]
#[test]
fn recursive_rmdir() {
    let tmpdir = tmpdir();
    let d1 = tmpdir.join("d1");
    let dt = d1.join("t");
    let dtt = dt.join("t");
    let d2 = tmpdir.join("d2");
    let canary = d2.join("do_not_delete");
    check!(fs::make_dir_all(&dtt));
    check!(fs::make_dir_all(&d2));
    check!(check!(File::create(&canary)).write(b"foo"));
    check!(fs::sym_link(&d2, &dt.join("d2")));
    check!(fs::remove_dir_all(&d1));

    assert!(!d1.is_dir());
    assert!(canary.exists());
}

#[test]
fn unicode_path_is_dir() {
    assert!(Path::new(".").is_dir());
    assert!(!Path::new("test/stdtest/fs.rs").is_dir());

    let tmpdir = tmpdir();

    let mut dirpath = tmpdir.path().clone();
    dirpath.push(format!("test-가一ー你好"));
    check!(fs::make_dir(&dirpath));
    assert!(dirpath.is_dir());

    let mut filepath = dirpath;
    filepath.push("unicode-file-\u{ac00}\u{4e00}\u{30fc}\u{4f60}\u{597d}.rs");
    check!(File::create(&filepath)); // ignore return; touch only
    assert!(!filepath.is_dir());
    assert!(filepath.exists());
}

#[test]
fn unicode_path_exists() {
    assert!(Path::new(".").exists());
    assert!(!Path::new("test/nonexistent-bogus-path").exists());

    let tmpdir = tmpdir();
    let unicode = tmpdir.path();
    let unicode = unicode.join(format!("test-각丁ー再见"));
    check!(fs::make_dir(&unicode));
    assert!(unicode.exists());
    assert!(!Path::new("test/unicode-bogus-path-각丁ー再见").exists());
}

#[test]
fn copy_file_does_not_exist() {
    let from = Path::new("test/nonexistent-bogus-path");
    let to = Path::new("test/other-bogus-path");

    match fs::copy(&from, &to) {
        Ok(..) => panic!(),
        Err(..) => {
            assert!(!from.exists());
            assert!(!to.exists());
        }
    }
}

#[test]
fn copy_file_ok() {
    let tmpdir = tmpdir();
    let input = tmpdir.join("in.txt");
    let out = tmpdir.join("out.txt");

    check!(check!(File::create(&input)).write(b"hello"));
    check!(fs::copy(&input, &out));
    let mut v = Vec::new();
    check!(check!(File::open(&out)).read_to_end(&mut v));
    assert_eq!(v.as_slice(), b"hello");

    assert_eq!(check!(input.file_attr()).perm(),
               check!(out.file_attr()).perm());
}

#[test]
fn copy_file_dst_dir() {
    let tmpdir = tmpdir();
    let out = tmpdir.join("out");

    check!(File::create(&out));
    match fs::copy(&out, tmpdir.path()) {
        Ok(..) => panic!(), Err(..) => {}
    }
}

#[test]
fn copy_file_dst_exists() {
    let tmpdir = tmpdir();
    let input = tmpdir.join("in");
    let output = tmpdir.join("out");

    check!(check!(File::create(&input)).write("foo".as_bytes()));
    check!(check!(File::create(&output)).write("bar".as_bytes()));
    check!(fs::copy(&input, &output));

    let mut v = Vec::new();
    check!(check!(File::open(&output)).read_to_end(&mut v));
    assert_eq!(v, b"foo".to_vec());
}

#[test]
fn copy_file_src_dir() {
    let tmpdir = tmpdir();
    let out = tmpdir.join("out");

    match fs::copy(tmpdir.path(), &out) {
        Ok(..) => panic!(), Err(..) => {}
    }
    assert!(!out.exists());
}

#[test]
fn copy_file_preserves_perm_bits() {
    let tmpdir = tmpdir();
    let input = tmpdir.join("in.txt");
    let out = tmpdir.join("out.txt");

    let attr = check!(check!(File::create(&input)).file_attr());
    let mut p = attr.perm();
    p.set_readonly(true);
    check!(fs::set_perm(&input, p));
    check!(fs::copy(&input, &out));
    assert!(check!(out.file_attr()).perm().readonly());
}

#[cfg(not(windows))] // FIXME(#10264) operation not permitted?
#[test]
fn symlinks_work() {
    let tmpdir = tmpdir();
    let input = tmpdir.join("in.txt");
    let out = tmpdir.join("out.txt");

    check!(check!(File::create(&input)).write("foobar".as_bytes()));
    check!(fs::sym_link(&input, &out));
    // if cfg!(not(windows)) {
    //     assert_eq!(check!(lstat(&out)).kind, FileType::Symlink);
    //     assert_eq!(check!(out.lstat()).kind, FileType::Symlink);
    // }
    assert_eq!(check!(fs::file_attr(&out)).size(),
               check!(fs::file_attr(&input)).size());
    let mut v = Vec::new();
    check!(check!(File::open(&out)).read_to_end(&mut v));
    assert_eq!(v, b"foobar".to_vec());
}

#[cfg(not(windows))] // apparently windows doesn't like symlinks
#[test]
fn symlink_noexist() {
    let tmpdir = tmpdir();
    // symlinks can point to things that don't exist
    check!(fs::sym_link(&tmpdir.join("foo"), &tmpdir.join("bar")));
    assert_eq!(check!(fs::read_link(&tmpdir.join("bar"))),
               tmpdir.join("foo"));
}

#[test]
fn readlink_not_symlink() {
    let tmpdir = tmpdir();
    match fs::read_link(tmpdir.path()) {
        Ok(..) => panic!("wanted a failure"),
        Err(..) => {}
    }
}

#[test]
fn links_work() {
    let tmpdir = tmpdir();
    let input = tmpdir.join("in.txt");
    let out = tmpdir.join("out.txt");

    check!(check!(File::create(&input)).write("foobar".as_bytes()));
    check!(fs::hard_link(&input, &out));
    assert_eq!(check!(fs::file_attr(&out)).size(),
               check!(fs::file_attr(&input)).size());
    assert_eq!(check!(fs::file_attr(&out)).size(),
               check!(input.file_attr()).size());
    let mut v = Vec::new();
    check!(check!(File::open(&out)).read_to_end(&mut v));
    assert_eq!(v, b"foobar".to_vec());

    // can't link to yourself
    match fs::hard_link(&input, &input) {
        Ok(..) => panic!("wanted a failure"),
        Err(..) => {}
    }
    // can't link to something that doesn't exist
    match fs::hard_link(&tmpdir.join("foo"), &tmpdir.join("bar")) {
        Ok(..) => panic!("wanted a failure"),
        Err(..) => {}
    }
}

#[test]
fn chmod_works() {
    let tmpdir = tmpdir();
    let file = tmpdir.join("in.txt");

    check!(File::create(&file));
    let attr = check!(fs::file_attr(&file));
    assert!(!attr.perm().readonly());
    let mut p = attr.perm();
    p.set_readonly(true);
    check!(fs::set_perm(&file, p.clone()));
    let attr = check!(fs::file_attr(&file));
    assert!(attr.perm().readonly());

    match fs::set_perm(&tmpdir.join("foo"), p) {
        Ok(..) => panic!("wanted a panic"),
        Err(..) => {}
    }
}

#[test]
fn sync_doesnt_kill_anything() {
    let tmpdir = tmpdir();
    let path = tmpdir.join("in.txt");

    let mut file = check!(File::create(&path));
    check!(file.flush());
    check!(file.flush_data());
    check!(file.write(b"foo"));
    check!(file.flush());
    check!(file.flush_data());
}

#[test]
fn truncate_works() {
    let tmpdir = tmpdir();
    let path = tmpdir.join("in.txt");

    let mut file = check!(File::create(&path));
    check!(file.write(b"foo"));
    check!(file.flush());

    // Do some simple things with truncation
    assert_eq!(check!(file.file_attr()).size(), 3);
    check!(file.truncate(10));
    assert_eq!(check!(file.file_attr()).size(), 10);
    check!(file.write(b"bar"));
    check!(file.flush());
    assert_eq!(check!(file.file_attr()).size(), 10);

    let mut v = Vec::new();
    check!(check!(File::open(&path)).read_to_end(&mut v));
    assert_eq!(v, b"foobar\0\0\0\0".to_vec());

    // Truncate to a smaller length, don't seek, and then write something.
    // Ensure that the intermediate zeroes are all filled in (we're seeked
    // past the end of the file).
    check!(file.truncate(2));
    assert_eq!(check!(file.file_attr()).size(), 2);
    check!(file.write(b"wut"));
    check!(file.flush());
    assert_eq!(check!(file.file_attr()).size(), 9);
    let mut v = Vec::new();
    check!(check!(File::open(&path)).read_to_end(&mut v));
    assert_eq!(v, b"fo\0\0\0\0wut".to_vec());
}

#[test]
fn open_flavors() {
    use io2::fs::OpenOptions as OO;
    fn c<T: Clone>(t: &T) -> T { t.clone() }

    let tmpdir = tmpdir();

    let mut r = OO::new(); r.read(true);
    let mut w = OO::new(); w.write(true);
    let mut rw = OO::new(); rw.write(true).read(true);

    match File::open_opts(&tmpdir.join("a"), &r) {
        Ok(..) => panic!(), Err(..) => {}
    }

    // Perform each one twice to make sure that it succeeds the second time
    // (where the file exists)
    check!(File::open_opts(&tmpdir.join("b"), c(&w).create(true)));
    assert!(tmpdir.join("b").exists());
    check!(File::open_opts(&tmpdir.join("b"), c(&w).create(true)));
    check!(File::open_opts(&tmpdir.join("b"), &w));

    check!(File::open_opts(&tmpdir.join("c"), c(&rw).create(true)));
    assert!(tmpdir.join("c").exists());
    check!(File::open_opts(&tmpdir.join("c"), c(&rw).create(true)));
    check!(File::open_opts(&tmpdir.join("c"), &rw));

    check!(File::open_opts(&tmpdir.join("d"), c(&w).append(true).create(true)));
    assert!(tmpdir.join("d").exists());
    check!(File::open_opts(&tmpdir.join("d"), c(&w).append(true).create(true)));
    check!(File::open_opts(&tmpdir.join("d"), c(&w).append(true)));

    check!(File::open_opts(&tmpdir.join("e"), c(&rw).append(true).create(true)));
    assert!(tmpdir.join("e").exists());
    check!(File::open_opts(&tmpdir.join("e"), c(&rw).append(true).create(true)));
    check!(File::open_opts(&tmpdir.join("e"), c(&rw).append(true)));

    check!(File::open_opts(&tmpdir.join("f"), c(&w).truncate(true).create(true)));
    assert!(tmpdir.join("f").exists());
    check!(File::open_opts(&tmpdir.join("f"), c(&w).truncate(true).create(true)));
    check!(File::open_opts(&tmpdir.join("f"), c(&w).truncate(true)));

    check!(File::open_opts(&tmpdir.join("g"), c(&rw).truncate(true).create(true)));
    assert!(tmpdir.join("g").exists());
    check!(File::open_opts(&tmpdir.join("g"), c(&rw).truncate(true).create(true)));
    check!(File::open_opts(&tmpdir.join("g"), c(&rw).truncate(true)));

    check!(check!(File::create(&tmpdir.join("h"))).write("foo".as_bytes()));
    check!(File::open_opts(&tmpdir.join("h"), &r));
    {
        let mut f = check!(File::open_opts(&tmpdir.join("h"), &r));
        assert!(f.write("wut".as_bytes()).is_err());
    }
    assert_eq!(check!(fs::file_attr(&tmpdir.join("h"))).size(), 3);
    {
        let mut f = check!(File::open_opts(&tmpdir.join("h"), c(&w).append(true)));
        check!(f.write("bar".as_bytes()));
    }
    assert_eq!(check!(fs::file_attr(&tmpdir.join("h"))).size(), 6);
    {
        let mut f = check!(File::open_opts(&tmpdir.join("h"),
                                           c(&w).truncate(true)));
        check!(f.write("bar".as_bytes()));
    }
    assert_eq!(check!(fs::file_attr(&tmpdir.join("h"))).size(), 3);
}

#[test]
fn utime() {
    let tmpdir = tmpdir();
    let path = tmpdir.join("a");
    check!(File::create(&path));
    // These numbers have to be bigger than the time in the day to account
    // for timezones Windows in particular will fail in certain timezones
    // with small enough values
    check!(fs::change_file_times(&path, 100000, 200000));
    assert_eq!(check!(path.file_attr()).accessed(), 100000);
    assert_eq!(check!(path.file_attr()).modified(), 200000);
}

#[test]
fn utime_noexist() {
    let tmpdir = tmpdir();

    match fs::change_file_times(&tmpdir.join("a"), 100, 200) {
        Ok(..) => panic!(),
        Err(..) => {}
    }
}

#[test]
fn binary_file() {
    let mut bytes = [0; 1024];
    StdRng::new().ok().unwrap().fill_bytes(&mut bytes);

    let tmpdir = tmpdir();

    check!(check!(File::create(&tmpdir.join("test"))).write(&bytes));
    let mut v = Vec::new();
    check!(check!(File::open(&tmpdir.join("test"))).read_to_end(&mut v));
    assert!(v == bytes.as_slice());
}

#[test]
fn unlink_readonly() {
    let tmpdir = tmpdir();
    let path = tmpdir.join("file");
    check!(File::create(&path));
    let mut perm = check!(fs::file_attr(&path)).perm();
    perm.set_readonly(true);
    check!(fs::set_perm(&path, perm));
    check!(fs::remove_file(&path));
}
