#[cfg(any(windows, test))]
use serde::Deserialize;
use std::{future::Future, io, net::Ipv4Addr};

/// A currently active Windows Private interface, including its identity and subnet.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Interface {
    pub(crate) index: u32,
    pub(crate) address: Ipv4Addr,
    pub(crate) prefix: u8,
}
impl Interface {
    fn mask(&self) -> u32 {
        u32::MAX << (32 - self.prefix)
    }
    pub(crate) fn broadcast(&self) -> Ipv4Addr {
        Ipv4Addr::from(u32::from(self.address) | !self.mask())
    }
    pub(crate) fn contains(&self, address: Ipv4Addr) -> bool {
        address != self.broadcast()
            && u32::from(address) & self.mask() == u32::from(self.address) & self.mask()
            && u32::from(address) & !self.mask() != 0
    }
}

/// Read-only OS boundary. Tests supply isolated synthetic interface snapshots.
pub(crate) trait InterfaceProvider {
    fn snapshot(&mut self) -> impl Future<Output = io::Result<Vec<Interface>>> + Send;
}

pub(crate) struct WindowsInterfaces;
impl InterfaceProvider for WindowsInterfaces {
    async fn snapshot(&mut self) -> io::Result<Vec<Interface>> {
        snapshot().await
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg(any(windows, test))]
struct RawInterface {
    index: u32,
    address: Ipv4Addr,
    prefix: u8,
    private: bool,
}

#[cfg(any(windows, test))]
fn parse(bytes: &[u8]) -> io::Result<Vec<Interface>> {
    let raw: Vec<RawInterface> =
        serde_json::from_slice(bytes).map_err(|_| io::ErrorKind::InvalidData)?;
    if raw.len() > 64 {
        return Err(io::ErrorKind::InvalidData.into());
    }
    let mut accepted = Vec::new();
    for item in &raw {
        if !item.private
            || !(1..=30).contains(&item.prefix)
            || item.address.is_loopback()
            || item.address.is_unspecified()
            || item.address.is_multicast()
            || item.address.is_broadcast()
        {
            continue;
        }
        let interface = Interface {
            index: item.index,
            address: item.address,
            prefix: item.prefix,
        };
        // An overlapping Public/Domain/unknown route makes the egress policy ambiguous.
        let overlap = raw.iter().any(|other| {
            !other.private
                && (other.prefix > 32 || other.prefix == 0 || {
                    let mask = u32::MAX << (32 - interface.prefix.min(other.prefix));
                    u32::from(interface.address) & mask == u32::from(other.address) & mask
                })
        });
        if !overlap {
            accepted.push(interface);
        }
    }
    accepted.sort();
    accepted.dedup();
    if accepted.len() > 16 {
        return Err(io::ErrorKind::InvalidData.into());
    }
    Ok(accepted)
}

#[cfg(windows)]
async fn snapshot() -> io::Result<Vec<Interface>> {
    use std::{path::Path, process::Stdio, time::Duration};
    use tokio::io::AsyncReadExt as _;
    let system = std::env::var_os("SystemRoot").ok_or(io::ErrorKind::NotFound)?;
    let mut command = tokio::process::Command::new(
        Path::new(&system).join("System32/WindowsPowerShell/v1.0/powershell.exe"),
    );
    command
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            include_str!("interfaces.ps1"),
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    command.creation_flags(0x0800_0000);
    let mut child = command.spawn()?;
    let read = async {
        let mut bytes = Vec::new();
        child
            .stdout
            .take()
            .ok_or(io::ErrorKind::BrokenPipe)?
            .take(16_385)
            .read_to_end(&mut bytes)
            .await?;
        if bytes.len() > 16_384 || !child.wait().await?.success() {
            return Err(io::ErrorKind::InvalidData.into());
        }
        parse(&bytes)
    };
    tokio::time::timeout(Duration::from_secs(2), read)
        .await
        .map_err(|_| io::ErrorKind::TimedOut)?
}

#[cfg(not(windows))]
async fn snapshot() -> io::Result<Vec<Interface>> {
    Err(io::ErrorKind::Unsupported.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn excludes_other_profiles_and_ambiguous_routes() {
        let profiles = br#"[{"index":1,"address":"192.168.3.10","prefix":24,"private":true},{"index":2,"address":"10.0.0.3","prefix":24,"private":false}]"#;
        let values = parse(profiles).unwrap();
        assert_eq!(values.len(), 1);
        assert_eq!(values[0].broadcast(), Ipv4Addr::new(192, 168, 3, 255));
        let ambiguous = br#"[{"index":1,"address":"192.168.3.10","prefix":24,"private":true},{"index":2,"address":"192.168.3.20","prefix":24,"private":false}]"#;
        assert!(parse(ambiguous).unwrap().is_empty());
        assert!(parse(b"not-json").is_err());
    }
}
