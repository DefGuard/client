#[cfg(target_os = "linux")]
mod linux;
#[cfg(windows)]
mod windows;

#[cfg(target_os = "linux")]
pub(crate) use linux::Ipv6Block;
#[cfg(windows)]
pub(crate) use windows::Ipv6Block;
