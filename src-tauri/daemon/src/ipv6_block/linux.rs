use std::{io, net::Ipv6Addr};

use netlink_packet_core::{
    NetlinkMessage, NetlinkPayload, NLM_F_ACK, NLM_F_CREATE, NLM_F_REPLACE, NLM_F_REQUEST,
};
use netlink_packet_route::{
    route::{
        RouteAddress, RouteAttribute, RouteHeader, RouteMessage, RouteProtocol, RouteScope,
        RouteType,
    },
    AddressFamily, RouteNetlinkMessage,
};
use netlink_sys::{constants::NETLINK_ROUTE, Socket, SocketAddr};
use tracing::error;

const SOCKET_BUFFER_LENGTH: usize = 12_288;

pub(crate) struct Ipv6Block {
    fwmark: u32,
}

impl Ipv6Block {
    pub(crate) fn new(fwmark: u32) -> Result<Self, io::Error> {
        netlink_request(
            RouteNetlinkMessage::NewRoute(route_message(fwmark)),
            NLM_F_REQUEST | NLM_F_ACK | NLM_F_CREATE | NLM_F_REPLACE,
        )?;
        Ok(Self { fwmark })
    }

    pub(crate) fn fwmark(&self) -> u32 {
        self.fwmark
    }

    /// Replaces the unreachable route with the tunnel's own `::/0` route in the same table.
    ///
    /// When an interface is configured again, wireguard-rs skips adding its `::/0` route because
    /// this block already occupies the slot, so deleting the block would leave the table without
    /// an IPv6 default and let IPv6 fall through to the main table. On failure the block is kept.
    pub(crate) fn into_tunnel_route(self, ifname: &str) -> Result<(), (Self, io::Error)> {
        let index = match nix::net::if_::if_nametoindex(ifname) {
            Ok(index) => index,
            Err(err) => return Err((self, err.into())),
        };
        let mut message = route_message(self.fwmark);
        message.header.scope = RouteScope::Link;
        message.header.kind = RouteType::Unicast;
        message.attributes.push(RouteAttribute::Oif(index));
        if let Err(err) = netlink_request(
            RouteNetlinkMessage::NewRoute(message),
            NLM_F_REQUEST | NLM_F_ACK | NLM_F_CREATE | NLM_F_REPLACE,
        ) {
            return Err((self, err));
        }
        // The route now belongs to the tunnel, so it must survive this block.
        std::mem::forget(self);
        Ok(())
    }
}

impl Drop for Ipv6Block {
    fn drop(&mut self) {
        if let Err(err) = netlink_request(
            RouteNetlinkMessage::DelRoute(route_message(self.fwmark)),
            NLM_F_REQUEST | NLM_F_ACK,
        ) {
            if err.kind() != io::ErrorKind::NotFound {
                error!(
                    "Failed to remove IPv6 unreachable route from table {}: {err}",
                    self.fwmark
                );
            }
        }
    }
}

fn route_message(fwmark: u32) -> RouteMessage {
    let mut message = RouteMessage::default();
    message.header = RouteHeader {
        address_family: AddressFamily::Inet6,
        destination_prefix_length: 0,
        table: RouteHeader::RT_TABLE_UNSPEC,
        protocol: RouteProtocol::Boot,
        scope: RouteScope::Universe,
        kind: RouteType::Unreachable,
        ..Default::default()
    };
    message.attributes.extend([
        RouteAttribute::Destination(RouteAddress::Inet6(Ipv6Addr::UNSPECIFIED)),
        RouteAttribute::Table(fwmark),
    ]);
    message
}

fn netlink_request(message: RouteNetlinkMessage, flags: u16) -> Result<(), io::Error> {
    let mut request = NetlinkMessage::from(message);
    request.header.flags = flags;
    request.finalize();

    let length = request.buffer_len();
    let mut buffer = vec![0; length];
    request.serialize(&mut buffer);

    let socket = Socket::new(NETLINK_ROUTE)?;
    socket.connect(&SocketAddr::new(0, 0))?;
    if socket.send(&buffer, 0)? != length {
        return Err(io::Error::from(io::ErrorKind::WriteZero));
    }

    loop {
        let mut receive_buffer = [0; SOCKET_BUFFER_LENGTH];
        let received = socket.recv(&mut &mut receive_buffer[..], 0)?;
        if received == 0 {
            return Err(io::Error::from(io::ErrorKind::UnexpectedEof));
        }

        let mut offset = 0;
        while offset < received {
            let response = NetlinkMessage::<RouteNetlinkMessage>::deserialize(
                &receive_buffer[offset..received],
            )
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
            let message_length = response.header.length as usize;
            if message_length == 0 || message_length > received - offset {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "invalid netlink response length",
                ));
            }

            match response.payload {
                NetlinkPayload::Error(message) if message.code.is_none() => return Ok(()),
                NetlinkPayload::Error(message) => return Err(message.to_io()),
                NetlinkPayload::Done(_) => return Ok(()),
                NetlinkPayload::Overrun(_) => return Err(io::Error::other("netlink overrun")),
                NetlinkPayload::Noop | NetlinkPayload::InnerMessage(_) => {}
                _ => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "unexpected netlink response",
                    ));
                }
            }
            offset += message_length;
        }
    }
}
