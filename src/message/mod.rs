//! Messaging primitives for discovering devices and services.

use std::io;
#[cfg(windows)]
use std::net;
use std::net::SocketAddr;

use net::connector::UdpConnector;
use net::IpVersionMode;

mod notify;
mod search;
mod ssdp;
mod listen;

pub use message::search::{SearchRequest, SearchResponse, SearchListener};
pub use message::notify::{NotifyMessage, NotifyListener};
pub use message::listen::Listen;

#[cfg(not(windows))]
use ifaces;

/// Multicast Socket Information
const UPNP_MULTICAST_IPV4_ADDR: &'static str = "239.255.255.250";
const UPNP_MULTICAST_IPV6_LINK_LOCAL_ADDR: &'static str = "FF02::C";
pub const UPNP_MULTICAST_PORT: u16 = 1900;

/// Default TTL For Multicast
const UPNP_MULTICAST_TTL: u32 = 2;

/// Enumerates different types of SSDP messages.
#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub enum MessageType {
    /// A notify message.
    Notify,
    /// A search message.
    Search,
    /// A response to a search message.
    Response,
}

/// Generate `UdpConnector` objects for all local `IPv4` interfaces.
fn all_local_connectors(multicast_ttl: Option<u32>, filter: IpVersionMode) -> io::Result<Vec<UdpConnector>> {
    trace!("Fetching all local connectors");
    map_local(|&addr| match (&filter, addr) {
        (&IpVersionMode::V4Only, SocketAddr::V4(n)) |
        (&IpVersionMode::Any, SocketAddr::V4(n)) => {
            Ok(Some(try!(UdpConnector::new((*n.ip(), 0), multicast_ttl))))
        }
        (&IpVersionMode::V6Only, SocketAddr::V6(n)) |
        (&IpVersionMode::Any, SocketAddr::V6(n)) => Ok(Some(try!(UdpConnector::new(n, multicast_ttl)))),
        _ => Ok(None),
    })
}

/// Invoke the closure for every local address found on the system
///
/// This method filters out _loopback_ and _global_ addresses.
fn map_local<F, R>(mut f: F) -> io::Result<Vec<R>>
    where F: FnMut(&SocketAddr) -> io::Result<Option<R>>
{
    let addrs_iter = try!(get_local_addrs());

    let mut obj_list = Vec::with_capacity(addrs_iter.len());

    for addr in addrs_iter {
        trace!("Found {}", addr);
        match addr {
            SocketAddr::V4(n) if !n.ip().is_loopback() => {
                if let Some(x) = try!(f(&addr)) {
                    obj_list.push(x);
                }
            }
            // Filter all loopback and global IPv6 addresses
            SocketAddr::V6(n) if !n.ip().is_loopback() && !n.ip().is_global() => {
                if let Some(x) = try!(f(&addr)) {
                    obj_list.push(x);
                }
            }
            _ => (),
        }
    }

    Ok(obj_list)
}

/// Generate a list of some object R constructed from all local `Ipv4Addr` objects.
///
/// If any of the `SocketAddr`'s fail to resolve, this function will not return an error.
#[cfg(windows)]
fn get_local_addrs() -> io::Result<Vec<SocketAddr>> {
    let host_iter = try!(net::lookup_host(""));
    Ok(host_iter.collect())
}

/// Generate a list of some object R constructed from all local `Ipv4Addr` objects.
///
/// If any of the `SocketAddr`'s fail to resolve, this function will not return an error.
#[cfg(not(windows))]
fn get_local_addrs() -> io::Result<Vec<SocketAddr>> {
    let iface_iter = try!(ifaces::Interface::get_all()).into_iter();
    Ok(iface_iter.filter(|iface| iface.kind != ifaces::Kind::Packet)
        .filter_map(|iface| iface.addr)
        .collect())
}
