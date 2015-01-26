#![allow(unstable)]
#![no_std]
extern crate io2;
#[macro_use] extern crate std;
extern crate core;

use core::prelude::*;

use io2::io::prelude::*;
use io2::io::{Cursor, SeekPos};
use std::vec::Vec;

#[test]
fn test_vec_writer() {
    let mut writer = Vec::new();
    assert_eq!(writer.write(&[0]), Ok(1));
    assert_eq!(writer.write(&[1, 2, 3]), Ok(3));
    assert_eq!(writer.write(&[4, 5, 6, 7]), Ok(4));
    let b: &[_] = &[0, 1, 2, 3, 4, 5, 6, 7];
    assert_eq!(writer, b);
}

#[test]
fn test_mem_writer() {
    let mut writer = Cursor::new(Vec::new());
    assert_eq!(writer.write(&[0]), Ok(1));
    assert_eq!(writer.write(&[1, 2, 3]), Ok(3));
    assert_eq!(writer.write(&[4, 5, 6, 7]), Ok(4));
    let b: &[_] = &[0, 1, 2, 3, 4, 5, 6, 7];
    assert_eq!(&writer.get_ref()[], b);
}

#[test]
fn test_buf_writer() {
    let mut buf = [0 as u8; 9];
    {
        let mut writer = Cursor::new(&mut buf[]);
        assert_eq!(writer.position(), 0);
        assert_eq!(writer.write(&[0]), Ok(1));
        assert_eq!(writer.position(), 1);
        assert_eq!(writer.write(&[1, 2, 3]), Ok(3));
        assert_eq!(writer.write(&[4, 5, 6, 7]), Ok(4));
        assert_eq!(writer.position(), 8);
        assert_eq!(writer.write(&[]), Ok(0));
        assert_eq!(writer.position(), 8);

        assert_eq!(writer.write(&[8, 9]), Ok(1));
        assert_eq!(writer.write(&[10]), Ok(0));
    }
    let b: &[_] = &[0, 1, 2, 3, 4, 5, 6, 7, 8];
    assert_eq!(buf, b);
}

#[test]
fn test_buf_writer_seek() {
    let mut buf = [0 as u8; 8];
    {
        let mut writer = Cursor::new(&mut buf[]);
        assert_eq!(writer.position(), 0);
        assert_eq!(writer.write(&[1]), Ok(1));
        assert_eq!(writer.position(), 1);

        assert_eq!(writer.seek(SeekPos::FromStart(2)), Ok(2));
        assert_eq!(writer.position(), 2);
        assert_eq!(writer.write(&[2]), Ok(1));
        assert_eq!(writer.position(), 3);

        assert_eq!(writer.seek(SeekPos::FromCur(-2)), Ok(1));
        assert_eq!(writer.position(), 1);
        assert_eq!(writer.write(&[3]), Ok(1));
        assert_eq!(writer.position(), 2);

        assert_eq!(writer.seek(SeekPos::FromEnd(-1)), Ok(7));
        assert_eq!(writer.position(), 7);
        assert_eq!(writer.write(&[4]), Ok(1));
        assert_eq!(writer.position(), 8);

    }
    let b: &[_] = &[1, 3, 2, 0, 0, 0, 0, 4];
    assert_eq!(buf, b);
}

#[test]
fn test_buf_writer_error() {
    let mut buf = [0 as u8; 2];
    let mut writer = Cursor::new(&mut buf[]);
    assert_eq!(writer.write(&[0]), Ok(1));
    assert_eq!(writer.write(&[0, 0]), Ok(1));
    assert_eq!(writer.write(&[0, 0]), Ok(0));
}

#[test]
fn test_mem_reader() {
    let mut reader = Cursor::new(vec!(0u8, 1, 2, 3, 4, 5, 6, 7));
    let mut buf = [];
    assert_eq!(reader.read(&mut buf), Ok(0));
    assert_eq!(reader.position(), 0);
    let mut buf = [0];
    assert_eq!(reader.read(&mut buf), Ok(1));
    assert_eq!(reader.position(), 1);
    let b: &[_] = &[0];
    assert_eq!(buf, b);
    let mut buf = [0; 4];
    assert_eq!(reader.read(&mut buf), Ok(4));
    assert_eq!(reader.position(), 5);
    let b: &[_] = &[1, 2, 3, 4];
    assert_eq!(buf, b);
    assert_eq!(reader.read(&mut buf), Ok(3));
    let b: &[_] = &[5, 6, 7];
    assert_eq!(&buf[..3], b);
    assert_eq!(reader.read(&mut buf), Ok(0));
}

#[test]
fn read_to_end() {
    let mut reader = Cursor::new(vec!(0u8, 1, 2, 3, 4, 5, 6, 7));
    let mut v = Vec::new();
    reader.read_to_end(&mut v).ok().unwrap();
    assert_eq!(v, [0, 1, 2, 3, 4, 5, 6, 7]);
}

#[test]
fn test_slice_reader() {
    let in_buf = vec![0u8, 1, 2, 3, 4, 5, 6, 7];
    let mut reader = &mut in_buf.as_slice();
    let mut buf = [];
    assert_eq!(reader.read(&mut buf), Ok(0));
    let mut buf = [0];
    assert_eq!(reader.read(&mut buf), Ok(1));
    assert_eq!(reader.len(), 7);
    let b: &[_] = &[0];
    assert_eq!(buf.as_slice(), b);
    let mut buf = [0; 4];
    assert_eq!(reader.read(&mut buf), Ok(4));
    assert_eq!(reader.len(), 3);
    let b: &[_] = &[1, 2, 3, 4];
    assert_eq!(buf.as_slice(), b);
    assert_eq!(reader.read(&mut buf), Ok(3));
    let b: &[_] = &[5, 6, 7];
    assert_eq!(&buf[..3], b);
    assert_eq!(reader.read(&mut buf), Ok(0));
}

#[test]
fn test_buf_reader() {
    let in_buf = vec![0u8, 1, 2, 3, 4, 5, 6, 7];
    let mut reader = Cursor::new(in_buf.as_slice());
    let mut buf = [];
    assert_eq!(reader.read(&mut buf), Ok(0));
    assert_eq!(reader.position(), 0);
    let mut buf = [0];
    assert_eq!(reader.read(&mut buf), Ok(1));
    assert_eq!(reader.position(), 1);
    let b: &[_] = &[0];
    assert_eq!(buf, b);
    let mut buf = [0; 4];
    assert_eq!(reader.read(&mut buf), Ok(4));
    assert_eq!(reader.position(), 5);
    let b: &[_] = &[1, 2, 3, 4];
    assert_eq!(buf, b);
    assert_eq!(reader.read(&mut buf), Ok(3));
    let b: &[_] = &[5, 6, 7];
    assert_eq!(&buf[..3], b);
    assert_eq!(reader.read(&mut buf), Ok(0));
}

#[test]
fn test_read_char() {
    let b = b"Vi\xE1\xBB\x87t";
    let mut c = Cursor::new(b).chars();
    assert_eq!(c.next(), Some(Ok('V')));
    assert_eq!(c.next(), Some(Ok('i')));
    assert_eq!(c.next(), Some(Ok('ệ')));
    assert_eq!(c.next(), Some(Ok('t')));
    assert_eq!(c.next(), None);
}

#[test]
fn test_read_bad_char() {
    let b = b"\x80";
    let mut c = Cursor::new(b).chars();
    assert!(c.next().unwrap().is_err());
}

#[test]
fn seek_past_end() {
    let buf = [0xff];
    let mut r = Cursor::new(&buf[]);
    assert_eq!(r.seek(SeekPos::FromStart(10)), Ok(10));
    assert_eq!(r.read(&mut [0]), Ok(0));

    let mut r = Cursor::new(vec!(10u8));
    assert_eq!(r.seek(SeekPos::FromStart(10)), Ok(10));
    assert_eq!(r.read(&mut [0]), Ok(0));

    let mut buf = [0];
    let mut r = Cursor::new(&mut buf[]);
    assert_eq!(r.seek(SeekPos::FromStart(10)), Ok(10));
    assert_eq!(r.write(&[3]), Ok(0));
}

#[test]
fn seek_before_0() {
    let buf = [0xff_u8];
    let mut r = Cursor::new(&buf[]);
    assert!(r.seek(SeekPos::FromEnd(-2)).is_err());

    let mut r = Cursor::new(vec!(10u8));
    assert!(r.seek(SeekPos::FromEnd(-2)).is_err());

    let mut buf = [0];
    let mut r = Cursor::new(&mut buf[]);
    assert!(r.seek(SeekPos::FromEnd(-2)).is_err());
}

#[test]
fn test_seekable_mem_writer() {
    let mut writer = Cursor::new(Vec::<u8>::new());
    assert_eq!(writer.position(), 0);
    assert_eq!(writer.write(&[0]), Ok(1));
    assert_eq!(writer.position(), 1);
    assert_eq!(writer.write(&[1, 2, 3]), Ok(3));
    assert_eq!(writer.write(&[4, 5, 6, 7]), Ok(4));
    assert_eq!(writer.position(), 8);
    let b: &[_] = &[0, 1, 2, 3, 4, 5, 6, 7];
    assert_eq!(&writer.get_ref()[], b);

    assert_eq!(writer.seek(SeekPos::FromStart(0)), Ok(0));
    assert_eq!(writer.position(), 0);
    assert_eq!(writer.write(&[3, 4]), Ok(2));
    let b: &[_] = &[3, 4, 2, 3, 4, 5, 6, 7];
    assert_eq!(&writer.get_ref()[], b);

    assert_eq!(writer.seek(SeekPos::FromCur(1)), Ok(3));
    assert_eq!(writer.write(&[0, 1]), Ok(2));
    let b: &[_] = &[3, 4, 2, 0, 1, 5, 6, 7];
    assert_eq!(&writer.get_ref()[], b);

    assert_eq!(writer.seek(SeekPos::FromEnd(-1)), Ok(7));
    assert_eq!(writer.write(&[1, 2]), Ok(2));
    let b: &[_] = &[3, 4, 2, 0, 1, 5, 6, 1, 2];
    assert_eq!(&writer.get_ref()[], b);

    assert_eq!(writer.seek(SeekPos::FromEnd(1)), Ok(10));
    assert_eq!(writer.write(&[1]), Ok(1));
    let b: &[_] = &[3, 4, 2, 0, 1, 5, 6, 1, 2, 0, 1];
    assert_eq!(&writer.get_ref()[], b);
}

#[test]
fn vec_seek_past_end() {
    let mut r = Cursor::new(Vec::new());
    assert_eq!(r.seek(SeekPos::FromStart(10)), Ok(10));
    assert_eq!(r.write(&[3]), Ok(1));
}

#[test]
fn vec_seek_before_0() {
    let mut r = Cursor::new(Vec::new());
    assert!(r.seek(SeekPos::FromEnd(-2)).is_err());
}
