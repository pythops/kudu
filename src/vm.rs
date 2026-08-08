use anyhow::{Context, Result};
use chrono::Utc;
use crossterm::event::{KeyCode, KeyEvent};
use qapi::qmp::{RunState, VncInfo};
use serde::{Deserialize, Serialize};
use std::{
    fmt::Debug,
    fs::{self, File},
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
    sync::{Arc, atomic::AtomicBool, mpsc::Sender},
    thread::{self},
};

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Flex, Layout, Margin, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState},
};

use crate::{
    Arch, BootOption, KVM_ENABLED,
    cloudinit::Cloudinit,
    distro::LinuxDistro::{self, TempleOS},
    event::{
        DownloadEvent,
        Event::{self, Download, VMStarted},
    },
    get_kudu_data_dir, get_kudu_run_dir,
    notification::{Notification, NotificationLevel},
    qemu::Qemu,
    storage::{Drive, Format, Interface, Media},
    vmbuilder::VMBuildData,
    vmedit::VMEditData,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VM {
    pub id: uuid::Uuid,
    pub name: String,
    pub boot_option: BootOption,
    pub os: Option<LinuxDistro>,
    pub arch: Arch,
    pub vcpu: u16,
    pub memory: u32,
    pub drives: Vec<Drive>,

    #[serde(skip)]
    pub downloading: Arc<AtomicBool>,

    #[serde(skip)]
    pub events: Vec<String>,

    #[serde(skip)]
    pub events_state: ListState,

    #[serde(skip)]
    pub vnc: Option<VncInfo>,

    #[serde(skip)]
    #[serde(default = "default_state")]
    pub state: RunState,
}

fn default_state() -> RunState {
    RunState::shutdown
}

impl VM {
    pub fn list(sender: Sender<Event>) -> Result<Vec<Self>> {
        let mut vms = Vec::new();
        let dir = get_kudu_data_dir().join("vms");
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            for e in fs::read_dir(path)? {
                let entry = e?;
                let path = entry.path();
                if path.ends_with("vm.json") {
                    let buf = fs::read_to_string(path)?;

                    let mut vm: VM = serde_json::from_str(&buf)?;

                    if let Ok(state) = Qemu::status(vm.id) {
                        vm.state = state;

                        if let Ok(vnc_info) = Qemu::vnc_infos(vm.id) {
                            vm.vnc = Some(vnc_info);
                        }

                        let vm_events_path = VM::get_events_file(vm.id);
                        Qemu::events(vm_events_path, vm.id, sender.clone());
                    }

                    vm.load_events_from_file()?;

                    vms.push(vm);
                }
            }
        }
        Ok(vms)
    }

    pub fn preview(&self) {
        if let Some(vnc) = &self.vnc {
            thread::spawn({
                let vnc = vnc.clone();
                move || {
                    if let Ok(mut child) = Command::new("vncviewer")
                        .arg(format!(
                            "{}:{}",
                            vnc.host.clone().unwrap(),
                            vnc.service.clone().unwrap()
                        ))
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .spawn()
                    {
                        let _ = child.wait();
                    }
                }
            });
        }
    }

    pub fn create(data: VMBuildData) -> Result<VM> {
        let mut drives = Vec::new();

        let id = uuid::Uuid::new_v4();
        let mut path = get_kudu_data_dir().join("vms");
        path.push(id.to_string());
        fs::create_dir(&path)
            .with_context(|| format!("Can not create {}", path.to_string_lossy()))?;

        path.push("vm.json");
        let mut file = File::create(&path)
            .with_context(|| format!("Can not create {}", path.to_string_lossy()))?;
        path.pop();

        if let Some(cloudinit_path) = data.cloudinit {
            path.push("cloudinit.iso");
            Cloudinit::from_path(&cloudinit_path, &path)?;
            let drive = Drive {
                path: path.clone(),
                interface: Interface::Virtio,
                media: Media::CdRom,
                format: Format::Raw,
                read_only: true,
                unit: None,
                size: None,
            };
            drives.push(drive);
            path.pop();
        }

        if data.enable_uefi
            && let Ok((code, vars)) = VM::get_uefi_file_path(data.arch)
        {
            path.push("uefi_vars.fd");
            fs::copy(vars, &path)?;
            let vars = path.clone();
            path.pop();

            let uefi_code_drive = Drive {
                path: code,
                interface: Interface::Pflash,
                format: Format::Raw,
                unit: Some(0),
                media: Media::Disk,
                read_only: true,
                size: None,
            };

            let uefi_vars_drive = Drive {
                path: vars,
                interface: Interface::Pflash,
                format: Format::Raw,
                unit: Some(1),
                media: Media::Disk,
                read_only: false,
                size: None,
            };

            drives.push(uefi_code_drive);
            drives.push(uefi_vars_drive);
        }

        for (index, disk) in data.disks.iter().enumerate() {
            path.push(format!("disk_{}", index));

            disk.create(&path)
                .with_context(|| format!("Can no create {}", path.to_string_lossy()))?;

            let drive = Drive {
                path: path.clone(),
                interface: Interface::Virtio,
                format: disk.format,
                media: Media::Disk,
                read_only: false,
                unit: None,
                size: Some(disk.size),
            };

            drives.push(drive);
            path.pop();
        }

        if let Some(path) = data.boot_file {
            let drive = Drive {
                path: path.clone(),
                interface: Interface::Virtio,
                format: Format::Raw,
                media: Media::CdRom,
                read_only: true,
                unit: None,
                size: None,
            };

            drives.push(drive);
        }

        let vm = VM {
            id,
            name: data.name,
            boot_option: data.boot_option,
            os: data.os,
            arch: data.arch,
            vcpu: data.vcpu,
            memory: data.memory,
            drives,
            downloading: Arc::new(AtomicBool::new(false)),
            events: Vec::new(),
            events_state: ListState::default(),
            vnc: None,
            state: RunState::shutdown,
        };

        let data = serde_json::to_string_pretty(&vm)?;
        file.write_all(data.as_bytes())?;
        Ok(vm)
    }

    pub fn disks(&self) -> Vec<Drive> {
        self.drives
            .clone()
            .into_iter()
            .filter(|drive| {
                drive.interface == Interface::Virtio
                    && drive.media == Media::Disk
                    && drive.path != self.get_boot_file()
                    && drive.size.is_some()
            })
            .collect()
    }

    pub fn cdroms(&self) -> Vec<Drive> {
        self.drives
            .clone()
            .into_iter()
            .filter(|drive| {
                drive.interface == Interface::Virtio
                    && drive.media == Media::CdRom
                    && drive.path != self.get_boot_file()
            })
            .collect()
    }

    pub fn edit(&mut self, data: VMEditData) -> Result<()> {
        self.vcpu = data.new_vcpu;
        self.memory = data.new_memory;

        for path in data.deleted_disks {
            if path.exists() {
                fs::remove_file(&path)
                    .with_context(|| format!("can not remove {}", path.to_string_lossy()))?;
            }
            self.drives.retain(|drive| drive.path != path);
        }

        for path in data.delete_cdroms {
            self.drives.retain(|drive| drive.path != path);
        }

        let mut path = get_kudu_data_dir().join("vms");
        path.push(self.id.to_string());

        let disks_len = self.disks().len();
        for (index, disk) in data.added_disks.iter().enumerate() {
            let index = index + disks_len;
            path.push(format!("disk_{}", index));

            disk.create(&path)
                .with_context(|| format!("Can not create {}", path.to_string_lossy()))?;

            let drive = Drive {
                path: path.clone(),
                interface: Interface::Virtio,
                format: disk.format,
                media: Media::Disk,
                read_only: false,
                unit: None,
                size: None,
            };

            self.drives.push(drive);
            path.pop();
        }

        path.push("vm.json");
        let mut file = File::create(&path)?;
        let vm = serde_json::to_string_pretty(&self)?;
        file.write_all(vm.as_bytes())?;

        Ok(())
    }

    pub fn delete(&mut self) -> Result<()> {
        if self.state != RunState::shutdown {
            self.shutdown()?;
        }
        let mut path = get_kudu_data_dir().join("vms");
        path.push(self.id.to_string());
        fs::remove_dir_all(path)?;

        let mut path = get_kudu_run_dir();
        path.push(self.id.to_string());
        if path.exists() {
            fs::remove_dir_all(path)?;
        }

        Ok(())
    }

    fn get_pid_file(id: uuid::Uuid) -> PathBuf {
        let mut path = get_kudu_run_dir();
        path.push(id.to_string());
        path.push("pidfile");
        path
    }

    pub fn get_socket_file(id: uuid::Uuid) -> PathBuf {
        let mut path = get_kudu_run_dir();
        path.push(id.to_string());
        path.push("socket");
        path
    }
    pub fn get_events_file(id: uuid::Uuid) -> PathBuf {
        let mut path = get_kudu_run_dir();
        path.push(id.to_string());
        path.push("events");
        path
    }

    pub fn remove_pid_file(&self) -> Result<()> {
        let mut path = get_kudu_run_dir();
        path.push(self.id.to_string());
        path.push("pidfile");

        fs::remove_file(path)?;

        Ok(())
    }

    pub fn shutdown(&mut self) -> Result<()> {
        let mut path = get_kudu_run_dir();
        path.push(self.id.to_string());

        fs::remove_dir_all(path)?;

        self.state = RunState::shutdown;
        self.vnc = None;
        let now = Utc::now().format("%Y-%m-%d %H:%M:%S");
        let event = format!("{} - SHUTDOWN", now);
        self.events.push(event);

        Ok(())
    }

    pub fn get_boot_file(&self) -> PathBuf {
        let mut path = get_kudu_data_dir();
        path.push("vms");
        path.push(self.id.to_string());
        path.push("boot");
        path
    }

    pub fn start(&mut self, sender: Sender<Event>) -> Result<()> {
        if self.boot_option == BootOption::CloudImage {
            let distro = self.os.unwrap();
            if !distro.is_available(self.arch) {
                thread::spawn({
                    let vm = self.clone();
                    let sender = sender.clone();
                    move || {
                        if vm.downloading.load(std::sync::atomic::Ordering::Relaxed) {
                            return;
                        } else {
                            vm.downloading
                                .store(true, std::sync::atomic::Ordering::Relaxed);
                        }
                        if let Err(error) = distro.download(vm.arch, sender.clone(), vm.id) {
                            vm.downloading
                                .store(false, std::sync::atomic::Ordering::Relaxed);
                            let _ = sender
                                .send(Download((vm.id, DownloadEvent::Error(error.to_string()))));
                            return;
                        }
                        vm.downloading
                            .store(false, std::sync::atomic::Ordering::Relaxed);
                    }
                });
                return Ok(());
            } else {
                if !self.get_boot_file().exists()
                    && let Err(e) = fs::copy(distro.get_file_path(self.arch), self.get_boot_file())
                {
                    let _ = sender.send(Event::Notification(Notification::error(e)));
                }

                if Some(TempleOS) == self.os {
                    let drive = Drive {
                        path: self.get_boot_file(),
                        interface: Interface::Ide,
                        media: Media::CdRom,
                        format: Format::Raw,
                        unit: None,
                        read_only: true,
                        size: None,
                    };

                    if !self.drives.contains(&drive) {
                        self.drives.push(drive);
                    }
                } else {
                    let drive = Drive {
                        path: self.get_boot_file(),
                        interface: Interface::Virtio,
                        media: Media::Disk,
                        format: Format::Qcow2,
                        unit: None,
                        read_only: false,
                        size: None,
                    };

                    if !self.drives.contains(&drive) {
                        self.drives.push(drive);
                    }
                }

                let mut path = get_kudu_data_dir().join("vms");
                path.push(self.id.to_string());
                path.push("vm.json");
                let mut file = File::create(&path)?;
                let vm = serde_json::to_string_pretty(&self)?;
                file.write_all(vm.as_bytes())?;
            }
        }

        let mut path = get_kudu_run_dir();
        path.push(self.id.to_string());
        if !path.exists()
            && let Err(e) = fs::create_dir(&path)
        {
            let _ = sender.send(Event::Notification(Notification::error(e)));
        }

        match Qemu::start(self, sender.clone()) {
            Ok(pid) => {
                let pid_file_path = VM::get_pid_file(self.id);
                if let Err(e) = fs::write(pid_file_path, pid.to_string()) {
                    let _ = sender.send(Event::Notification(Notification::error(e)));
                }

                let _ = sender.send(VMStarted(self.id));
            }
            Err(e) => {
                let _ = sender.send(Event::Notification(Notification::error(e)));
            }
        }

        Ok(())
    }

    pub fn save_events_to_file(&self) -> Result<()> {
        let mut path = get_kudu_data_dir();
        path.push("vms");
        path.push(self.id.to_string());
        path.push("events.json");

        let mut file = File::create(&path)?;

        let events = serde_json::to_string_pretty(&self.events)?;
        file.write_all(events.as_bytes())?;

        Ok(())
    }

    pub fn load_events_from_file(&mut self) -> Result<()> {
        let mut path = get_kudu_data_dir();
        path.push("vms");
        path.push(self.id.to_string());
        path.push("events.json");

        let buf = fs::read_to_string(path)?;
        let events: Vec<String> = serde_json::from_str(&buf)?;

        self.events = events;

        Ok(())
    }

    pub fn handle_key_events(&mut self, key_event: KeyEvent, sender: Sender<Event>) -> Result<()> {
        match key_event.code {
            KeyCode::Char(' ') => match self.state {
                RunState::running => {
                    Qemu::pause(self.id)?;
                }
                RunState::paused => {
                    Qemu::resume(self.id)?;
                }
                RunState::shutdown => {
                    let _ = self.start(sender);
                }
                _ => {}
            },
            KeyCode::Char('s') => {
                if let Err(error) = Qemu::power_down(self.id) {
                    let notif =
                        crate::notification::Notification::new(error, NotificationLevel::Error);

                    let _ = sender.send(Event::Notification(notif));
                }
            }
            KeyCode::Char('x') if self.state != RunState::shutdown => {
                if let Err(error) = Qemu::quit(self.id) {
                    let notif =
                        crate::notification::Notification::new(error, NotificationLevel::Error);

                    let _ = sender.send(Event::Notification(notif));
                }
            }

            _ => {}
        }

        Ok(())
    }

    pub fn render(&mut self, frame: &mut Frame, block: Rect) {
        let (infos_block, events_block) = {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Fill(1), Constraint::Length(10)])
                .flex(Flex::SpaceBetween)
                .split(block);
            (chunks[0], chunks[1])
        };

        frame.render_widget(
            Block::new()
                .borders(Borders::all())
                .border_type(BorderType::Rounded)
                .title(" Information ")
                .title_alignment(ratatui::layout::HorizontalAlignment::Center)
                .border_style(Style::default().yellow()),
            infos_block,
        );

        frame.render_widget(
            Block::new()
                .borders(Borders::all())
                .border_type(BorderType::Rounded)
                .title(" Events ")
                .title_alignment(ratatui::layout::HorizontalAlignment::Center)
                .border_style(Style::default().yellow()),
            events_block,
        );

        let enable_uefi = self
            .drives
            .iter()
            .any(|drive| drive.interface == Interface::Pflash);

        let mut items = vec![
            ListItem::from(vec![
                Line::from(vec![
                    Span::from("Id      ").bold().fg(Color::Yellow),
                    Span::from(" ".repeat(4)),
                    Span::from(self.id.to_string()),
                ]),
                Line::from(""),
            ]),
            ListItem::from(vec![
                Line::from(vec![
                    Span::from("Name    ").bold().fg(Color::Yellow),
                    Span::from(" ".repeat(4)),
                    Span::from(&self.name),
                ]),
                Line::from(""),
            ]),
            ListItem::from(vec![
                Line::from(vec![
                    Span::from("State   ").bold().fg(Color::Yellow),
                    Span::from(" ".repeat(4)),
                    Span::from(format!("{:?}", self.state)),
                ]),
                Line::from(""),
            ]),
            ListItem::from(vec![
                Line::from(vec![
                    Span::from("vCPU    ").bold().fg(Color::Yellow),
                    Span::from(" ".repeat(4)),
                    Span::from(self.vcpu.to_string()),
                ]),
                Line::from(""),
            ]),
            ListItem::from(vec![
                Line::from(vec![
                    Span::from("Memory  ").bold().fg(Color::Yellow),
                    Span::from(" ".repeat(4)),
                    Span::from(format!("{} MB", self.memory)),
                ]),
                Line::from(""),
            ]),
            ListItem::from(vec![
                Line::from(vec![
                    Span::from("Firmware").bold().fg(Color::Yellow),
                    Span::from(" ".repeat(4)),
                    Span::from(if enable_uefi { "UEFI" } else { "BIOS" }),
                ]),
                Line::from(""),
            ]),
            ListItem::from({
                let mut lines = Vec::new();
                let disks = self.disks();
                if disks.is_empty() {
                    vec![
                        Line::from(vec![
                            Span::from("Disks  ").bold().fg(Color::Yellow),
                            Span::from(" ".repeat(4)),
                            Span::from(" - "),
                        ]),
                        Line::from(""),
                    ]
                } else {
                    lines.push(Line::from(vec![
                        Span::from("Disks  ").bold().fg(Color::Yellow),
                        Span::from(" ".repeat(4)),
                        Span::from(format!(
                            " Disk 0: size={}GiB, format={}",
                            disks[0].size.unwrap(),
                            disks[0].format
                        )),
                    ]));
                    for (index, disk) in disks.iter().skip(1).enumerate() {
                        lines.push(Line::from(vec![
                            Span::from(" ".repeat(12)),
                            Span::from(format!(
                                "Disk {}: size={}GiB, format={}",
                                index + 1,
                                disk.size.unwrap(),
                                disk.format
                            )),
                        ]))
                    }
                    lines.push(Line::from(""));
                    lines
                }
            }),
            ListItem::from(vec![
                Line::from(vec![
                    Span::from("KVM     ").bold().fg(Color::Yellow),
                    Span::from(" ".repeat(4)),
                    Span::from({
                        if let Ok(host_arch) = Arch::try_from(std::env::consts::ARCH)
                            && host_arch == self.arch
                            && unsafe { KVM_ENABLED }
                        {
                            "Enabled"
                        } else {
                            "Disabled"
                        }
                    }),
                ]),
                Line::from(""),
            ]),
            ListItem::from(vec![
                Line::from(vec![
                    Span::from("Arch    ").bold().fg(Color::Yellow),
                    Span::from(" ".repeat(4)),
                    Span::from(self.arch.to_string()),
                ]),
                Line::from(""),
            ]),
        ];

        if self.boot_option == BootOption::CloudImage {
            items.push(ListItem::from(vec![
                Line::from(vec![
                    Span::from("Distro  ").bold().fg(Color::Yellow),
                    Span::from(" ".repeat(4)),
                    Span::from(self.os.unwrap().to_string()),
                ]),
                Line::from(""),
            ]))
        }

        if let Some(vnc_info) = self.vnc.clone() {
            items.push(ListItem::from(vec![
                Line::from(vec![
                    Span::from("Vnc     ").bold().fg(Color::Yellow),
                    Span::from(" ".repeat(4)),
                    Span::from({
                        if vnc_info.enabled {
                            format!("{} {}", vnc_info.host.unwrap(), vnc_info.service.unwrap())
                        } else {
                            "Disabled".to_string()
                        }
                    }),
                ]),
                Line::from(""),
            ]));
        }
        let list = List::new(items);

        frame.render_widget(
            list,
            infos_block.inner(Margin {
                horizontal: 4,
                vertical: 2,
            }),
        );

        // Events
        let events = if self.events.len() < 8 {
            self.events.clone()
        } else {
            self.events.last_chunk::<8>().unwrap().to_vec()
        };

        let events = events.iter().map(|event| {
            let splits: Vec<&str> = event.split(" - ").collect();
            Line::from(vec![
                Span::from(splits[0]).green(),
                Span::from(" - "),
                Span::from(splits[1]).blue(),
            ])
        });

        let list = List::new(events);
        frame.render_stateful_widget(
            list,
            events_block.inner(Margin {
                horizontal: 1,
                vertical: 1,
            }),
            &mut self.events_state,
        );
    }

    pub fn get_uefi_file_path(arch: Arch) -> Result<(PathBuf, PathBuf)> {
        if let Ok(Some(os)) = LinuxDistro::get_from_os_release() {
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
}
