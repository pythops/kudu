use anyhow::Result;
use qapi::{
    Qmp,
    qmp::{self, RunState, SetPasswordOptions, SetPasswordOptionsBase, VncInfo},
};
use std::{
    os::unix::net::UnixStream,
    path::PathBuf,
    process::{Command, Stdio},
    sync::mpsc::Sender,
    thread::{self},
};

use crate::{
    Arch, KVM_ENABLED,
    event::Event,
    vm::{VM, VmId},
};

#[derive(Debug)]
pub struct Qemu;

impl Qemu {
    pub fn start(vm: &VM, sender: Sender<Event>) -> Result<u32> {
        let vm_socket_path = VM::get_socket_file(vm.id);
        let vm_events_path = VM::get_events_file(vm.id);

        let mut command = match vm.arch {
            Arch::X86_64 => {
                let mut command = Command::new("qemu-system-x86_64");
                command
                    .arg("-device")
                    .arg("VGA,edid=on,xres=1920,yres=1080");

                command
            }
            Arch::Aarch64 => {
                let mut command = Command::new("qemu-system-aarch64");
                command
                    .arg("-machine")
                    .arg("virt")
                    .arg("-cpu")
                    .arg("max")
                    .arg("-device")
                    .arg("virtio-gpu-pci")
                    .arg("-device")
                    .arg("qemu-xhci")
                    .arg("-device")
                    .arg("usb-kbd");

                command
            }
            Arch::Riscv64 => {
                let mut command = Command::new("qemu-system-riscv64");
                command
                    .arg("-machine")
                    .arg("virt")
                    .arg("-cpu")
                    .arg("max")
                    .arg("-device")
                    .arg("virtio-gpu-pci")
                    .arg("-device")
                    .arg("qemu-xhci")
                    .arg("-device")
                    .arg("usb-kbd");

                command
            }
        };

        if vm.networks.is_empty() {
            command.arg("-nic").arg("none");
        } else {
            for network in &vm.networks {
                command.args(network.to_qemu_arg());
            }
        }

        if let Ok(host_arch) = Arch::try_from(std::env::consts::ARCH)
            && host_arch == vm.arch
            && unsafe { KVM_ENABLED }
        {
            command.arg("-enable-kvm");
        }

        if let Some(remote_access) = &vm.remote_access {
            command.args(remote_access.to_qemu_arg());
        } else {
            command.arg("-vnc").arg("none");
        }

        for fs in &vm.fs {
            command.args(fs.to_qemu_arg());
        }

        command
            .arg("-daemonize")
            .arg("-qmp")
            .arg(format!(
                "unix:{},server,wait=off",
                vm_socket_path.to_string_lossy()
            ))
            .arg("-qmp")
            .arg(format!(
                "unix:{},server,wait=off",
                vm_events_path.to_string_lossy()
            ))
            .arg("-m")
            .arg(vm.memory.to_string())
            .arg("-smp")
            .arg(vm.vcpu.to_string())
            .arg("-boot")
            .arg("order=d");

        for drive in &vm.drives {
            command.arg("-drive").arg(drive.to_qemu_arg());
        }

        command.stdout(Stdio::null());
        command.stderr(Stdio::piped());

        let child = command.spawn()?;

        let pid = child.id();

        let output = child.wait_with_output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(anyhow::anyhow!(stderr));
        }

        Qemu::events(vm_events_path, vm.id, sender);

        Ok(pid)
    }

    pub fn events(vm_events_path: PathBuf, id: VmId, sender: Sender<Event>) {
        thread::spawn(move || {
            if let Ok(stream) = UnixStream::connect(vm_events_path) {
                let mut qmp = Qmp::from_stream(&stream);
                let Ok(_handshake) = qmp.handshake() else {
                    return;
                };

                loop {
                    if qmp.nop().is_err() {
                        break;
                    }
                    for event in qmp.events() {
                        let _ = sender.send(Event::QemuEvent((id, event)));
                    }
                }

                let _ = sender.send(Event::QemuExit(id));
            }
        });
    }

    pub fn pause(id: VmId) -> Result<()> {
        let socket = VM::get_socket_file(id);
        let stream = UnixStream::connect(socket)?;
        let mut qmp = Qmp::from_stream(&stream);
        let _ = qmp.handshake()?;
        qmp.execute(&qmp::stop {})?;
        Ok(())
    }

    pub fn resume(id: VmId) -> Result<()> {
        let socket = VM::get_socket_file(id);
        let stream = UnixStream::connect(socket)?;
        let mut qmp = Qmp::from_stream(&stream);
        let _ = qmp.handshake()?;
        qmp.execute(&qmp::cont {})?;
        Ok(())
    }

    pub fn quit(id: VmId) -> Result<()> {
        let socket = VM::get_socket_file(id);
        let stream = UnixStream::connect(socket)?;
        let mut qmp = Qmp::from_stream(&stream);
        let _ = qmp.handshake()?;
        qmp.execute(&qmp::quit {})?;
        Ok(())
    }

    pub fn power_down(id: VmId) -> Result<()> {
        let socket = VM::get_socket_file(id);
        let stream = UnixStream::connect(socket)?;
        let mut qmp = Qmp::from_stream(&stream);
        let _ = qmp.handshake()?;
        qmp.execute(&qmp::system_powerdown {})?;
        Ok(())
    }

    pub fn vnc_infos(id: VmId) -> Result<VncInfo> {
        let socket = VM::get_socket_file(id);
        let stream = UnixStream::connect(socket)?;
        let mut qmp = Qmp::from_stream(&stream);
        let _ = qmp.handshake()?;
        Ok(qmp.execute(&qmp::query_vnc {})?)
    }

    pub fn status(id: VmId) -> Result<RunState> {
        let socket = VM::get_socket_file(id);
        let stream = UnixStream::connect(socket)?;
        let mut qmp = Qmp::from_stream(&stream);
        let _ = qmp.handshake().expect("handshake failed");
        let status = qmp.execute(&qmp::query_status {})?;
        Ok(status.status)
    }

    pub fn set_password(id: VmId, password: String) -> Result<()> {
        let socket = VM::get_socket_file(id);
        let stream = UnixStream::connect(socket)?;
        let mut qmp = Qmp::from_stream(&stream);
        let _ = qmp.handshake()?;
        let password = SetPasswordOptions::vnc {
            base: SetPasswordOptionsBase {
                password,
                connected: None,
            },
            vnc: qmp::SetPasswordOptionsVnc { display: None },
        };
        qmp.execute(&qmp::set_password(password))?;
        Ok(())
    }
}
