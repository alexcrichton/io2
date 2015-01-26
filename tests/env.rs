#![allow(unstable)]
extern crate io2;

use io2::env::*;
use std::iter::repeat;
use std::rand::{self, Rng};
use std::ffi::{OsString, OsStr};

fn make_rand_name() -> OsString {
    let mut rng = rand::thread_rng();
    let n = format!("TEST{}", rng.gen_ascii_chars().take(10)
                                 .collect::<String>());
    let n = OsString::from_string(n);
    assert!(var(&n).is_none());
    n
}

fn eq(a: Option<OsString>, b: Option<&str>) {
    assert_eq!(a.as_ref().map(|s| &**s), b.map(OsStr::from_str).map(|s| &*s));
}

#[test]
fn test_set_var() {
    let n = make_rand_name();
    set_var(&n, "VALUE");
    eq(var(&n), Some("VALUE"));
}

#[test]
fn test_remove_var() {
    let n = make_rand_name();
    set_var(&n, "VALUE");
    remove_var(&n);
    eq(var(&n), None);
}

#[test]
fn test_set_var_overwrite() {
    let n = make_rand_name();
    set_var(&n, "1");
    set_var(&n, "2");
    eq(var(&n), Some("2"));
    set_var(&n, "");
    eq(var(&n), Some(""));
}

#[test]
fn test_var_big() {
    let mut s = "".to_string();
    let mut i = 0;
    while i < 100 {
        s.push_str("aaaaaaaaaa");
        i += 1;
    }
    let n = make_rand_name();
    set_var(&n, s.as_slice());
    eq(var(&n), Some(s.as_slice()));
}

#[test]
fn test_self_exe_path() {
    let path = current_exe();
    assert!(path.is_some());
    let path = path.unwrap();

    // Hard to test this function
    assert!(path.is_absolute());
}

#[test]
fn test_env_var() {
    for (k, v) in vars() {
        let v2 = var(&k);
        // MingW seems to set some funky environment variables like
        // "=C:=C:\MinGW\msys\1.0\bin" and "!::=::\" that are returned
        // from vars() but not visible from var().
        assert!(v2.is_none() || v2.as_ref().map(|s| &**s) == Some(&*v));
    }
}

#[test]
fn test_env_set_get_huge() {
    let n = make_rand_name();
    let s = repeat("x").take(10000).collect::<String>();
    set_var(&n, &s);
    eq(var(&n), Some(s.as_slice()));
    remove_var(&n);
    eq(var(&n), None);
}

#[test]
fn test_env_set_var() {
    let n = make_rand_name();

    let mut e = vars();
    set_var(&n, "VALUE");
    assert!(!e.any(|(k, v)| {
        &*k == &*n && &*v == "VALUE"
    }));

    assert!(vars().any(|(k, v)| {
        &*k == &*n && &*v == "VALUE"
    }));
}

#[test]
fn test() {
    assert!((!Path::new("test-path").is_absolute()));

    current_dir().unwrap();
}

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

#[test]
#[cfg(windows)]
fn split_paths_windows() {
    fn check_parse(unparsed: &str, parsed: &[&str]) -> bool {
        split_paths(unparsed).collect::<Vec<_>>() ==
            parsed.iter().map(|s| Path::new(*s)).collect::<Vec<_>>()
    }

    assert!(check_parse("", &mut [""]));
    assert!(check_parse(r#""""#, &mut [""]));
    assert!(check_parse(";;", &mut ["", "", ""]));
    assert!(check_parse(r"c:\", &mut [r"c:\"]));
    assert!(check_parse(r"c:\;", &mut [r"c:\", ""]));
    assert!(check_parse(r"c:\;c:\Program Files\",
                        &mut [r"c:\", r"c:\Program Files\"]));
    assert!(check_parse(r#"c:\;c:\"foo"\"#, &mut [r"c:\", r"c:\foo\"]));
    assert!(check_parse(r#"c:\;c:\"foo;bar"\;c:\baz"#,
                        &mut [r"c:\", r"c:\foo;bar\", r"c:\baz"]));
}

#[test]
#[cfg(unix)]
fn split_paths_unix() {
    fn check_parse(unparsed: &str, parsed: &[&str]) -> bool {
        split_paths(unparsed).collect::<Vec<_>>() ==
            parsed.iter().map(|s| Path::new(*s)).collect::<Vec<_>>()
    }

    assert!(check_parse("", &mut [""]));
    assert!(check_parse("::", &mut ["", "", ""]));
    assert!(check_parse("/", &mut ["/"]));
    assert!(check_parse("/:", &mut ["/", ""]));
    assert!(check_parse("/:/usr/local", &mut ["/", "/usr/local"]));
}

#[test]
#[cfg(unix)]
fn join_paths_unix() {
    fn test_eq(input: &[&str], output: &str) -> bool {
        &*join_paths(input.iter().map(|s| *s)).unwrap() ==
            OsStr::from_str(output)
    }

    assert!(test_eq(&[], ""));
    assert!(test_eq(&["/bin", "/usr/bin", "/usr/local/bin"],
                     "/bin:/usr/bin:/usr/local/bin"));
    assert!(test_eq(&["", "/bin", "", "", "/usr/bin", ""],
                     ":/bin:::/usr/bin:"));
    assert!(join_paths(["/te:st"].iter().map(|s| *s)).is_err());
}

#[test]
#[cfg(windows)]
fn join_paths_windows() {
    fn test_eq(input: &[&str], output: &str) -> bool {
        &*join_paths(input.iter().map(|s| *s)).unwrap() ==
            OsStr::from_str(output)
    }

    assert!(test_eq(&[], ""));
    assert!(test_eq(&[r"c:\windows", r"c:\"],
                    r"c:\windows;c:\"));
    assert!(test_eq(&["", r"c:\windows", "", "", r"c:\", ""],
                    r";c:\windows;;;c:\;"));
    assert!(test_eq(&[r"c:\te;st", r"c:\"],
                    r#""c:\te;st";c:\"#));
    assert!(join_paths([r#"c:\te"st"#].iter().map(|s| *s)).is_err());
}
