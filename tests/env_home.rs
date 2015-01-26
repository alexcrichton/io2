#![allow(unstable)]
extern crate io2;

use io2::env::*;

#[test]
#[cfg(unix)]
fn test_home_dir() {
    let oldhome = var("HOME");

    set_var("HOME", "/home/MountainView");
    assert!(home_dir() == Some(Path::new("/home/MountainView")));

    set_var("HOME", "");
    assert!(home_dir().is_none());

    for s in oldhome.iter() {
        set_var("HOME", s);
    }
}

#[test]
#[cfg(windows)]
fn test_home_dir() {
    let oldhome = var("HOME");
    let olduserprofile = var("USERPROFILE");

    set_var("HOME", "");
    set_var("USERPROFILE", "");

    assert!(home_dir().is_none());

    set_var("HOME", "/home/MountainView");
    assert!(home_dir() == Some(Path::new("/home/MountainView")));

    set_var("HOME", "");

    set_var("USERPROFILE", "/home/MountainView");
    assert!(home_dir() == Some(Path::new("/home/MountainView")));

    set_var("HOME", "/home/MountainView");
    set_var("USERPROFILE", "/home/PaloAlto");
    assert!(home_dir() == Some(Path::new("/home/MountainView")));

    for s in oldhome.iter() {
        set_var("HOME", s);
    }
    for s in olduserprofile.iter() {
        set_var("USERPROFILE", s);
    }
}
