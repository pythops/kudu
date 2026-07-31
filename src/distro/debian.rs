use serde::{Deserialize, Serialize};

use crate::Arch;

#[derive(Debug, Default, Clone, Copy, PartialEq, strum_macros::Display, Deserialize, Serialize)]
pub enum DebianRelease {
    #[default]
    Trixie = 13,
    Bookworm = 12,
    Forky = 14,
}

impl DebianRelease {
    pub fn get_url(&self, arch: Arch) -> String {
        let arch = match arch {
            Arch::X86_64 => "amd64",
            Arch::Aarch64 => "arm64",
            Arch::Riscv64 => "riscv64",
        };

        format!(
            "http://cloud.debian.org/images/cloud/{}/latest/debian-{}-generic-{}.qcow2",
            self.to_string().to_lowercase(),
            *self as u8,
            arch
        )
    }
}
