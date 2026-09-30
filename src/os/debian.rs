use serde::{Deserialize, Serialize};

use crate::Arch;

#[derive(Debug, Default, Clone, Copy, PartialEq, strum::Display, Deserialize, Serialize)]
pub enum DebianRelease {
    #[default]
    Trixie,
    Bookworm,
    Forky,
}

impl DebianRelease {
    pub fn get_url(&self, arch: Arch) -> String {
        let arch = match arch {
            Arch::X86_64 => "amd64",
            Arch::Aarch64 => "arm64",
            Arch::Riscv64 => "riscv64",
        };

        let name = self.to_string().to_lowercase();
        let version = self.get_version_number();

        format!(
            "http://cloud.debian.org/images/cloud/{name}/latest/debian-{version}-generic-{arch}.qcow2",
        )
    }

    pub fn get_version_number(&self) -> &'static str {
        match self {
            DebianRelease::Trixie => "13",
            DebianRelease::Bookworm => "12",
            DebianRelease::Forky => "14",
        }
    }
}
