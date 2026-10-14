pub mod bridge;
pub mod builder;
pub mod port_forwarding;

use std::{fmt::Display, path::PathBuf};

use rustix::path::Arg;
use serde::{Deserialize, Serialize};

use port_forwarding::PortMapping;

pub type NetworkId = String;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Network {
    pub id: NetworkId,
    pub backend: NetworkBackend,
    pub nic: Nic,
    pub mac: Option<String>,
    pub port_mappings: Vec<PortMapping>,
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Default, strum::Display, Deserialize, Serialize)]
#[strum(serialize_all = "lowercase")]
pub enum Nic {
    #[default]
    #[strum(to_string = "virtio-net-pci")]
    Virtio,
    E1000,
    RTL8139,
}

#[non_exhaustive]
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub enum NetworkBackend {
    #[default]
    Slirp,
    Passt,
    Tap,
    Bridge(String),
    Vde((PathBuf, Option<u16>)),
}

impl Display for NetworkBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NetworkBackend::Slirp => {
                write!(f, "Slirp")
            }
            NetworkBackend::Passt => {
                write!(f, "Passt")
            }
            NetworkBackend::Tap => {
                write!(f, "Tap")
            }
            NetworkBackend::Bridge(interface) => {
                write!(f, "Bridge({})", interface)
            }
            NetworkBackend::Vde((socket, port)) => {
                let port = if let Some(port) = port {
                    port.to_string()
                } else {
                    "Auto".to_string()
                };
                write!(f, "Vde({}/{port})", socket.display())
            }
        }
    }
}

impl Default for Network {
    fn default() -> Self {
        let id = format!("{:08x}", rand::random::<u32>());
        Self {
            id,
            backend: NetworkBackend::default(),
            nic: Nic::default(),
            port_mappings: Vec::new(),
            mac: None,
        }
    }
}

impl Network {
    pub fn new(
        backend: NetworkBackend,
        nic: Nic,
        port_mappings: Vec<PortMapping>,
        mac: Option<String>,
    ) -> Network {
        let id = format!("{:08x}", rand::random::<u32>());
        Self {
            id,
            backend,
            nic,
            port_mappings,
            mac,
        }
    }

    pub fn to_qemu_arg(&self) -> Vec<String> {
        let id = format!("net{}", self.id);
        let device = ["-device".to_string(), {
            if let Some(mac) = &self.mac {
                format!("{},netdev={},mac={}", self.nic, id, mac)
            } else {
                format!("{},netdev={}", self.nic, id)
            }
        }];

        match &self.backend {
            NetworkBackend::Passt => {
                let mapping_arg = self
                    .port_mappings
                    .iter()
                    .map(|mapping| {
                        let protocol = match mapping.protocol {
                            port_forwarding::Protocol::TCP => "tcp-ports",
                            port_forwarding::Protocol::UDP => "udp-ports",
                        };

                        format!("{}={}:{}", protocol, mapping.host_port, mapping.guest_port)
                    })
                    .collect::<Vec<String>>()
                    .join(",");

                let netdev = ["-netdev".to_string(), {
                    if mapping_arg.is_empty() {
                        format!("passt,id={}", id)
                    } else {
                        format!("passt,id={},{}", id, mapping_arg)
                    }
                }];

                [&device[..], &netdev[..]].concat()
            }
            NetworkBackend::Slirp => {
                let mapping_arg = self
                    .port_mappings
                    .iter()
                    .map(|mapping| {
                        format!(
                            "hostfwd={}::{}-:{}",
                            mapping.protocol.to_string().to_lowercase(),
                            mapping.host_port,
                            mapping.guest_port
                        )
                    })
                    .collect::<Vec<String>>()
                    .join(",");

                let netdev = ["-netdev".to_string(), {
                    if mapping_arg.is_empty() {
                        format!("user,id={}", id)
                    } else {
                        format!("user,id={},{}", id, mapping_arg)
                    }
                }];

                [&device[..], &netdev[..]].concat()
            }
            NetworkBackend::Tap => {
                let netdev = [
                    "-netdev".to_string(),
                    format!("tap,id={},ifname=tap{},script=no,downscript=no", id, id),
                ];
                [&device[..], &netdev[..]].concat()
            }
            NetworkBackend::Bridge(br) => {
                let netdev = ["-netdev".to_string(), format!("bridge,br={br},id={id}")];
                [&device[..], &netdev[..]].concat()
            }
            NetworkBackend::Vde((socket, port)) => {
                let sock = socket.to_string_lossy().to_string();

                let netdev = if let Some(port) = port {
                    [
                        "-netdev".to_string(),
                        format!("vde,sock={sock},port={port},id={id}"),
                    ]
                } else {
                    ["-netdev".to_string(), format!("vde,sock={sock},id={id}")]
                };

                [&device[..], &netdev[..]].concat()
            }
        }
    }
}
