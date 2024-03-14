use std::net::{Ipv4Addr, SocketAddrV4};

#[derive(Clone, Copy)]
pub struct Logical {
    addr: [u8; 4],
    port: u16
}

impl Logical {
    pub fn from_socket_addr_v4(s: SocketAddrV4) -> Self {
        Self {
            addr: s.ip().octets(),
            port: s.port()
        }
    }
    pub fn to_socket_addr_v4(self) -> SocketAddrV4 {
        SocketAddrV4::new(Ipv4Addr::from(self.addr), self.port)
    }
}

