use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, strum::Display, Deserialize, Serialize)]
pub enum NetworkBackend {
    #[strum(to_string = "User - {0}")]
    User(UserMode),
}

impl NetworkBackend {
    pub fn to_qemu_arg(&self) -> Vec<&str> {
        vec![
            "-netdev",
            "user,id=net0",
            "-device",
            "virtio-net-pci,netdev=net0",
        ]
    }
}

impl Default for NetworkBackend {
    fn default() -> Self {
        NetworkBackend::User(UserMode::Slirp)
    }
}

#[derive(Debug, Default, Clone, Copy, strum::Display, Deserialize, Serialize)]
#[strum(serialize_all = "UPPERCASE")]
pub enum UserMode {
    #[default]
    Slirp,
    Passt,
}
