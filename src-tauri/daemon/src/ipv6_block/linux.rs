use std::{io, net::Ipv6Addr};

use netlink_packet_core::{
    NetlinkMessage, NetlinkPayload, NLM_F_ACK, NLM_F_CREATE, NLM_F_DUMP, NLM_F_DUMP_INTR,
    NLM_F_REPLACE, NLM_F_REQUEST,
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
// Linux assigns this metric to the reject route when no metric is supplied.
const DEFAULT_IPV6_ROUTE_METRIC: u32 = 1024;
// IPv6 route deletion ignores the route type but matches the protocol, so a protocol of our own
// keeps Drop from deleting the tunnel's `::/0` route. 77 is not assigned by the kernel or iproute2.
const ROUTE_PROTOCOL: RouteProtocol = RouteProtocol::Other(77);

/// Owns the IPv6 reject route in the interface's fwmark table.
/// Drop attempts to remove it; the kernel does not remove it when the interface or daemon exits.
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
        if let Err(err) = replace_tunnel_route(self.fwmark, ifname, false) {
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
            let already_removed = err.kind() == io::ErrorKind::NotFound
                || err.raw_os_error() == Some(nix::errno::Errno::ESRCH as i32);
            if !already_removed {
                error!(
                    "Failed to remove IPv6 unreachable route from table {}: {err}",
                    self.fwmark
                );
            }
        }
    }
}

/// Restores the tunnel's IPv6 default after a daemon restart, but only if the fwmark table's
/// sole IPv6 default is Defguard's reject route.
pub(crate) fn recover_stale_route(fwmark: u32, ifname: &str) -> Result<(), io::Error> {
    let mut request = RouteMessage::default();
    request.header.address_family = AddressFamily::Inet6;
    let routes = netlink_routes(
        RouteNetlinkMessage::GetRoute(request),
        NLM_F_REQUEST | NLM_F_DUMP,
    )?;
    let defaults: Vec<_> = routes
        .iter()
        .filter(|route| {
            let table = route
                .attributes
                .iter()
                .find_map(|attribute| match attribute {
                    RouteAttribute::Table(table) => Some(*table),
                    _ => None,
                })
                .unwrap_or(u32::from(route.header.table));
            route.header.address_family == AddressFamily::Inet6
                && route.header.destination_prefix_length == 0
                && route.header.source_prefix_length == 0
                && table == fwmark
        })
        .collect();
    if let [route] = defaults.as_slice() {
        let metric = route
            .attributes
            .iter()
            .find_map(|attribute| match attribute {
                RouteAttribute::Priority(metric) => Some(*metric),
                _ => None,
            });
        if route.header.protocol == ROUTE_PROTOCOL
            && route.header.kind == RouteType::Unreachable
            && metric == Some(DEFAULT_IPV6_ROUTE_METRIC)
        {
            replace_tunnel_route(fwmark, ifname, true)?;
        }
    } else if defaults.len() > 1 {
        return Err(io::Error::other(
            "multiple IPv6 default routes in fwmark table",
        ));
    }
    Ok(())
}

/// Installs the tunnel route without deleting the reject first. If `existing_only` is true,
/// the route at the expected metric must already exist.
fn replace_tunnel_route(fwmark: u32, ifname: &str, existing_only: bool) -> Result<(), io::Error> {
    let index = nix::net::if_::if_nametoindex(ifname)?;
    let mut message = route_message(fwmark);
    message.header.protocol = RouteProtocol::Boot;
    message.header.scope = RouteScope::Link;
    message.header.kind = RouteType::Unicast;
    message.attributes.push(RouteAttribute::Oif(index));
    let mut flags = NLM_F_REQUEST | NLM_F_ACK | NLM_F_REPLACE;
    if existing_only {
        message
            .attributes
            .push(RouteAttribute::Priority(DEFAULT_IPV6_ROUTE_METRIC));
    } else {
        flags |= NLM_F_CREATE;
    }
    netlink_request(RouteNetlinkMessage::NewRoute(message), flags)
}

fn route_message(fwmark: u32) -> RouteMessage {
    let mut message = RouteMessage::default();
    message.header = RouteHeader {
        address_family: AddressFamily::Inet6,
        destination_prefix_length: 0,
        table: RouteHeader::RT_TABLE_UNSPEC,
        protocol: ROUTE_PROTOCOL,
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
    netlink_routes(message, flags).map(|_| ())
}

fn netlink_routes(
    message: RouteNetlinkMessage,
    flags: u16,
) -> Result<Vec<RouteMessage>, io::Error> {
    let mut routes = Vec::new();
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
            if response.header.flags & NLM_F_DUMP_INTR != 0 {
                return Err(io::Error::other("netlink route dump interrupted"));
            }
            let message_length = response.header.length as usize;
            if message_length == 0 || message_length > received - offset {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "invalid netlink response length",
                ));
            }

            match response.payload {
                NetlinkPayload::Error(message) if message.code.is_none() => return Ok(routes),
                NetlinkPayload::Error(message) => return Err(message.to_io()),
                NetlinkPayload::Done(message) if message.code == 0 => return Ok(routes),
                NetlinkPayload::Done(message) => {
                    return Err(io::Error::from_raw_os_error(message.code.saturating_neg()))
                }
                NetlinkPayload::Overrun(_) => return Err(io::Error::other("netlink overrun")),
                NetlinkPayload::InnerMessage(RouteNetlinkMessage::NewRoute(route)) => {
                    routes.push(route)
                }
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
