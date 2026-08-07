use std::fs::File;
use std::io::Read;
use std::process::Command;
use std::thread;
use std::{fs::create_dir_all, sync::mpsc::Sender};

use anyhow::Context;
use kvm_ioctls::Kvm;
use ratatui::layout::Rect;
use rustix::fs::statfs;
use which::which;

use anyhow::Result;
use anyhow::anyhow;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Margin},
    style::{Color, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, HighlightSpacing, List, ListItem, ListState},
};

use crate::KVM_ENABLED;
use crate::confirmation::delete::DeleteConfirmation;
use crate::vmedit::EditVM;
use crate::{
    Arch, event::Event, get_kudu_data_dir, get_kudu_run_dir, help::Help,
    notification::Notification, vm::VM, vmbuilder::VMBuilder,
};

#[derive(Debug, Default, PartialEq)]
pub enum FocusedSection {
    #[default]
    Main,
    NewVM,
    EditVM,
    DeleteConfirmation,
}

#[derive(Debug, Default)]
pub enum NewVMFocusedSection {
    #[default]
    Name,
    CPU,
    Memory,
    Disk,
}

#[derive(Debug)]
pub struct App {
    pub running: bool,
    pub notifications: Vec<Notification>,
    pub focused_section: FocusedSection,
    pub help: Help,
    pub vms: Vec<VM>,
    pub new_vm: Option<VMBuilder>,
    pub edit_vm: Option<EditVM>,
    pub vm_list_state: ListState,
    pub available_uefi: Vec<Arch>,
    pub available_archs: Vec<Arch>,
    pub kvm_version: Result<u8>,
    total_memory: Result<usize>,
    pub delete_confirmation: Option<DeleteConfirmation>,
}

impl App {
    fn init() -> Result<()> {
        let dir = get_kudu_data_dir();
        if !dir.exists() {
            create_dir_all(&dir)?;

            let vms_dir = dir.join("vms");
            create_dir_all(&vms_dir)?;

            // Disable COW on vms dir if btrfs
            if let Ok(stat) = statfs(&vms_dir)
                && stat.f_type as u64 == 0x9123683E
            {
                which("chattr").with_context(|| anyhow!("chattr command not found."))?;
                Command::new("chattr").arg("+C").arg(vms_dir).output()?;
            }

            let storage_dir = dir.join("storage");
            create_dir_all(storage_dir)?;
        }

        let dir = get_kudu_run_dir();

        if !dir.exists() {
            create_dir_all(dir)?;
        }

        Ok(())
    }

    fn get_kvm_version() -> Result<u8> {
        let kvm = Kvm::new()?;
        Ok(kvm.get_api_version() as u8)
    }

    fn get_total_memory() -> Result<usize> {
        let mut memory_file = File::open("/proc/meminfo").context("Faile to open /proc/meminfo")?;
        let mut buffer = String::new();
        memory_file.read_to_string(&mut buffer)?;

        let mut total = 0;

        for line in buffer.lines() {
            let mut parts: Vec<&str> = line.split_whitespace().collect();
            parts.truncate(2);

            if let ["MemTotal:", v] = parts.as_slice() {
                total = v.parse::<usize>()?;
            }
        }

        Ok(total / 1024)
    }

    pub fn new(sender: Sender<Event>) -> Result<App> {
        App::init()?;

        let mut vms = Vec::new();

        if let Ok(loaded_vms) = VM::list(sender) {
            vms.extend_from_slice(&loaded_vms);
        }

        let vm_list_state = if !vms.is_empty() {
            ListState::default().with_selected(Some(0))
        } else {
            ListState::default()
        };

        let mut available_archs = Vec::new();
        let mut available_uefi = Vec::new();

        if which("qemu-system-x86_64").is_ok() {
            available_archs.push(Arch::X86_64);
            if let Ok((uefi, vars)) = VM::get_uefi_file_path(Arch::X86_64)
                && uefi.exists()
                && vars.exists()
            {
                available_uefi.push(Arch::X86_64);
            }
        }
        if which("qemu-system-aarch64").is_ok() {
            available_archs.push(Arch::Aarch64);
            if let Ok((uefi, vars)) = VM::get_uefi_file_path(Arch::Aarch64)
                && uefi.exists()
                && vars.exists()
            {
                available_uefi.push(Arch::Aarch64);
            }
        }
        if which("qemu-system-riscv64").is_ok() {
            available_archs.push(Arch::Riscv64);
            if let Ok((uefi, vars)) = VM::get_uefi_file_path(Arch::Riscv64)
                && uefi.exists()
                && vars.exists()
            {
                available_uefi.push(Arch::Riscv64);
            }
        }

        if available_archs.is_empty() {
            return Err(anyhow!(
                "Qemu is not installed.\n\
                Please install one of the packages:\n\
                qemu-system-x86_64\n\
                qemu-system-aarch64\n\
                qemu-system-riscv64"
            ));
        }

        which("xorriso").with_context(|| "Please install xorriso package.")?;

        let kvm_version = App::get_kvm_version();

        if kvm_version.is_ok() {
            unsafe { KVM_ENABLED = true };
        }

        Ok(Self {
            running: true,
            focused_section: FocusedSection::default(),
            notifications: Vec::new(),
            help: Help,
            vms,
            new_vm: None,
            edit_vm: None,
            vm_list_state,
            available_archs,
            available_uefi,
            kvm_version,
            total_memory: App::get_total_memory(),
            delete_confirmation: None,
        })
    }

    fn render_host_infos(&self, block: Rect, frame: &mut Frame) {
        frame.render_widget(
            Block::new()
                .borders(Borders::all())
                .border_type(BorderType::Rounded)
                .title(" Host ")
                .title_alignment(ratatui::layout::HorizontalAlignment::Center)
                .border_style(Style::default().yellow()),
            block,
        );

        let (left, right) = {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Length(40), Constraint::Length(50)])
                .flex(ratatui::layout::Flex::SpaceAround)
                .split(block);
            (chunks[0], chunks[1])
        };

        let left_infos = vec![
            Line::from(vec![
                Span::from("CPU   "),
                Span::from("  "),
                Span::from(thread::available_parallelism().unwrap().to_string()),
            ]),
            Line::from(vec![
                Span::from("Memory"),
                Span::from("  "),
                Span::from(if let Ok(v) = self.total_memory {
                    format!("{v} MB")
                } else {
                    "-".to_string()
                }),
            ]),
            Line::from(vec![
                Span::from("Arch  "),
                Span::from("  "),
                Span::from(std::env::consts::ARCH),
            ]),
            Line::from(vec![Span::from("KVM   "), Span::from("  "), {
                match self.kvm_version {
                    Ok(v) => Span::from(format!("Enabled. Version {}", v)).green(),
                    Err(_) => Span::from("Disabled or Unavailble").red(),
                }
            }]),
        ];

        frame.render_widget(
            List::new(left_infos),
            left.inner(Margin {
                horizontal: 1,
                vertical: 1,
            }),
        );

        let right_infos = vec![
            Line::from(vec![
                Span::from("x86_64 "),
                Span::from("  "),
                {
                    if self.available_archs.contains(&Arch::X86_64) {
                        Span::from("qemu-system-x86_64    ").green()
                    } else {
                        Span::from("qemu-system-x86_64    ").red()
                    }
                },
                {
                    if self.available_uefi.contains(&Arch::X86_64) {
                        Span::from(" UEFI   ").green()
                    } else {
                        Span::from(" UEFI   ").red()
                    }
                },
                {
                    if self.available_archs.contains(&Arch::X86_64) {
                        Span::from(" BIOS   ").green()
                    } else {
                        Span::from(" BIOS   ").red()
                    }
                },
            ]),
            Line::from(vec![
                Span::from("aarch64"),
                Span::from("  "),
                {
                    if self.available_archs.contains(&Arch::Aarch64) {
                        Span::from("qemu-system-aarch64   ").green()
                    } else {
                        Span::from("qemu-system-aarch64   ").red()
                    }
                },
                {
                    if self.available_uefi.contains(&Arch::Aarch64) {
                        Span::from(" UEFI  ").green()
                    } else {
                        Span::from(" UEFI  ").red()
                    }
                },
            ]),
            Line::from(vec![
                Span::from("riscv64"),
                Span::from("  "),
                {
                    if self.available_archs.contains(&Arch::Riscv64) {
                        Span::from("qemu-system-riscv64   ").green()
                    } else {
                        Span::from("qemu-system-riscv64   ").red()
                    }
                },
                {
                    if self.available_uefi.contains(&Arch::Riscv64) {
                        Span::from(" UEFI  ").green()
                    } else {
                        Span::from(" UEFI  ").red()
                    }
                },
            ]),
        ];

        frame.render_widget(
            List::new(right_infos),
            right.inner(Margin {
                horizontal: 1,
                vertical: 1,
            }),
        );
    }

    pub fn render(&mut self, frame: &mut Frame) {
        let (host_block, main_block, help_block) = {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(6),
                    Constraint::Fill(1),
                    Constraint::Length(3),
                ])
                .margin(1)
                .split(frame.area());
            (chunks[0], chunks[1], chunks[2])
        };

        self.render_host_infos(host_block, frame);

        if !self.vms.is_empty() {
            let (vm_list_block, vm_description_block) = {
                let chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Length(25), Constraint::Fill(1)])
                    .margin(1)
                    .split(main_block);
                (chunks[0], chunks[1])
            };
            frame.render_widget(
                Block::new()
                    .borders(Borders::all())
                    .border_type(BorderType::Rounded)
                    .title(" VMs ")
                    .title_alignment(ratatui::layout::HorizontalAlignment::Center)
                    .border_style(Style::default().yellow()),
                vm_list_block,
            );

            let vm_names = self.vms.iter().map(|vm| {
                ListItem::new(vec![
                    Line::from(""),
                    Line::from(format!("  {}", vm.name)),
                    Line::from(""),
                ])
            });

            let vm_list = List::new(vm_names)
                .highlight_style(Style::new().bg(Color::Yellow).black().bold())
                .highlight_spacing(HighlightSpacing::Always);

            frame.render_stateful_widget(
                vm_list,
                vm_list_block.inner(Margin {
                    horizontal: 1,
                    vertical: 1,
                }),
                &mut self.vm_list_state,
            );

            if let Some(selected_vm_index) = self.vm_list_state.selected()
                && let Some(vm) = self.vms.get_mut(selected_vm_index)
            {
                vm.render(frame, vm_description_block);
            }
        } else {
            let message = Text::from(Line::from(vec![
                Span::from("Press "),
                Span::from("n").bold(),
                Span::from(" to create new VM"),
            ]))
            .centered();

            let block = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Fill(1),
                    Constraint::Length(1),
                    Constraint::Fill(1),
                ])
                .margin(1)
                .split(main_block)[1];

            frame.render_widget(message, block);
            frame.render_widget(
                Block::new()
                    .borders(Borders::all())
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().yellow()),
                main_block,
            );
        }

        if let Some(vm_builder) = &mut self.new_vm {
            vm_builder.render(frame);
        }

        if let Some(delete_confirmation) = &mut self.delete_confirmation {
            delete_confirmation.render(frame);
        }

        if let Some(edit_vm) = &mut self.edit_vm {
            edit_vm.render(frame);
        }

        self.help.render(frame, self, help_block);
    }

    pub fn tick(&mut self) {
        self.notifications.iter_mut().for_each(|n| n.ttl -= 1);
        self.notifications.retain(|n| n.ttl > 0);
    }

    pub fn quit(&mut self) {
        self.running = false;
    }
}
