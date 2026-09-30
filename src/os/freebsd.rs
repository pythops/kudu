use serde::{Deserialize, Serialize};

use crate::Arch::{self};

#[derive(Debug, Default, Clone, Copy, PartialEq, strum::Display, Deserialize, Serialize)]
pub enum FreebsdRelease {
    V14,
    #[default]
    V15,
    V16,
}

impl FreebsdRelease {
    pub fn get_url(&self, arch: Arch) -> String {
        let version = match self {
            FreebsdRelease::V14 => "14.5-STABLE",
            FreebsdRelease::V15 => "15.1-STABLE",
            FreebsdRelease::V16 => "16.0-CURRENT",
        };

        match arch {
            Arch::X86_64 => {
                format!(
                    "https://download.freebsd.org/ftp/snapshots/VM-IMAGES/{version}/amd64/Latest/FreeBSD-{version}-amd64-BASIC-CLOUDINIT-zfs.qcow2.xz"
                )
            }
            Arch::Aarch64 => {
                format!(
                    "https://download.freebsd.org/ftp/snapshots/VM-IMAGES/{version}/aarch64/Latest/FreeBSD-{version}-arm64-aarch64-BASIC-CLOUDINIT-zfs.qcow2.xz"
                )
            }
            Arch::Riscv64 => {
                format!(
                    "https://download.freebsd.org/ftp/snapshots/VM-IMAGES/{version}/riscv64/Latest/FreeBSD-{version}-riscv-riscv64-zfs.qcow2.xz"
                )
            }
        }
    }
}
