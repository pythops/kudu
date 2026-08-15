pub mod vnc;

use serde::{Deserialize, Serialize};

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RemoteAccess {
    Vnc(vnc::VNC),
    Spice,
}

impl RemoteAccess {
    pub fn to_qemu_arg(&self) -> Vec<String> {
        match self {
            RemoteAccess::Vnc(vnc) => vnc.to_qemu_arg(),
            RemoteAccess::Spice => {
                //TODO:
                todo!()
            }
        }
    }
}
