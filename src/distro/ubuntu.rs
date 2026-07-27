use serde::{Deserialize, Serialize};

use crate::Arch;

#[derive(Debug, Default, Clone, Copy, strum_macros::Display, Deserialize, Serialize)]
pub enum UbuntuRelease {
    #[default]
    Resolute,
    Noble,
    Jammy,
}

impl UbuntuRelease {
    pub fn get_url(&self, arch: Arch) -> String {
        let arch = match arch {
            Arch::X86_64 => "amd64",
            Arch::Aarch64 => "arm64",
            Arch::Riscv64 => "riscv64",
        };

        format!(
            "https://cloud-images.ubuntu.com/{}/current/{}-server-cloudimg-{}.img",
            self.to_string().to_lowercase(),
            self.to_string().to_lowercase(),
            arch
        )
    }

    pub fn get_number(&self) -> f32 {
        match self {
            UbuntuRelease::Resolute => 26.04,
            UbuntuRelease::Noble => 24.04,
            UbuntuRelease::Jammy => 22.04,
        }
    }
}
