extern crate io2;

use io2::net::*;
use io2::net::Ipv6MulticastScope::*;

#[test]
fn test_from_str_ipv4() {
    assert_eq!(Ok(Ipv4Addr::new(127, 0, 0, 1)), "127.0.0.1".parse());
    assert_eq!(Ok(Ipv4Addr::new(255, 255, 255, 255)), "255.255.255.255".parse());
    assert_eq!(Ok(Ipv4Addr::new(0, 0, 0, 0)), "0.0.0.0".parse());

    // out of range
    let none: Option<IpAddr> = "256.0.0.1".parse().ok();
    assert_eq!(None, none);
    // too short
    let none: Option<IpAddr> = "255.0.0".parse().ok();
    assert_eq!(None, none);
    // too long
    let none: Option<IpAddr> = "255.0.0.1.2".parse().ok();
    assert_eq!(None, none);
    // no number between dots
    let none: Option<IpAddr> = "255.0..1".parse().ok();
    assert_eq!(None, none);
}

#[test]
fn test_from_str_ipv6() {
    assert_eq!(Ok(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0)), "0:0:0:0:0:0:0:0".parse());
    assert_eq!(Ok(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1)), "0:0:0:0:0:0:0:1".parse());

    assert_eq!(Ok(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1)), "::1".parse());
    assert_eq!(Ok(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0)), "::".parse());

    assert_eq!(Ok(Ipv6Addr::new(0x2a02, 0x6b8, 0, 0, 0, 0, 0x11, 0x11)),
            "2a02:6b8::11:11".parse());

    // too long group
    let none: Option<IpAddr> = "::00000".parse().ok();
    assert_eq!(None, none);
    // too short
    let none: Option<IpAddr> = "1:2:3:4:5:6:7".parse().ok();
    assert_eq!(None, none);
    // too long
    let none: Option<IpAddr> = "1:2:3:4:5:6:7:8:9".parse().ok();
    assert_eq!(None, none);
    // triple colon
    let none: Option<IpAddr> = "1:2:::6:7:8".parse().ok();
    assert_eq!(None, none);
    // two double colons
    let none: Option<IpAddr> = "1:2::6::8".parse().ok();
    assert_eq!(None, none);
}

#[test]
fn test_from_str_ipv4_in_ipv6() {
    assert_eq!(Ok(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 49152, 545)),
            "::192.0.2.33".parse());
    assert_eq!(Ok(Ipv6Addr::new(0, 0, 0, 0, 0, 0xFFFF, 49152, 545)),
            "::FFFF:192.0.2.33".parse());
    assert_eq!(Ok(Ipv6Addr::new(0x64, 0xff9b, 0, 0, 0, 0, 49152, 545)),
            "64:ff9b::192.0.2.33".parse());
    assert_eq!(Ok(Ipv6Addr::new(0x2001, 0xdb8, 0x122, 0xc000, 0x2, 0x2100, 49152, 545)),
            "2001:db8:122:c000:2:2100:192.0.2.33".parse());

    // colon after v4
    let none: Option<IpAddr> = "::127.0.0.1:".parse().ok();
    assert_eq!(None, none);
    // not enough groups
    let none: Option<IpAddr> = "1.2.3.4.5:127.0.0.1".parse().ok();
    assert_eq!(None, none);
    // too many groups
    let none: Option<IpAddr> = "1.2.3.4.5:6:7:127.0.0.1".parse().ok();
    assert_eq!(None, none);
}

#[test]
fn test_from_str_socket_addr() {
    assert_eq!(Ok(SocketAddr { ip: IpAddr::new_v4(77, 88, 21, 11), port: 80 }),
            "77.88.21.11:80".parse());
    assert_eq!(Ok(SocketAddr { ip: IpAddr::new_v6(0x2a02, 0x6b8, 0, 1, 0, 0, 0, 1), port: 53 }),
            "[2a02:6b8:0:1::1]:53".parse());
    assert_eq!(Ok(SocketAddr { ip: IpAddr::new_v6(0, 0, 0, 0, 0, 0, 0x7F00, 1), port: 22 }),
            "[::127.0.0.1]:22".parse());

    // without port
    let none: Option<SocketAddr> = "127.0.0.1".parse().ok();
    assert_eq!(None, none);
    // without port
    let none: Option<SocketAddr> = "127.0.0.1:".parse().ok();
    assert_eq!(None, none);
    // wrong brackets around v4
    let none: Option<SocketAddr> = "[127.0.0.1]:22".parse().ok();
    assert_eq!(None, none);
    // port out of range
    let none: Option<SocketAddr> = "127.0.0.1:123456".parse().ok();
    assert_eq!(None, none);
}

#[test]
fn ipv6_addr_to_string() {
    // ipv4-mapped address
    let a1 = Ipv6Addr::new(0, 0, 0, 0, 0, 0xffff, 0xc000, 0x280);
    assert_eq!(a1.to_string(), "::ffff:192.0.2.128");

    // ipv4-compatible address
    let a1 = Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0xc000, 0x280);
    assert_eq!(a1.to_string(), "::192.0.2.128");

    // v6 address with no zero segments
    assert_eq!(Ipv6Addr::new(8, 9, 10, 11, 12, 13, 14, 15).to_string(),
               "8:9:a:b:c:d:e:f");

    // reduce a single run of zeros
    assert_eq!("ae::ffff:102:304",
               Ipv6Addr::new(0xae, 0, 0, 0, 0, 0xffff, 0x0102, 0x0304).to_string());

    // don't reduce just a single zero segment
    assert_eq!("1:2:3:4:5:6:0:8",
               Ipv6Addr::new(1, 2, 3, 4, 5, 6, 0, 8).to_string());

    // 'any' address
    assert_eq!("::", Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0).to_string());

    // loopback address
    assert_eq!("::1", Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1).to_string());

    // ends in zeros
    assert_eq!("1::", Ipv6Addr::new(1, 0, 0, 0, 0, 0, 0, 0).to_string());

    // two runs of zeros, second one is longer
    assert_eq!("1:0:0:4::8", Ipv6Addr::new(1, 0, 0, 4, 0, 0, 0, 8).to_string());

    // two runs of zeros, equal length
    assert_eq!("1::4:5:0:0:8", Ipv6Addr::new(1, 0, 0, 4, 5, 0, 0, 8).to_string());
}

#[test]
fn ipv4_to_ipv6() {
    assert_eq!(Ipv6Addr::new(0, 0, 0, 0, 0, 0xffff, 0x1234, 0x5678),
               Ipv4Addr::new(0x12, 0x34, 0x56, 0x78).to_ipv6_mapped());
    assert_eq!(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0x1234, 0x5678),
               Ipv4Addr::new(0x12, 0x34, 0x56, 0x78).to_ipv6_compatible());
}

#[test]
fn ipv6_to_ipv4() {
    assert_eq!(Ipv6Addr::new(0, 0, 0, 0, 0, 0xffff, 0x1234, 0x5678).to_ipv4(),
               Some(Ipv4Addr::new(0x12, 0x34, 0x56, 0x78)));
    assert_eq!(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0x1234, 0x5678).to_ipv4(),
               Some(Ipv4Addr::new(0x12, 0x34, 0x56, 0x78)));
    assert_eq!(Ipv6Addr::new(0, 0, 1, 0, 0, 0, 0x1234, 0x5678).to_ipv4(),
               None);
}

#[test]
fn ipv4_properties() {
    fn check(octets: &[u8; 4], unspec: bool, loopback: bool,
             private: bool, link_local: bool, global: bool,
             multicast: bool) {
        println!("testing IPv4 address {:?}", octets);
        let ip = Ipv4Addr::new(octets[0], octets[1], octets[2], octets[3]);
        assert_eq!(octets, ip.octets());

        assert_eq!(ip.is_unspecified(), unspec);
        assert_eq!(ip.is_loopback(), loopback);
        assert_eq!(ip.is_private(), private);
        assert_eq!(ip.is_link_local(), link_local);
        assert_eq!(ip.is_global(), global);
        assert_eq!(ip.is_multicast(), multicast);
    }

    //    address                unspec loopbk privt  linloc global multicast
    check(&[0, 0, 0, 0],         true,  false, false, false, true,  false);
    check(&[0, 0, 0, 1],         false, false, false, false, true,  false);
    check(&[1, 0, 0, 0],         false, false, false, false, true,  false);
    check(&[10, 9, 8, 7],        false, false, true,  false, false, false);
    check(&[127, 1, 2, 3],       false, true,  false, false, false, false);
    check(&[172, 31, 254, 253],  false, false, true,  false, false,  false);
    check(&[169, 254, 253, 242], false, false, false, true,  false, false);
    check(&[192, 168, 254, 253], false, false, true,  false, false, false);
    check(&[224, 0, 0, 0],       false, false, false, false, true,  true);
    check(&[239, 255, 255, 255], false, false, false, false, true,  true);
    check(&[255, 255, 255, 255], false, false, false, false, true,  false);
}

#[test]
fn ipv6_properties() {
    fn check(str_addr: &str, unspec: bool, loopback: bool,
             unique_local: bool, global: bool,
             u_link_local: bool, u_site_local: bool, u_global: bool,
             m_scope: Option<Ipv6MulticastScope>) {
        println!("testing IPv6 address {:?}", str_addr);
        let ip: Ipv6Addr = str_addr.parse().ok().unwrap();
        assert_eq!(str_addr, ip.to_string());

        assert_eq!(ip.is_unspecified(), unspec);
        assert_eq!(ip.is_loopback(), loopback);
        assert_eq!(ip.is_unique_local(), unique_local);
        assert_eq!(ip.is_global(), global);
        assert_eq!(ip.is_unicast_link_local(), u_link_local);
        assert_eq!(ip.is_unicast_site_local(), u_site_local);
        assert_eq!(ip.is_unicast_global(), u_global);
        assert_eq!(ip.multicast_scope(), m_scope);
        assert_eq!(ip.is_multicast(), m_scope.is_some());
    }

    //    unspec loopbk uniqlo global unill  unisl  uniglo mscope
    check("::",
          true,  false, false, true,  false, false, true,  None);
    check("::1",
          false, true,  false, false, false, false, false, None);
    check("::0.0.0.2",
          false, false, false, true,  false, false, true,  None);
    check("1::",
          false, false, false, true,  false, false, true,  None);
    check("fc00::",
          false, false, true,  false, false, false, false, None);
    check("fdff:ffff::",
          false, false, true,  false, false, false, false, None);
    check("fe80:ffff::",
          false, false, false, false, true,  false, false, None);
    check("febf:ffff::",
          false, false, false, false, true,  false, false, None);
    check("fec0::",
          false, false, false, false, false, true,  false, None);
    check("ff01::",
          false, false, false, false, false, false, false, Some(InterfaceLocal));
    check("ff02::",
          false, false, false, false, false, false, false, Some(LinkLocal));
    check("ff03::",
          false, false, false, false, false, false, false, Some(RealmLocal));
    check("ff04::",
          false, false, false, false, false, false, false, Some(AdminLocal));
    check("ff05::",
          false, false, false, false, false, false, false, Some(SiteLocal));
    check("ff08::",
          false, false, false, false, false, false, false, Some(OrganizationLocal));
    check("ff0e::",
          false, false, false, true,  false, false, false, Some(Global));
}

#[test]
fn to_socket_addr_socketaddr() {
    let a = SocketAddr { ip: IpAddr::new_v4(77, 88, 21, 11), port: 12345 };
    assert_eq!(Ok(vec![a]), a.to_socket_addrs());
}

#[test]
fn to_socket_addr_ipaddr_u16() {
    let a = IpAddr::new_v4(77, 88, 21, 11);
    let p = 12345u16;
    let e = SocketAddr { ip: a, port: p };
    assert_eq!(Ok(vec![e]), (a, p).to_socket_addrs());
}

#[test]
fn to_socket_addr_str_u16() {
    let a = SocketAddr { ip: IpAddr::new_v4(77, 88, 21, 11), port: 24352 };
    assert_eq!(Ok(vec![a]), ("77.88.21.11", 24352u16).to_socket_addrs());

    let a = SocketAddr { ip: IpAddr::new_v6(0x2a02, 0x6b8, 0, 1, 0, 0, 0, 1), port: 53 };
    assert_eq!(Ok(vec![a]), ("2a02:6b8:0:1::1", 53).to_socket_addrs());

    let a = SocketAddr { ip: IpAddr::new_v4(127, 0, 0, 1), port: 23924 };
    assert!(("localhost", 23924u16).to_socket_addrs().unwrap().contains(&a));
}

#[test]
fn to_socket_addr_str() {
    let a = SocketAddr { ip: IpAddr::new_v4(77, 88, 21, 11), port: 24352 };
    assert_eq!(Ok(vec![a]), "77.88.21.11:24352".to_socket_addrs());

    let a = SocketAddr { ip: IpAddr::new_v6(0x2a02, 0x6b8, 0, 1, 0, 0, 0, 1), port: 53 };
    assert_eq!(Ok(vec![a]), "[2a02:6b8:0:1::1]:53".to_socket_addrs());

    let a = SocketAddr { ip: IpAddr::new_v4(127, 0, 0, 1), port: 23924 };
    assert!("localhost:23924".to_socket_addrs().unwrap().contains(&a));
}
