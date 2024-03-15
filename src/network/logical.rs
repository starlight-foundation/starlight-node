use std::{array::IntoIter, fmt::Display, str::FromStr};

use serde::{Deserialize, Serialize};

use crate::{error, node::Error};

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct Logical {
    addr: [u8; 4],
    port: u16,
}

impl Logical {
    pub fn to_bytes(&self) -> [u8; 6] {
        let mut bytes = [0u8; 6];
        bytes[0..4].copy_from_slice(&self.addr);
        bytes[4..6].copy_from_slice(&self.port.to_le_bytes());
        bytes
    }
}

impl Display for Logical {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", std::net::Ipv4Addr::from(self.addr), self.port)
    }
}

impl std::net::ToSocketAddrs for Logical {
    type Iter = IntoIter<std::net::SocketAddr, 1>;
    fn to_socket_addrs(&self) -> std::io::Result<Self::Iter> {
        Ok([self.to_socket_addr()].into_iter())
    }
}

impl Logical {
    pub fn to_socket_addr(self) -> std::net::SocketAddr {
        std::net::SocketAddr::V4(std::net::SocketAddrV4::new(
            std::net::Ipv4Addr::from(self.addr),
            self.port,
        ))
    }
}

impl From<std::net::SocketAddrV4> for Logical {
    fn from(s: std::net::SocketAddrV4) -> Self {
        Self {
            addr: s.ip().octets(),
            port: s.port(),
        }
    }
}

impl FromStr for Logical {
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
