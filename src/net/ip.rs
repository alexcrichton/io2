// Copyright 2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use prelude::v1::*;

use fmt;

pub trait ToIpAddr {
    /// Convert the address to a generic IpAddr
    fn to_ip_addr(&self) -> IpAddr;
}

#[derive(Copy, PartialEq, Eq, Clone, Hash, Debug)]
pub struct Ipv4Addr {
    octets: [u8; 4]
}

#[derive(Copy, PartialEq, Eq, Clone, Hash, Debug)]
pub struct Ipv6Addr {
    segments: [u16; 8]
}

#[derive(Copy, PartialEq, Eq, Clone, Hash, Debug)]
pub enum Ipv6MulticastScope {
    InterfaceLocal,
    LinkLocal,
    RealmLocal,
    AdminLocal,
    SiteLocal,
    OrganizationLocal,
    Global
}

#[derive(Copy, PartialEq, Eq, Clone, Hash, Debug)]
pub enum IpAddr {
    V4(Ipv4Addr),
    V6(Ipv6Addr)
}

impl IpAddr {
    /// Create a new IpAddr that contains an IPv4 address.
    ///
    /// The result will represent the IP address a.b.c.d
    pub fn new_v4(a: u8, b: u8, c: u8, d: u8) -> IpAddr {
        Ipv4Addr::new(a, b, c, d).to_ip_addr()
    }

    /// Create a new IpAddr that contains an IPv6 address.
    ///
    /// The result will represent the IP address a:b:c:d:e:f
    pub fn new_v6(a: u16, b: u16, c: u16, d: u16, e: u16, f: u16, g: u16,
                  h: u16) -> IpAddr {
        Ipv6Addr::new(a, b, c, d, e, f, g, h).to_ip_addr()
    }
}

impl ToIpAddr for IpAddr {
    fn to_ip_addr(&self) -> IpAddr {
        *self
    }
}

#[stable(feature = "rust1", since = "1.0.0")]
impl fmt::Display for IpAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            IpAddr::V4(v4) => v4.fmt(f),
            IpAddr::V6(v6) => v6.fmt(f)
        }
    }
}

impl Ipv4Addr {
    /// Create a new IPv4 address from four eight-bit octets.
    ///
    /// The result will represent the IP address a.b.c.d
    pub fn new(a: u8, b: u8, c: u8, d: u8) -> Ipv4Addr {
        Ipv4Addr {
            octets: [a, b, c, d]
        }
    }

    /// Returns the four eight-bit integers that make up this address
    pub fn octets(&self) -> &[u8; 4] {
        &self.octets
    }

    /// Returns true for the special 'unspecified' address 0.0.0.0
    pub fn is_unspecified(&self) -> bool {
        self.octets == [0, 0, 0, 0]
    }

    /// Returns true if this is a loopback address (127.0.0.0/8)
    pub fn is_loopback(&self) -> bool {
        self.octets[0] == 127
    }

    /// Returns true if this is a private address.
    ///
    /// The private address ranges are defined in RFC1918 and include:
    ///
    ///  - 10.0.0.0/8
    ///  - 172.16.0.0/12
    ///  - 192.168.0.0/16
    pub fn is_private(&self) -> bool {
        match (self.octets[0], self.octets[1]) {
            (10, _) => true,
            (172, b) if b >= 16 && b <= 31 => true,
            (192, 168) => true,
            _ => false
        }
    }

    /// Returns true if the address is link-local (169.254.0.0/16)
    pub fn is_link_local(&self) -> bool {
        self.octets[0] == 169 && self.octets[1] == 254
    }

    /// Returns true if the address appears to be globally routable.
    ///
    /// Non-globally-routable networks include the private networks (10.0.0.0/8,
    /// 172.16.0.0/12 and 192.168.0.0/16), the loopback network (127.0.0.0/8),
    /// and the link-local network (169.254.0.0/16).
    pub fn is_global(&self) -> bool {
        !self.is_private() && !self.is_loopback() && !self.is_link_local()
    }

    /// Returns true if this is a multicast address.
    ///
    /// Multicast addresses have a most significant octet between 224 and 239.
    pub fn is_multicast(&self) -> bool {
        self.octets[0] >= 224 && self.octets[0] <= 239
    }

    /// Convert this address to an IPv4-compatible IPv6 address
    ///
    /// a.b.c.d becomes ::a.b.c.d
    pub fn to_ipv6_compatible(&self) -> Ipv6Addr {
        Ipv6Addr::new(0, 0, 0, 0, 0, 0,
                      ((self.octets[0] as u16) << 8) | self.octets[1] as u16,
                      ((self.octets[2] as u16) << 8) | self.octets[3] as u16)
    }

    /// Convert this address to an IPv4-mapped IPv6 address
    ///
    /// a.b.c.d becomes ::ffff:a.b.c.d
    pub fn to_ipv6_mapped(&self) -> Ipv6Addr {
        Ipv6Addr::new(0, 0, 0, 0, 0, 0xffff,
                      ((self.octets[0] as u16) << 8) | self.octets[1] as u16,
                      ((self.octets[2] as u16) << 8) | self.octets[3] as u16)
    }

}

impl ToIpAddr for Ipv4Addr {
    fn to_ip_addr(&self) -> IpAddr {
        IpAddr::V4(*self)
    }
}

impl fmt::Display for Ipv4Addr {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{}.{}.{}.{}", self.octets[0], self.octets[1],
               self.octets[2], self.octets[3])
    }
}

impl Ipv6Addr {
    /// Create a new IPv6 address from eight 16-bit segments.
    ///
    /// The result will represent the IP address a:b:c:d:e:f
    pub fn new(a: u16, b: u16, c: u16, d: u16, e: u16, f: u16, g: u16,
               h: u16) -> Ipv6Addr {
        Ipv6Addr {
            segments: [a, b, c, d, e, f, g, h]
        }
    }

    /// Return the eight 16-bit segments that make up this address
    pub fn segments(&self) -> &[u16; 8] {
        &self.segments
    }

    /// Returns true for the special 'unspecified' address ::
    pub fn is_unspecified(&self) -> bool {
        self.segments == [0, 0, 0, 0, 0, 0, 0, 0]
    }

    /// Returns true if this is a loopback address (::1)
    pub fn is_loopback(&self) -> bool {
        self.segments == [0, 0, 0, 0, 0, 0, 0, 1]
    }

    /// Returns true if the address appears to be globally routable.
    ///
    /// Non-globally-routable networks include the loopback address; the
    /// link-local, site-local, and unique local unicast addresses; and the
    /// interface-, link-, realm-, admin- and site-local multicast addresses.
    pub fn is_global(&self) -> bool {
        match self.multicast_scope() {
            Some(Ipv6MulticastScope::Global) => true,
            None => self.is_unicast_global(),
            _ => false
        }
    }

    /// Returns true if this is a unique local address (IPv6)
    ///
    /// Unique local addresses are defined in RFC4193 and have the form fc00::/7
    pub fn is_unique_local(&self) -> bool {
        (self.segments[0] & 0xfe00) == 0xfc00
    }

    /// Returns true if the address is unicast and link-local (fe80::/10)
    pub fn is_unicast_link_local(&self) -> bool {
        (self.segments[0] & 0xffc0) == 0xfe80
    }

    /// Returns true if this is a deprecated unicast site-local address (IPv6
    /// fec0::/10)
    pub fn is_unicast_site_local(&self) -> bool {
        (self.segments[0] & 0xffc0) == 0xfec0
    }

    /// Returns true if the address is a globally routable unicast address
    ///
    /// Non-globally-routable unicast addresses include the loopback address,
    /// the link-local addresses, the deprecated site-local addresses and the
    /// unique local addresses.
    pub fn is_unicast_global(&self) -> bool {
        !self.is_multicast()
            && !self.is_loopback() && !self.is_unicast_link_local()
            && !self.is_unicast_site_local() && !self.is_unique_local()
    }

    /// Returns the address's multicast scope if the address is multicast.
    pub fn multicast_scope(&self) -> Option<Ipv6MulticastScope> {
        if self.is_multicast() {
            match self.segments[0] & 0x000f {
                1 => Some(Ipv6MulticastScope::InterfaceLocal),
                2 => Some(Ipv6MulticastScope::LinkLocal),
                3 => Some(Ipv6MulticastScope::RealmLocal),
                4 => Some(Ipv6MulticastScope::AdminLocal),
                5 => Some(Ipv6MulticastScope::SiteLocal),
                8 => Some(Ipv6MulticastScope::OrganizationLocal),
                14 => Some(Ipv6MulticastScope::Global),
                _ => None
            }
        } else {
            None
        }
    }

    /// Returns true if this is a multicast address.
    ///
    /// Multicast addresses have the form ff00::/8.
    pub fn is_multicast(&self) -> bool {
        (self.segments[0] & 0xff00) == 0xff00
    }

    /// Convert this address to an IPv4 address. Returns None if this address is
    /// neither IPv4-compatible or IPv4-mapped.
    ///
    /// ::a.b.c.d and ::ffff:a.b.c.d become a.b.c.d
    pub fn to_ipv4(&self) -> Option<Ipv4Addr> {
        match self.segments {
            [0, 0, 0, 0, 0, f, g, h] if f == 0 || f == 0xffff => {
                Some(Ipv4Addr::new((g >> 8) as u8, g as u8,
                                   (h >> 8) as u8, h as u8))
            },
            _ => None
        }
    }
}

impl ToIpAddr for Ipv6Addr {
    fn to_ip_addr(&self) -> IpAddr {
        IpAddr::V6(*self)
    }
}

impl fmt::Display for Ipv6Addr {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self.segments {
            // We need special cases for :: and ::1, otherwise they're formatted
            // as ::0.0.0.[01]
            [0, 0, 0, 0, 0, 0, 0, 0] => write!(fmt, "::"),
            [0, 0, 0, 0, 0, 0, 0, 1] => write!(fmt, "::1"),
            // Ipv4 Compatible address
            [0, 0, 0, 0, 0, 0, g, h] => {
                write!(fmt, "::{}.{}.{}.{}", (g >> 8) as u8, g as u8,
                       (h >> 8) as u8, h as u8)
            }
            // Ipv4-Mapped address
            [0, 0, 0, 0, 0, 0xffff, g, h] => {
                write!(fmt, "::ffff:{}.{}.{}.{}", (g >> 8) as u8, g as u8,
                       (h >> 8) as u8, h as u8)
            },
            _ => {
                fn find_zero_slice(segments: &[u16; 8]) -> (usize, usize) {
                    let mut longest_span_len = 0;
                    let mut longest_span_at = 0;
                    let mut cur_span_len = 0;
                    let mut cur_span_at = 0;

                    for i in range(0, 8) {
                        if segments[i] == 0 {
                            if cur_span_len == 0 {
                                cur_span_at = i;
                            }

                            cur_span_len += 1;

                            if cur_span_len > longest_span_len {
                                longest_span_len = cur_span_len;
                                longest_span_at = cur_span_at;
                            }
                        } else {
                            cur_span_len = 0;
                            cur_span_at = 0;
                        }
                    }

                    (longest_span_at, longest_span_len)
                }

                let (zeros_at, zeros_len) = find_zero_slice(&self.segments);

                if zeros_len > 1 {
                    fn fmt_subslice(segments: &[u16]) -> String {
                        segments
                            .iter()
                            .map(|&seg| format!("{:x}", seg))
                            .collect::<Vec<String>>()
                            .as_slice()
                            .connect(":")
                    }

                    write!(fmt, "{}::{}",
                           fmt_subslice(&self.segments[..zeros_at]),
                           fmt_subslice(&self.segments[zeros_at + zeros_len..]))
                } else {
                    let &[a, b, c, d, e, f, g, h] = &self.segments;
                    write!(fmt, "{:x}:{:x}:{:x}:{:x}:{:x}:{:x}:{:x}:{:x}",
                           a, b, c, d, e, f, g, h)
                }
            }
        }
    }
}
