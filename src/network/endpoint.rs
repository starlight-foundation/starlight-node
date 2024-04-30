use std::{array::IntoIter, fmt::Display, net::Ipv4Addr, str::{Chars, FromStr}};

use bincode::{Decode, Encode};
use nanoserde::{DeJson, DeJsonErr, DeJsonState, SerJson, SerJsonState};

use crate::{error, util::Error};

/// An ipv4 endpoint
#[derive(Clone, Copy, Debug, Encode, Decode)]
pub struct Endpoint {
    pub addr: [u8; 4],
    pub port: u16,
}

impl Endpoint {
    /// Checks if the IP address of the endpoint is an external, Internet-accessible IP.
    pub fn is_external(&self) -> bool {
        let addr = Ipv4Addr::from(self.addr);
        !addr.is_private()
            && !addr.is_loopback()
            && !addr.is_multicast()
            && !addr.is_unspecified()
            && !addr.is_link_local()
            && !addr.is_documentation()
            && !addr.is_broadcast()
    }
    pub fn to_bytes(&self) -> [u8; 6] {
        let mut bytes = [0u8; 6];
        bytes[0..4].copy_from_slice(&self.addr);
        bytes[4..6].copy_from_slice(&self.port.to_le_bytes());
        bytes
    }
}

impl Display for Endpoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", std::net::Ipv4Addr::from(self.addr), self.port)
    }
}

impl std::net::ToSocketAddrs for Endpoint {
    type Iter = IntoIter<std::net::SocketAddr, 1>;
    fn to_socket_addrs(&self) -> std::io::Result<Self::Iter> {
        Ok([self.to_socket_addr()].into_iter())
    }
}

impl Endpoint {
    pub fn to_socket_addr(self) -> std::net::SocketAddr {
        std::net::SocketAddr::V4(std::net::SocketAddrV4::new(
            std::net::Ipv4Addr::from(self.addr),
            self.port,
        ))
    }
}

impl From<std::net::SocketAddrV4> for Endpoint {
    fn from(s: std::net::SocketAddrV4) -> Self {
        Self {
            addr: s.ip().octets(),
            port: s.port(),
        }
    }
}

impl FromStr for Endpoint {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.split('/').last().unwrap();
        let mut parts = s.split(':');
        let addr = parts.next().ok_or(error!("no ip"))?;
        let port = parts.next().ok_or(error!("no port"))?.parse()?;
        Ok(Self {
            addr: addr.parse::<std::net::Ipv4Addr>()?.octets(),
            port,
        })
    }
}

impl SerJson for Endpoint {
    fn ser_json(&self, d: usize, s: &mut SerJsonState) {
        self.to_string().ser_json(d, s)
    }
}

impl DeJson for Endpoint {
    fn de_json(state: &mut DeJsonState, input: &mut Chars) -> Result<Self, DeJsonErr> {
        let s = String::de_json(state, input)?;
        Self::from_str(&s).map_err(|e| DeJsonErr {
            msg: e.to_string(),
            line: state.line,
            col: state.col
        })
    }
}
