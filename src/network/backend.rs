use std::path::Path;

use super::port_forwarding::PortMapping;

use serde::{Deserialize, Serialize};

#[non_exhaustive]
#[derive(Debug, Clone, Default, PartialEq, Copy, strum::Display, Deserialize, Serialize)]
pub enum NetworkBackend {
    #[default]
    User,
    Passt,
}

impl NetworkBackend {
    pub fn to_qemu_arg(
        &self,
        port_mappings: &[PortMapping],
        socket_path: Option<&Path>,
    ) -> Vec<String> {
        let mapping_arg = port_mappings
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

        let mut args = vec![
            "-device".to_string(),
            "virtio-net-pci,netdev=net0".to_string(),
            "-netdev".to_string(),
        ];

        match self {
            NetworkBackend::User => {
                if mapping_arg.is_empty() {
                    args.push("user,id=net0".to_string());
                } else {
                    args.push(format!("user,id=net0,{}", mapping_arg));
                }
            }
            NetworkBackend::Passt => {
                if let Some(path) = socket_path {
                    args.push(format!(
                        "stream,id=net0,server=off,addr.type=unix,addr.path={},{}",
                        path.to_string_lossy(),
                        mapping_arg
                    ));
                }
            }
        }

        args
    }
}
