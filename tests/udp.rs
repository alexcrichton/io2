#![feature(core, std_misc)]

extern crate io2;

use io2::io::ErrorKind;
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

fn each_ip(f: &mut FnMut(SocketAddr, SocketAddr)) {
    f(next_test_ip4(), next_test_ip4());
    println!("ipv6");
    f(next_test_ip6(), next_test_ip6());
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
    let addr = SocketAddr { ip: IpAddr::new_v4(0, 0, 0, 0), port: 1 };
    match UdpSocket::bind(&addr) {
        Ok(..) => panic!(),
        Err(e) => assert_eq!(e.kind(), ErrorKind::PermissionDenied),
    }
}

#[test]
fn socket_smoke_test_ip4() {
    each_ip(&mut |server_ip, client_ip| {
        let (tx1, rx1) = channel();
        let (tx2, rx2) = channel();

        let _t = Thread::spawn(move|| {
            let client = t!(UdpSocket::bind(&client_ip));
            rx1.recv().unwrap();
            t!(client.send_to(&[99], &server_ip));
            tx2.send(()).unwrap();
        });

        let server = t!(UdpSocket::bind(&server_ip));
        tx1.send(()).unwrap();
        let mut buf = [0];
        let (nread, src) = t!(server.recv_from(&mut buf));
        assert_eq!(nread, 1);
        assert_eq!(buf[0], 99);
        assert_eq!(src, client_ip);
        rx2.recv().unwrap();
    })
}

pub fn socket_name(addr: SocketAddr) {
    let server = t!(UdpSocket::bind(&addr));
    assert_eq!(addr, t!(server.socket_addr()));
}

#[test]
fn socket_name_ip4() {
    each_ip(&mut |addr, _| {
        socket_name(addr)
    })
}

#[test]
fn udp_clone_smoke() {
    each_ip(&mut |addr1, addr2| {
        let sock1 = t!(UdpSocket::bind(&addr1));
        let sock2 = t!(UdpSocket::bind(&addr2));

        let _t = Thread::spawn(move|| {
            let mut buf = [0, 0];
            assert_eq!(sock2.recv_from(&mut buf), Ok((1, addr1)));
            assert_eq!(buf[0], 1);
            t!(sock2.send_to(&[2], &addr1));
        });

        let sock3 = t!(sock1.duplicate());

        let (tx1, rx1) = channel();
        let (tx2, rx2) = channel();
        let _t = Thread::spawn(move|| {
            rx1.recv().unwrap();
            t!(sock3.send_to(&[1], &addr2));
            tx2.send(()).unwrap();
        });
        tx1.send(()).unwrap();
        let mut buf = [0, 0];
        assert_eq!(sock1.recv_from(&mut buf), Ok((1, addr2)));
        rx2.recv().unwrap();
    })
}

#[test]
fn udp_clone_two_read() {
    each_ip(&mut |addr1, addr2| {
        let sock1 = t!(UdpSocket::bind(&addr1));
        let sock2 = t!(UdpSocket::bind(&addr2));
        let (tx1, rx) = channel();
        let tx2 = tx1.clone();

        let _t = Thread::spawn(move|| {
            t!(sock2.send_to(&[1], &addr1));
            rx.recv().unwrap();
            t!(sock2.send_to(&[2], &addr1));
            rx.recv().unwrap();
        });

        let sock3 = t!(sock1.duplicate());

        let (done, rx) = channel();
        let _t = Thread::spawn(move|| {
            let mut buf = [0, 0];
            t!(sock3.recv_from(&mut buf));
            tx2.send(()).unwrap();
            done.send(()).unwrap();
        });
        let mut buf = [0, 0];
        t!(sock1.recv_from(&mut buf));
        tx1.send(()).unwrap();

        rx.recv().unwrap();
    })
}

#[test]
fn udp_clone_two_write() {
    each_ip(&mut |addr1, addr2| {
        let sock1 = t!(UdpSocket::bind(&addr1));
        let sock2 = t!(UdpSocket::bind(&addr2));

        let (tx, rx) = channel();
        let (serv_tx, serv_rx) = channel();

        let _t = Thread::spawn(move|| {
            let mut buf = [0, 1];
            rx.recv().unwrap();
            t!(sock2.recv_from(&mut buf));
            serv_tx.send(()).unwrap();
        });

        let sock3 = t!(sock1.duplicate());

        let (done, rx) = channel();
        let tx2 = tx.clone();
        let _t = Thread::spawn(move|| {
            match sock3.send_to(&[1], &addr2) {
                Ok(..) => { let _ = tx2.send(()); }
                Err(..) => {}
            }
            done.send(()).unwrap();
        });
        match sock1.send_to(&[2], &addr2) {
            Ok(..) => { let _ = tx.send(()); }
            Err(..) => {}
        }
        drop(tx);

        rx.recv().unwrap();
        serv_rx.recv().unwrap();
    })
}
