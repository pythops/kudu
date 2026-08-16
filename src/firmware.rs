use anyhow::Result;
use std::path::PathBuf;

use crate::{Arch, os::Os};

pub fn get_uefi_file_path(arch: Arch) -> Result<(PathBuf, PathBuf)> {
    if let Ok(Some(os)) = Os::get_from_os_release() {
        match os.as_str() {
            "ubuntu" | "debian" => match arch {
                Arch::X86_64 => Ok((
                    PathBuf::from("/usr/share/OVMF/OVMF_CODE_4M.fd"),
                    PathBuf::from("/usr/share/OVMF/OVMF_VARS_4M.fd"),
                )),
                Arch::Aarch64 => Ok((
                    PathBuf::from("/usr/share/AAVMF/AAVMF_CODE.fd"),
                    PathBuf::from("/usr/share/AAVMF/AAVMF_VARS.fd"),
                )),
                Arch::Riscv64 => Ok((
                    PathBuf::from("/usr/share/qemu-efi-riscv64/RISCV_VIRT_CODE.fd"),
                    PathBuf::from("/usr/share/qemu-efi-riscv64/RISCV_VIRT_VARS.fd"),
                )),
            },
            "arch" => match arch {
                Arch::X86_64 => Ok((
                    PathBuf::from("/usr/share/edk2/x64/OVMF.4m.fd"),
                    PathBuf::from("/usr/share/edk2/x64/OVMF_VARS.4m.fd"),
                )),
                Arch::Aarch64 => Ok((
                    PathBuf::from("/usr/share/edk2/aarch64/QEMU_EFI.fd"),
                    PathBuf::from("/usr/share/edk2/aarch64/QEMU_VARS.fd"),
                )),
                Arch::Riscv64 => Ok((
                    PathBuf::from("/usr/share/edk2/riscv64/RISCV_VIRT_CODE.fd"),
                    PathBuf::from("/usr/share/edk2/riscv64/RISCV_VIRT_VARS.fd"),
                )),
            },
            "fedora" | "rhel" => match arch {
                Arch::X86_64 => Ok((
                    PathBuf::from("/usr/share/edk2/ovmf/OVMF_CODE.fd"),
                    PathBuf::from("/usr/share/edk2/ovmf/OVMF_VARS.fd"),
                )),
                Arch::Aarch64 => Ok((
                    PathBuf::from("/usr/share/edk2/aarch64/QEMU_EFI.fd"),
                    PathBuf::from("/usr/share/edk2/aarch64/QEMU_VARS.fd"),
                )),
                _ => Err(anyhow::anyhow!("Unsupported Arch")),
            },
            _ => Err(anyhow::anyhow!("Unsupported OS")),
        }
    } else {
        Err(anyhow::anyhow!("Can not recognize the OS"))
    }
}

pub fn uefi_package(arch: Arch) -> Result<&'static str> {
    if let Ok(Some(os)) = Os::get_from_os_release() {
        match os.as_str() {
            "ubuntu" | "debian" => match arch {
                Arch::X86_64 => Ok("ovmf"),
                Arch::Aarch64 => Ok("qemu-efi-aarch64"),
                Arch::Riscv64 => Ok("qemu-efi-riscv64"),
            },
            "arch" => match arch {
                Arch::X86_64 => Ok("edk2-ovmf"),
                Arch::Aarch64 => Ok("edk2-aarch64"),
                Arch::Riscv64 => Ok("edk2-riscv64"),
            },
            "fedora" | "rhel" => match arch {
                Arch::X86_64 => Ok("edk2-ovmf"),
                Arch::Aarch64 => Ok("edk2-aarch64"),
                _ => Err(anyhow::anyhow!("Unsupported Arch")),
            },
            _ => Err(anyhow::anyhow!("Unsupported OS")),
        }
    } else {
        Err(anyhow::anyhow!("Can not recognize the OS"))
    }
}
