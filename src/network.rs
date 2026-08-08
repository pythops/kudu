use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, strum::Display, Deserialize, Serialize)]
pub enum NetworkBackend {
    #[strum(to_string = "User - {0}")]
    User(UserMode),
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
