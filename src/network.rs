pub mod builder;
pub mod port_forwarding;

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
#[derive(Debug, Clone, Default, PartialEq, Copy, strum::Display, Deserialize, Serialize)]
pub enum NetworkBackend {
    #[default]
    User,
    Passt,
    Tap,
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

        match self.backend {
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
            NetworkBackend::User => {
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
        }
    }
}
