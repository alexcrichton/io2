#![feature(std_misc, core)]

extern crate io2;

use io2::io::ErrorKind;
use io2::io::prelude::*;
use io2::net::*;

use std::sync::atomic::{AtomicUsize, ATOMIC_USIZE_INIT, Ordering};
use std::sync::mpsc::channel;
use std::thread::Thread;

fn next_test_ip4() -> SocketAddr {
    static PORT: AtomicUsize = ATOMIC_USIZE_INIT;
    SocketAddr {
        ip: IpAddr::new_v4(127, 0, 0, 1),
        port: PORT.fetch_add(1, Ordering::SeqCst) as u16 + 10000,
    }
}

fn next_test_ip6() -> SocketAddr {
    static PORT: AtomicUsize = ATOMIC_USIZE_INIT;
    SocketAddr {
        ip: IpAddr::new_v6(0, 0, 0, 0, 0, 0, 0, 1),
        port: PORT.fetch_add(1, Ordering::SeqCst) as u16 + 10000,
    }
}

fn each_ip(f: &mut FnMut(SocketAddr)) {
    f(next_test_ip4());
    println!("ipv6");
    f(next_test_ip6());
}

macro_rules! t {
    ($e:expr) => {
        match $e {
            Ok(t) => t,
            Err(e) => panic!("received error for `{}`: {}", stringify!($e), e),
        }
    }
}

// FIXME #11530 this fails on android because tests are run as root
#[cfg_attr(any(windows, target_os = "android"), ignore)]
#[test]
fn bind_error() {
    match TcpListener::bind("0.0.0.0:1") {
        Ok(..) => panic!(),
        Err(e) => assert_eq!(e.kind(), ErrorKind::PermissionDenied),
    }
}

#[test]
fn connect_error() {
    match TcpStream::connect("0.0.0.0:1") {
        Ok(..) => panic!(),
        Err(e) => assert_eq!(e.kind(), ErrorKind::ConnectionRefused),
    }
}

#[test]
fn listen_localhost() {
    let socket_addr = next_test_ip4();
    let listener = t!(TcpListener::bind(&socket_addr));

    let _t = Thread::scoped(move || {
        let mut stream = t!(TcpStream::connect(&("localhost",
                                                 socket_addr.port)));
        t!(stream.write(&[144]));
    });

    let mut stream = t!(listener.accept()).0;
    let mut buf = [0];
    t!(stream.read(&mut buf));
    assert!(buf[0] == 144);
}

#[test]
fn connect_ip4_loopback() {
    let addr = next_test_ip4();
    let acceptor = t!(TcpListener::bind(&addr));

    let _t = Thread::scoped(move|| {
        let mut stream = t!(TcpStream::connect(&("127.0.0.1", addr.port)));
        t!(stream.write(&[44]));
    });

    let mut stream = t!(acceptor.accept()).0;
    let mut buf = [0];
    t!(stream.read(&mut buf));
    assert!(buf[0] == 44);
}

#[test]
fn connect_ip6_loopback() {
    let addr = next_test_ip6();
    let acceptor = t!(TcpListener::bind(&addr));

    let _t = Thread::scoped(move|| {
        let mut stream = t!(TcpStream::connect(&("::1", addr.port)));
        t!(stream.write(&[66]));
    });

    let mut stream = t!(acceptor.accept()).0;
    let mut buf = [0];
    t!(stream.read(&mut buf));
    assert!(buf[0] == 66);
}

#[test]
fn smoke_test_ip6() {
    each_ip(&mut |addr| {
        let acceptor = t!(TcpListener::bind(&addr));

        let (tx, rx) = channel();
        let _t = Thread::scoped(move|| {
            let mut stream = t!(TcpStream::connect(&addr));
            t!(stream.write(&[99]));
            tx.send(t!(stream.socket_addr())).unwrap();
        });

        let (mut stream, addr) = t!(acceptor.accept());
        let mut buf = [0];
        t!(stream.read(&mut buf));
        assert!(buf[0] == 99);
        assert_eq!(addr, t!(rx.recv()));
    })
}

#[test]
fn read_eof_ip4() {
    each_ip(&mut |addr| {
        let acceptor = t!(TcpListener::bind(&addr));

        let _t = Thread::scoped(move|| {
            let _stream = t!(TcpStream::connect(&addr));
            // Close
        });

        let mut stream = t!(acceptor.accept()).0;
        let mut buf = [0];
        let nread = t!(stream.read(&mut buf));
        assert_eq!(nread, 0);
        let nread = t!(stream.read(&mut buf));
        assert_eq!(nread, 0);
    })
}

#[test]
fn write_close() {
    each_ip(&mut |addr| {
        let acceptor = t!(TcpListener::bind(&addr));

        let (tx, rx) = channel();
        let _t = Thread::scoped(move|| {
            drop(t!(TcpStream::connect(&addr)));
            tx.send(()).unwrap();
        });

        let mut stream = t!(acceptor.accept()).0;
        rx.recv().unwrap();
        let buf = [0];
        match stream.write(&buf) {
            Ok(..) => {}
            Err(e) => {
                assert!(e.kind() == ErrorKind::ConnectionReset ||
                        e.kind() == ErrorKind::BrokenPipe ||
                        e.kind() == ErrorKind::ConnectionAborted,
                        "unknown error: {}", e);
            }
        }
    })
}

#[test]
fn multiple_connect_serial_ip4() {
    each_ip(&mut |addr| {
        let max = 10;
        let acceptor = t!(TcpListener::bind(&addr));

        let _t = Thread::scoped(move|| {
            for _ in 0..max {
                let mut stream = t!(TcpStream::connect(&addr));
                t!(stream.write(&[99]));
            }
        });

        for stream in acceptor.incoming().take(max) {
            let mut stream = t!(stream);
            let mut buf = [0];
            t!(stream.read(&mut buf));
            assert_eq!(buf[0], 99);
        }
    })
}

#[test]
fn multiple_connect_interleaved_greedy_schedule() {
    static MAX: usize = 10;
    each_ip(&mut |addr| {
        let acceptor = t!(TcpListener::bind(&addr));

        let _t = Thread::scoped(move|| {
            let acceptor = acceptor;
            for (i, stream) in acceptor.incoming().enumerate().take(MAX) {
                // Start another task to handle the connection
                let _t = Thread::scoped(move|| {
                    let mut stream = t!(stream);
                    let mut buf = [0];
                    t!(stream.read(&mut buf));
                    assert!(buf[0] == i as u8);
                });
            }
        });

        connect(0, addr);
    });

    fn connect(i: usize, addr: SocketAddr) {
        if i == MAX { return }

        let t = Thread::scoped(move|| {
            let mut stream = t!(TcpStream::connect(&addr));
            // Connect again before writing
            connect(i + 1, addr);
            t!(stream.write(&[i as u8]));
        });
        t.join().ok().unwrap();
    }
}

#[test]
fn multiple_connect_interleaved_lazy_schedule_ip4() {
    static MAX: usize = 10;
    each_ip(&mut |addr| {
        let acceptor = t!(TcpListener::bind(&addr));

        let _t = Thread::scoped(move|| {
            for stream in acceptor.incoming().take(MAX) {
                // Start another task to handle the connection
                let _t = Thread::scoped(move|| {
                    let mut stream = t!(stream);
                    let mut buf = [0];
                    t!(stream.read(&mut buf));
                    assert!(buf[0] == 99);
                });
            }
        });

        connect(0, addr);
    });

    fn connect(i: usize, addr: SocketAddr) {
        if i == MAX { return }

        let t = Thread::scoped(move|| {
            let mut stream = t!(TcpStream::connect(&addr));
            connect(i + 1, addr);
            t!(stream.write(&[99]));
        });
        t.join().ok().unwrap();
    }
}

pub fn socket_name(addr: SocketAddr) {
    let listener = t!(TcpListener::bind(&addr));
    let so_name = t!(listener.socket_addr());
    assert_eq!(addr, so_name);
}

pub fn peer_name(addr: SocketAddr) {
    let acceptor = t!(TcpListener::bind(&addr));
    let _t = Thread::scoped(move|| {
        t!(acceptor.accept());
    });

    let stream = t!(TcpStream::connect(&addr));
    assert_eq!(addr, t!(stream.peer_addr()));
}

#[test]
fn socket_and_peer_name_ip4() {
    each_ip(&mut |addr| {
        peer_name(addr);
        socket_name(addr);
    })
}

#[test]
fn partial_read() {
    each_ip(&mut |addr| {
        let (tx, rx) = channel();
        let srv = t!(TcpListener::bind(&addr));
        let _t = Thread::scoped(move|| {
            let mut cl = t!(srv.accept()).0;
            cl.write(&[10]).unwrap();
            let mut b = [0];
            t!(cl.read(&mut b));
            tx.send(()).unwrap();
        });

        let mut c = t!(TcpStream::connect(&addr));
        let mut b = [0; 10];
        assert_eq!(c.read(&mut b), Ok(1));
        t!(c.write(&[1]));
        rx.recv().unwrap();
    })
}

#[test]
fn double_bind() {
    each_ip(&mut |addr| {
        let _listener = t!(TcpListener::bind(&addr));
        match TcpListener::bind(&addr) {
            Ok(..) => panic!(),
            Err(e) => {
                assert!(e.kind() == ErrorKind::ConnectionRefused ||
                        e.kind() == ErrorKind::Other,
                        "unknown error: {} {:?}", e, e.kind());
            }
        }
    })
}

#[test]
fn fast_rebind() {
    each_ip(&mut |addr| {
        let acceptor = t!(TcpListener::bind(&addr));

        let _t = Thread::scoped(move|| {
            t!(TcpStream::connect(&addr));
        });

        t!(acceptor.accept());
        drop(acceptor);
        t!(TcpListener::bind(&addr));
    });
}

#[test]
fn tcp_clone_smoke() {
    each_ip(&mut |addr| {
        let acceptor = t!(TcpListener::bind(&addr));

        let _t = Thread::scoped(move|| {
            let mut s = t!(TcpStream::connect(&addr));
            let mut buf = [0, 0];
            assert_eq!(s.read(&mut buf), Ok(1));
            assert_eq!(buf[0], 1);
            t!(s.write(&[2]));
        });

        let mut s1 = t!(acceptor.accept()).0;
        let s2 = t!(s1.duplicate());

        let (tx1, rx1) = channel();
        let (tx2, rx2) = channel();
        let _t = Thread::scoped(move|| {
            let mut s2 = s2;
            rx1.recv().unwrap();
            t!(s2.write(&[1]));
            tx2.send(()).unwrap();
        });
        tx1.send(()).unwrap();
        let mut buf = [0, 0];
        assert_eq!(s1.read(&mut buf), Ok(1));
        rx2.recv().unwrap();
    })
}

#[test]
fn tcp_clone_two_read() {
    each_ip(&mut |addr| {
        let acceptor = t!(TcpListener::bind(&addr));
        let (tx1, rx) = channel();
        let tx2 = tx1.clone();

        let _t = Thread::scoped(move|| {
            let mut s = t!(TcpStream::connect(&addr));
            t!(s.write(&[1]));
            rx.recv().unwrap();
            t!(s.write(&[2]));
            rx.recv().unwrap();
        });

        let mut s1 = t!(acceptor.accept()).0;
        let s2 = t!(s1.duplicate());

        let (done, rx) = channel();
        let _t = Thread::scoped(move|| {
            let mut s2 = s2;
            let mut buf = [0, 0];
            t!(s2.read(&mut buf));
            tx2.send(()).unwrap();
            done.send(()).unwrap();
        });
        let mut buf = [0, 0];
        t!(s1.read(&mut buf));
        tx1.send(()).unwrap();

        rx.recv().unwrap();
    })
}

#[test]
fn tcp_clone_two_write() {
    each_ip(&mut |addr| {
        let acceptor = t!(TcpListener::bind(&addr));

        let _t = Thread::scoped(move|| {
            let mut s = t!(TcpStream::connect(&addr));
            let mut buf = [0, 1];
            t!(s.read(&mut buf));
            t!(s.read(&mut buf));
        });

        let mut s1 = t!(acceptor.accept()).0;
        let s2 = t!(s1.duplicate());

        let (done, rx) = channel();
        let _t = Thread::scoped(move|| {
            let mut s2 = s2;
            t!(s2.write(&[1]));
            done.send(()).unwrap();
        });
        t!(s1.write(&[2]));

        rx.recv().unwrap();
    })
}

#[test]
fn shutdown_smoke() {
    each_ip(&mut |addr| {
        let a = t!(TcpListener::bind(&addr));
        let _t = Thread::scoped(move|| {
            let mut c = t!(a.accept()).0;
            let mut b = [0];
            assert_eq!(c.read(&mut b), Ok(0));
            t!(c.write(&[1]));
        });

        let mut s = t!(TcpStream::connect(&addr));
        t!(s.shutdown(Shutdown::Write));
        assert!(s.write(&[1]).is_err());
        let mut b = [0, 0];
        assert_eq!(t!(s.read(&mut b)), 1);
        assert_eq!(b[0], 1);
    })
}

#[test]
fn close_readwrite_smoke() {
    each_ip(&mut |addr| {
        let a = t!(TcpListener::bind(&addr));
        let (tx, rx) = channel::<()>();
        let _t = Thread::scoped(move|| {
            let _s = t!(a.accept());
            let _ = rx.recv();
        });

        let mut b = [0];
        let mut s = t!(TcpStream::connect(&addr));
        let mut s2 = t!(s.duplicate());

        // closing should prevent reads/writes
        t!(s.shutdown(Shutdown::Write));
        assert!(s.write(&[0]).is_err());
        t!(s.shutdown(Shutdown::Read));
        assert_eq!(s.read(&mut b), Ok(0));

        // closing should affect previous handles
        assert!(s2.write(&[0]).is_err());
        assert_eq!(s2.read(&mut b), Ok(0));

        // closing should affect new handles
        let mut s3 = t!(s.duplicate());
        assert!(s3.write(&[0]).is_err());
        assert_eq!(s3.read(&mut b), Ok(0));

        // make sure these don't die
        let _ = s2.shutdown(Shutdown::Read);
        let _ = s2.shutdown(Shutdown::Write);
        let _ = s3.shutdown(Shutdown::Read);
        let _ = s3.shutdown(Shutdown::Write);
        drop(tx);
    })
}

#[test]
fn close_read_wakes_up() {
    each_ip(&mut |addr| {
        let a = t!(TcpListener::bind(&addr));
        let (tx1, rx) = channel::<()>();
        let _t = Thread::scoped(move|| {
            let _s = t!(a.accept());
            let _ = rx.recv();
        });

        let s = t!(TcpStream::connect(&addr));
        let s2 = t!(s.duplicate());
        let (tx, rx) = channel();
        let _t = Thread::scoped(move|| {
            let mut s2 = s2;
            assert_eq!(t!(s2.read(&mut [0])), 0);
            tx.send(()).unwrap();
        });
        // this should wake up the child task
        t!(s.shutdown(Shutdown::Read));

        // this test will never finish if the child doesn't wake up
        rx.recv().unwrap();
        drop(tx1);
    })
}

#[test]
fn clone_while_reading() {
    each_ip(&mut |addr| {
        let accept = t!(TcpListener::bind(&addr));

        // Enqueue a task to write to a socket
        let (tx, rx) = channel();
        let (txdone, rxdone) = channel();
        let txdone2 = txdone.clone();
        let _t = Thread::scoped(move|| {
            let mut tcp = t!(TcpStream::connect(&addr));
            rx.recv().unwrap();
            t!(tcp.write(&[0]));
            txdone2.send(()).unwrap();
        });

        // Spawn off a reading clone
        let tcp = t!(accept.accept()).0;
        let tcp2 = t!(tcp.duplicate());
        let txdone3 = txdone.clone();
        let _t = Thread::scoped(move|| {
            let mut tcp2 = tcp2;
            t!(tcp2.read(&mut [0]));
            txdone3.send(()).unwrap();
        });

        // Try to ensure that the reading clone is indeed reading
        for _ in 0..50 {
            Thread::yield_now();
        }

        // clone the handle again while it's reading, then let it finish the
        // read.
        let _ = t!(tcp.duplicate());
        tx.send(()).unwrap();
        rxdone.recv().unwrap();
        rxdone.recv().unwrap();
    })
}

#[test]
fn clone_accept_smoke() {
    each_ip(&mut |addr| {
        let a = t!(TcpListener::bind(&addr));
        let a2 = t!(a.duplicate());

        let _t = Thread::scoped(move|| {
            let _ = TcpStream::connect(&addr);
        });
        let _t = Thread::scoped(move|| {
            let _ = TcpStream::connect(&addr);
        });

        t!(a.accept());
        t!(a2.accept());
    })
}

#[test]
fn clone_accept_concurrent() {
    each_ip(&mut |addr| {
        let a = t!(TcpListener::bind(&addr));
        let a2 = t!(a.duplicate());

        let (tx, rx) = channel();
        let tx2 = tx.clone();

        let _t = Thread::scoped(move|| {
            tx.send(t!(a.accept())).unwrap();
        });
        let _t = Thread::scoped(move|| {
            tx2.send(t!(a2.accept())).unwrap();
        });

        let _t = Thread::scoped(move|| {
            let _ = TcpStream::connect(&addr);
        });
        let _t = Thread::scoped(move|| {
            let _ = TcpStream::connect(&addr);
        });

        rx.recv().unwrap();
        rx.recv().unwrap();
    })
}
