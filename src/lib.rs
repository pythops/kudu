use std::{env, path::PathBuf};

use serde::{Deserialize, Serialize};

pub mod access;
pub mod app;
pub mod cloudinit;
pub mod confirmation;
pub mod event;
pub mod firmware;
pub mod handlers;
pub mod help;
pub mod network;
pub mod notification;
pub mod os;
pub mod qemu;
pub mod storage;
pub mod tui;
pub mod uefi;
pub mod ui;
pub mod vm;
pub mod vmbuilder;
pub mod vmedit;

static mut KVM_ENABLED: bool = false;

fn get_kudu_data_dir() -> PathBuf {
    if unsafe { libc::geteuid() } == 0 {
        PathBuf::from("/var/lib/kudu")
    } else {
        env::home_dir().unwrap().join(".local/share/kudu")
    }
}

fn get_kudu_run_dir() -> PathBuf {
    let pid = unsafe { libc::geteuid() };
    if pid == 0 {
        PathBuf::from("/var/lib/kudu/run")
    } else {
        PathBuf::from(format!("/run/user/{}/kudu", pid))
    }
}

#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, Hash, strum::Display, Deserialize, Serialize,
)]
pub enum Arch {
    #[default]
    X86_64,
    Aarch64,
    Riscv64,
}

impl TryFrom<&str> for Arch {
    type Error = &'static str;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "x86_64" => Ok(Arch::X86_64),
            "aarch64" => Ok(Arch::Aarch64),
            "riscv64" => Ok(Arch::Riscv64),
            _ => Err("Unknown"),
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum BootOption {
    #[default]
    CloudImage,
    LocalFile,
}
