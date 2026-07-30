use anyhow::Result;
use std::{
    mem::discriminant,
    path::PathBuf,
    sync::{Arc, atomic::AtomicBool, mpsc::Sender},
};

use crossterm::event::{KeyCode, KeyEvent};

use qapi::qmp::RunState;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{
        Block, BorderType, Borders, Clear, List, ListItem, ListState, Padding, Row, Table,
        TableState,
    },
};
use tui_input::{Input, backend::crossterm::EventHandler};

use crate::{
    Arch,
    cloudinit::Cloudinit,
    confirmation::cancel::CancelConfirmation,
    disk::{Disk, DiskBuilder},
    distro::{
        LinuxDistro::{self, ArchLinux},
        debian::DebianRelease,
        ubuntu::UbuntuRelease,
    },
    event::Event,
    qemu::Network,
    vm::VM,
};

#[derive(Debug, Clone, Default)]
struct UserInputField {
    field: Input,
    error: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
enum Section {
    Distro(DistroSection),
    Hardware(HardwareSection),
    Disk,
    Network,
    Summary,
}

#[derive(Debug, Default, Clone, PartialEq)]
enum DistroSection {
    #[default]
    Name,
    OS,
    Release,
    Cloudinit,
}

#[derive(Debug, Default, Clone, PartialEq)]
enum HardwareSection {
    #[default]
    Cpu,
    Arch,
    Memory,
    Uefi,
}

#[derive(Debug, Clone)]
pub struct VMBuilder {
    focused_section: Section,
    pub arch: Arch,
    name: UserInputField,
    cloudinit: UserInputField,
    pub distro: LinuxDistro,
    vcpus: UserInputField,
    memory: UserInputField,
    pub network: Network,
    pub confirmation: Option<CancelConfirmation>,
    enable_uefi: bool,
    ubuntu_release: UbuntuRelease,
    debian_release: DebianRelease,
    pub disks: Vec<Disk>,
    distk_state: TableState,
    new_disk: Option<DiskBuilder>,
}

impl Default for VMBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl VMBuilder {
    pub fn new() -> VMBuilder {
        Self {
            focused_section: Section::Distro(DistroSection::Name),
            arch: Arch::try_from(std::env::consts::ARCH).unwrap_or_default(),
            name: UserInputField {
                field: Input::default(),
                error: None,
            },
            distro: LinuxDistro::default(),
            vcpus: UserInputField {
                field: Input::from("1"),
                error: None,
            },
            memory: UserInputField {
                field: Input::from("512"),
                error: None,
            },
            cloudinit: UserInputField {
                field: Input::default(),
                error: None,
            },
            network: Network::default(),
            confirmation: None,
            enable_uefi: true,
            ubuntu_release: UbuntuRelease::default(),
            debian_release: DebianRelease::default(),
            disks: Vec::new(),
            distk_state: TableState::default(),
            new_disk: None,
        }
    }

    pub fn build(&self) -> VM {
        let distro = match self.distro {
            LinuxDistro::Debian(_) => LinuxDistro::Debian(self.debian_release),
            LinuxDistro::Ubuntu(_) => LinuxDistro::Ubuntu(self.ubuntu_release),
            ArchLinux => ArchLinux,
        };

        VM {
            id: uuid::Uuid::new_v4(),
            arch: self.arch,
            name: self.name.field.to_string(),
            vcpus: self.vcpus.field.to_string().parse::<u16>().unwrap(),
            memory: self.memory.field.to_string().parse::<u32>().unwrap(),
            state: RunState::shutdown,
            distro,
            events: Vec::new(),
            events_state: ListState::default(),
            vnc: None,
            cloudinit: Cloudinit::from_path(self.cloudinit.field.value()).ok(),
            enable_uefi: self.enable_uefi,
            uefi: None,
            downloading: Arc::new(AtomicBool::new(false)),
            disks: self.disks.clone(),
        }
    }

    fn validate_distro_section(&mut self) -> bool {
        let mut valid = true;

        self.name.error = None;
        self.cloudinit.error = None;

        if self.name.field.value().is_empty() {
            self.name.error = Some("Field required".into());
            valid = false;
        }

        if !self.cloudinit.field.value().is_empty()
            && !PathBuf::from(self.cloudinit.field.value()).exists()
        {
            self.cloudinit.error = Some("Cloudinit file does not exists".into());
            valid = false;
        }

        valid
    }

    fn validate_harware_section(&mut self) -> bool {
        let mut valid = true;

        self.vcpus.error = None;
        self.memory.error = None;

        if self.vcpus.field.value().is_empty() {
            self.vcpus.error = Some("Field required".into());
            valid = false;
        } else {
            match self.vcpus.field.value().parse::<u16>() {
                Ok(v) => {
                    if v == 0 {
                        self.vcpus.error = Some("vcpu value can not be 0".into());
                        valid = false;
                    }
                }
                Err(_) => {
                    self.vcpus.error = Some("vcpu value should be a number".into());
                    valid = false;
                }
            }
        };

        if self.memory.field.value().is_empty() {
            self.memory.error = Some("Field required".into());
            valid = false;
        } else {
            match self.memory.field.value().parse::<u32>() {
                Ok(v) => {
                    if v == 0 {
                        self.memory.error = Some("Memory value can not be 0".into());
                        valid = false;
                    }
                }
                Err(_) => {
                    self.memory.error = Some("Memory value should be a number".into());
                    valid = false;
                }
            }
        };
        valid
    }

    pub fn handle_key_events(&mut self, key_event: KeyEvent, sender: Sender<Event>) -> Result<()> {
        if self.confirmation.is_some() && key_event.code == KeyCode::Esc {
            self.confirmation = None;
            return Ok(());
        }

        if let Some(confirmation) = &mut self.confirmation {
            confirmation.handle_key_events(key_event, sender)?;
            return Ok(());
        }

        if let Some(new_disk) = &mut self.new_disk {
            match key_event.code {
                KeyCode::Esc => {
                    self.new_disk = None;
                }
                KeyCode::Enter => {
                    if new_disk.validate() {
                        let disk = new_disk.build()?;
                        self.disks.push(disk);
                        self.new_disk = None;

                        if self.distk_state.selected().is_none() {
                            self.distk_state.select(Some(0));
                        }
                    }
                }

                _ => {
                    new_disk.handle_key_events(key_event);
                }
            }

            return Ok(());
        }

        if key_event.code == KeyCode::Esc {
            self.confirmation = Some(CancelConfirmation::default());
            return Ok(());
        }

        match key_event.code {
            KeyCode::Tab => match self.focused_section {
                Section::Distro(_) => {
                    if self.validate_distro_section() {
                        self.focused_section = Section::Hardware(HardwareSection::default());
                    }
                }
                Section::Hardware(_) => {
                    if self.validate_harware_section() {
                        self.focused_section = Section::Disk;
                    }
                }
                Section::Disk => self.focused_section = Section::Network,
                Section::Network => self.focused_section = Section::Summary,
                Section::Summary => {
                    self.focused_section = Section::Distro(DistroSection::default())
                }
            },
            KeyCode::BackTab => match self.focused_section {
                Section::Distro(_) => {
                    if self.validate_distro_section() {
                        self.focused_section = Section::Summary;
                    }
                }
                Section::Hardware(_) => {
                    if self.validate_harware_section() {
                        self.focused_section = Section::Distro(DistroSection::default());
                    }
                }
                Section::Disk => {
                    self.focused_section = Section::Hardware(HardwareSection::default())
                }
                Section::Network => self.focused_section = Section::Disk,
                Section::Summary => self.focused_section = Section::Network,
            },
            _ => match &self.focused_section {
                Section::Distro(distro_section) => match distro_section {
                    DistroSection::Name => match key_event.code {
                        KeyCode::Up if self.validate_distro_section() => {
                            self.focused_section = Section::Distro(DistroSection::Cloudinit);
                        }
                        KeyCode::Down if self.validate_distro_section() => {
                            self.focused_section = Section::Distro(DistroSection::OS);
                        }
                        _ => {
                            self.name
                                .field
                                .handle_event(&crossterm::event::Event::Key(key_event));
                        }
                    },
                    DistroSection::OS => match key_event.code {
                        KeyCode::Up | KeyCode::Char('k') => {
                            self.focused_section = Section::Distro(DistroSection::Name);
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            self.focused_section = Section::Distro(DistroSection::Release);
                        }
                        KeyCode::Left | KeyCode::Char('h') => match self.distro {
                            LinuxDistro::Debian(_) => {
                                self.distro = LinuxDistro::Ubuntu(UbuntuRelease::default());
                            }
                            LinuxDistro::Ubuntu(_) => {
                                self.distro = LinuxDistro::ArchLinux;
                            }
                            LinuxDistro::ArchLinux => {
                                self.distro = LinuxDistro::Debian(DebianRelease::default());
                            }
                        },
                        KeyCode::Right | KeyCode::Char('l') => match self.distro {
                            LinuxDistro::Debian(_) => {
                                self.distro = LinuxDistro::ArchLinux;
                            }
                            LinuxDistro::Ubuntu(_) => {
                                self.distro = LinuxDistro::Debian(DebianRelease::default());
                            }
                            LinuxDistro::ArchLinux => {
                                self.distro = LinuxDistro::Ubuntu(UbuntuRelease::default());
                            }
                        },
                        _ => {}
                    },
                    DistroSection::Release => match key_event.code {
                        KeyCode::Up | KeyCode::Char('k') => {
                            self.focused_section = Section::Distro(DistroSection::OS);
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            self.focused_section = Section::Distro(DistroSection::Cloudinit);
                        }
                        KeyCode::Right | KeyCode::Char('l') => match self.distro {
                            LinuxDistro::ArchLinux => {}
                            LinuxDistro::Debian(_) => match self.debian_release {
                                DebianRelease::Trixie => {
                                    self.debian_release = DebianRelease::Bookworm;
                                }
                                DebianRelease::Bookworm => {
                                    self.debian_release = DebianRelease::Forky;
                                }
                                DebianRelease::Forky => {
                                    self.debian_release = DebianRelease::Trixie;
                                }
                            },
                            LinuxDistro::Ubuntu(_) => match self.ubuntu_release {
                                UbuntuRelease::Resolute => {
                                    self.ubuntu_release = UbuntuRelease::Noble;
                                }
                                UbuntuRelease::Noble => {
                                    self.ubuntu_release = UbuntuRelease::Jammy;
                                }
                                UbuntuRelease::Jammy => {
                                    self.ubuntu_release = UbuntuRelease::Resolute;
                                }
                            },
                        },
                        KeyCode::Left | KeyCode::Char('h') => match self.distro {
                            LinuxDistro::ArchLinux => {}
                            LinuxDistro::Debian(_) => match self.debian_release {
                                DebianRelease::Trixie => {
                                    self.debian_release = DebianRelease::Forky;
                                }
                                DebianRelease::Bookworm => {
                                    self.debian_release = DebianRelease::Trixie;
                                }
                                DebianRelease::Forky => {
                                    self.debian_release = DebianRelease::Bookworm;
                                }
                            },
                            LinuxDistro::Ubuntu(_) => match self.ubuntu_release {
                                UbuntuRelease::Resolute => {
                                    self.ubuntu_release = UbuntuRelease::Jammy;
                                }
                                UbuntuRelease::Noble => {
                                    self.ubuntu_release = UbuntuRelease::Resolute;
                                }
                                UbuntuRelease::Jammy => {
                                    self.ubuntu_release = UbuntuRelease::Noble;
                                }
                            },
                        },
                        _ => {}
                    },
                    DistroSection::Cloudinit => match key_event.code {
                        KeyCode::Up if self.validate_distro_section() => {
                            self.focused_section = Section::Distro(DistroSection::Release);
                        }
                        KeyCode::Down if self.validate_distro_section() => {
                            self.focused_section = Section::Distro(DistroSection::Name);
                        }
                        _ => {
                            self.cloudinit
                                .field
                                .handle_event(&crossterm::event::Event::Key(key_event));
                        }
                    },
                },
                Section::Hardware(hardware_section) => match hardware_section {
                    HardwareSection::Cpu => match key_event.code {
                        KeyCode::Up if self.validate_harware_section() => {
                            self.focused_section = Section::Hardware(HardwareSection::Uefi);
                        }
                        KeyCode::Down if self.validate_harware_section() => {
                            self.focused_section = Section::Hardware(HardwareSection::Memory);
                        }
                        _ => {
                            self.vcpus
                                .field
                                .handle_event(&crossterm::event::Event::Key(key_event));
                        }
                    },
                    HardwareSection::Memory => match key_event.code {
                        KeyCode::Up if self.validate_harware_section() => {
                            self.focused_section = Section::Hardware(HardwareSection::Cpu);
                        }
                        KeyCode::Down if self.validate_harware_section() => {
                            self.focused_section = Section::Hardware(HardwareSection::Arch);
                        }
                        _ => {
                            self.memory
                                .field
                                .handle_event(&crossterm::event::Event::Key(key_event));
                        }
                    },
                    HardwareSection::Arch => match key_event.code {
                        KeyCode::Right | KeyCode::Char('l') => match self.arch {
                            Arch::X86_64 => {
                                self.arch = Arch::Riscv64;
                            }
                            Arch::Aarch64 => {
                                self.arch = Arch::X86_64;
                            }
                            Arch::Riscv64 => {
                                self.arch = Arch::Aarch64;
                            }
                        },
                        KeyCode::Left | KeyCode::Char('h') => match self.arch {
                            Arch::X86_64 => {
                                self.arch = Arch::Aarch64;
                            }
                            Arch::Aarch64 => {
                                self.arch = Arch::Riscv64;
                            }
                            Arch::Riscv64 => {
                                self.arch = Arch::X86_64;
                            }
                        },
                        KeyCode::Up | KeyCode::Char('k') => {
                            self.focused_section = Section::Hardware(HardwareSection::Memory);
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            self.focused_section = Section::Hardware(HardwareSection::Uefi);
                        }
                        _ => {}
                    },
                    HardwareSection::Uefi => match key_event.code {
                        KeyCode::Right
                        | KeyCode::Char('l')
                        | KeyCode::Left
                        | KeyCode::Char('h')
                            if self.arch == Arch::X86_64 =>
                        {
                            self.enable_uefi = !self.enable_uefi;
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            self.focused_section = Section::Hardware(HardwareSection::Arch);
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            self.focused_section = Section::Hardware(HardwareSection::Cpu);
                        }
                        _ => {}
                    },
                },
                Section::Disk => match key_event.code {
                    KeyCode::Down | KeyCode::Char('j') => {
                        if let Some(index) = self.distk_state.selected() {
                            self.distk_state
                                .select(Some(index.saturating_add(1).min(self.disks.len() - 1)));
                        }
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if let Some(index) = self.distk_state.selected() {
                            self.distk_state.select(Some(index.saturating_sub(1)));
                        }
                    }
                    KeyCode::Char('d') => {
                        if let Some(index) = self.distk_state.selected() {
                            self.disks.remove(index);

                            if !self.disks.is_empty() {
                                self.distk_state.select(Some(index.saturating_sub(1)));
                            } else {
                                self.distk_state.select(None);
                            }
                        }
                    }
                    KeyCode::Char('n') => {
                        self.new_disk = Some(DiskBuilder::default());
                    }
                    _ => {}
                },
                Section::Network => {}
                Section::Summary if key_event.code == KeyCode::Enter => {
                    let vm = self.build();
                    sender.send(Event::VMCreated(vm))?;
                }
                _ => {}
            },
        }

        Ok(())
    }

    fn title_span(&self, section: Section) -> Span<'_> {
        let is_focused = discriminant(&self.focused_section) == discriminant(&section);
        match section {
            Section::Distro(_) => {
                if is_focused {
                    Span::styled(
                        "   Distro   ",
                        Style::default().bg(Color::Yellow).fg(Color::Black).bold(),
                    )
                } else {
                    Span::from("   Distro   ").fg(Color::DarkGray)
                }
            }
            Section::Hardware(_) => {
                if is_focused {
                    Span::styled(
                        "  Hardware  ",
                        Style::default().bg(Color::Yellow).fg(Color::Black).bold(),
                    )
                } else {
                    Span::from("  Hardware  ").fg(Color::DarkGray)
                }
            }
            Section::Disk => {
                if is_focused {
                    Span::styled(
                        "    Disk    ",
                        Style::default().bg(Color::Yellow).fg(Color::Black).bold(),
                    )
                } else {
                    Span::from("    Disk    ").fg(Color::DarkGray)
                }
            }
            Section::Network => {
                if is_focused {
                    Span::styled(
                        "  Network   ",
                        Style::default().bg(Color::Yellow).fg(Color::Black).bold(),
                    )
                } else {
                    Span::from("  Network   ").fg(Color::DarkGray)
                }
            }
            Section::Summary => {
                if is_focused {
                    Span::styled(
                        "  Summary   ",
                        Style::default().bg(Color::Yellow).fg(Color::Black).bold(),
                    )
                } else {
                    Span::from("  Summary   ").fg(Color::DarkGray)
                }
            }
        }
    }
    fn render_header(&self, frame: &mut Frame, block: Rect) {
        frame.render_widget(
            Block::default()
                .title({
                    Line::from(vec![
                        self.title_span(Section::Distro(DistroSection::Name)),
                        self.title_span(Section::Hardware(HardwareSection::Cpu)),
                        self.title_span(Section::Disk),
                        self.title_span(Section::Network),
                        self.title_span(Section::Summary),
                    ])
                })
                .title_alignment(Alignment::Center)
                .padding(Padding::top(1)),
            block,
        );
    }

    pub fn render(&mut self, frame: &mut Frame) {
        let area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Fill(1),
                Constraint::Percentage(80),
                Constraint::Fill(1),
            ])
            .margin(1)
            .split(frame.area())[1];

        let area = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Fill(1),
                Constraint::Length(80),
                Constraint::Fill(1),
            ])
            .margin(1)
            .split(area)[1];

        frame.render_widget(Clear, area);

        frame.render_widget(
            Block::new()
                .title(" New VM 󰏖  ")
                .title_alignment(ratatui::layout::HorizontalAlignment::Center)
                .borders(Borders::all())
                .border_type(if self.confirmation.is_some() | self.new_disk.is_some() {
                    BorderType::default()
                } else {
                    BorderType::Thick
                })
                .border_style(Style::default().yellow()),
            area,
        );

        let area = area.inner(Margin {
            horizontal: 5,
            vertical: 2,
        });

        let (section_block, area) = {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(1), Constraint::Fill(1)])
                .flex(ratatui::layout::Flex::SpaceBetween)
                .split(area);

            (chunks[0], chunks[1])
        };

        self.render_header(frame, section_block);

        match &self.focused_section {
            Section::Distro(distro_section) => {
                let (name_block, os_block, release_block, cloudinit_block) = {
                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Length(4),
                            Constraint::Length(4),
                            Constraint::Length(4),
                            Constraint::Length(4),
                        ])
                        .margin(2)
                        .split(area);

                    (chunks[0], chunks[1], chunks[2], chunks[3])
                };

                let name = vec![
                    Line::from(vec![
                        {
                            if distro_section == &DistroSection::Name {
                                Span::from("> Name  ").bold()
                            } else {
                                Span::from("  Name  ")
                            }
                        },
                        Span::from(" ".repeat(6)),
                        Span::from({
                            let original_length = self.name.field.to_string().len();
                            let target_length = 50;

                            self.name
                                .field
                                .to_string()
                                .chars()
                                .chain(std::iter::repeat_n(' ', target_length - original_length))
                                .collect::<String>()
                        })
                        .on_dark_gray(),
                    ]),
                    Line::from(vec![
                        Span::from(" ".repeat(14)),
                        Span::from(self.name.clone().error.unwrap_or("".to_string())).red(),
                    ]),
                ];

                let os = Line::from(vec![
                    {
                        if distro_section == &DistroSection::OS {
                            Span::from("> OS    ").bold()
                        } else {
                            Span::from("  OS    ")
                        }
                    },
                    Span::from(" ".repeat(6)),
                    Span::from(format!("< {} >", self.distro)),
                ]);

                let release = Line::from(vec![
                    {
                        if distro_section == &DistroSection::Release {
                            Span::from("> Release").bold()
                        } else {
                            Span::from("  Release")
                        }
                    },
                    Span::from(" ".repeat(6)),
                    Span::from({
                        match self.distro {
                            LinuxDistro::Ubuntu(_) => format!(
                                "< {} - {} >",
                                self.ubuntu_release,
                                self.ubuntu_release.get_number()
                            ),
                            LinuxDistro::Debian(_) => {
                                format!(
                                    "< {} - {} >",
                                    self.debian_release, self.debian_release as u8,
                                )
                            }
                            LinuxDistro::ArchLinux => "-".to_string(),
                        }
                    }),
                ]);

                let cloudinit = vec![
                    Line::from(vec![
                        {
                            if distro_section == &DistroSection::Cloudinit {
                                Span::from("> Cloudinit ").bold()
                            } else {
                                Span::from("  Cloudinit ")
                            }
                        },
                        Span::from(" ".repeat(2)),
                        Span::from({
                            let original_length: u32 =
                                self.cloudinit.field.to_string().len() as u32;
                            let target_length = 50_u32;

                            self.cloudinit
                                .field
                                .to_string()
                                .chars()
                                .chain(std::iter::repeat_n(
                                    ' ',
                                    target_length
                                        .saturating_sub(original_length)
                                        .try_into()
                                        .unwrap(),
                                ))
                                .collect::<String>()
                        })
                        .on_dark_gray(),
                    ]),
                    Line::from(vec![
                        {
                            if distro_section == &DistroSection::Cloudinit {
                                Span::from("  File Path").bold()
                            } else {
                                Span::from("  File Path")
                            }
                        },
                        Span::from(" ".repeat(3)),
                        Span::from(self.cloudinit.clone().error.unwrap_or("".to_string())).red(),
                    ]),
                ];

                frame.render_widget(Text::from(name), name_block);
                frame.render_widget(os, os_block);
                frame.render_widget(release, release_block);
                frame.render_widget(Text::from(cloudinit), cloudinit_block);

                // FIX: cursor shows on the confirmation popup
                if self.confirmation.is_none() {
                    match distro_section {
                        DistroSection::Name => {
                            let x = area.x + self.name.field.visual_cursor() as u16 + 16;
                            let y = area.y + 2;
                            frame.set_cursor_position((x, y));
                        }
                        DistroSection::Cloudinit => {
                            let x = area.x + self.cloudinit.field.visual_cursor() as u16 + 16;
                            let y = area.y + 14;
                            frame.set_cursor_position((x, y));
                        }
                        _ => {}
                    }
                }
            }
            Section::Hardware(hardware_section) => {
                let (cpu_block, memory_block, arch_block, uefi_block) = {
                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Length(4),
                            Constraint::Length(4),
                            Constraint::Length(4),
                            Constraint::Length(4),
                        ])
                        .margin(2)
                        .split(area);

                    (chunks[0], chunks[1], chunks[2], chunks[3])
                };
                let cpu = vec![
                    Line::from(vec![
                        {
                            if hardware_section == &HardwareSection::Cpu {
                                Span::from("> CPU   ").bold()
                            } else {
                                Span::from("  CPU   ")
                            }
                        },
                        Span::from(" ".repeat(6)),
                        Span::from({
                            let original_length = self.vcpus.field.to_string().len();
                            let target_length = 50;

                            self.vcpus
                                .field
                                .to_string()
                                .chars()
                                .chain(std::iter::repeat_n(' ', target_length - original_length))
                                .collect::<String>()
                        })
                        .on_dark_gray(),
                    ]),
                    Line::from(vec![
                        Span::from(" ".repeat(14)),
                        Span::from(self.vcpus.clone().error.unwrap_or("".to_string())).red(),
                    ]),
                ];

                let memory = vec![
                    Line::from(vec![
                        {
                            if hardware_section == &HardwareSection::Memory {
                                Span::from("> Memory").bold()
                            } else {
                                Span::from("  Memory")
                            }
                        },
                        Span::from(" ".repeat(6)),
                        Span::from({
                            let original_length = self.memory.field.to_string().len();
                            let target_length = 47;

                            self.memory
                                .field
                                .to_string()
                                .chars()
                                .chain(std::iter::repeat_n(' ', target_length - original_length))
                                .collect::<String>()
                        })
                        .on_dark_gray(),
                        Span::from(" MB"),
                    ]),
                    Line::from(vec![
                        Span::from(" ".repeat(14)),
                        Span::from(self.memory.clone().error.unwrap_or("".to_string())).red(),
                    ]),
                ];
                let arch = Line::from(vec![
                    {
                        if hardware_section == &HardwareSection::Arch {
                            Span::from("> Arch  ").bold()
                        } else {
                            Span::from("  Arch  ")
                        }
                    },
                    Span::from(" ".repeat(6)),
                    Span::from(format!("< {} >", self.arch).to_lowercase()),
                ]);

                let uefi = Line::from(vec![
                    {
                        if hardware_section == &HardwareSection::Uefi {
                            Span::from("> Uefi  ").bold()
                        } else {
                            Span::from("  Uefi  ")
                        }
                    },
                    Span::from(" ".repeat(6)),
                    Span::from({
                        if self.arch == Arch::X86_64 {
                            if self.enable_uefi {
                                "[x] UEFI        [ ] BIOS"
                            } else {
                                "[ ] UEFI        [x] BIOS"
                            }
                        } else {
                            "[x] UEFI"
                        }
                    }),
                ]);

                frame.render_widget(Text::from(cpu), cpu_block);
                frame.render_widget(Text::from(memory), memory_block);
                frame.render_widget(Text::from(arch), arch_block);
                frame.render_widget(Text::from(uefi), uefi_block);
                // FIX: cursor shows on the confirmation popup
                if self.confirmation.is_none() {
                    match hardware_section {
                        HardwareSection::Cpu => {
                            let x = area.x + self.vcpus.field.visual_cursor() as u16 + 16;
                            let y = area.y + 2;
                            frame.set_cursor_position((x, y));
                        }
                        HardwareSection::Memory => {
                            let x = area.x + self.memory.field.visual_cursor() as u16 + 16;
                            let y = area.y + 6;
                            frame.set_cursor_position((x, y));
                        }
                        _ => {}
                    }
                }
            }

            Section::Disk => {
                if self.disks.is_empty() {
                    let message = Text::from("Press n to add additional disks").centered();
                    frame.render_widget(
                        message,
                        area.inner(Margin {
                            horizontal: 0,
                            vertical: 3,
                        }),
                    );
                } else {
                    let widths = [
                        Constraint::Length(5),
                        Constraint::Length(15),
                        Constraint::Length(15),
                    ];
                    let disks = self.disks.iter().enumerate().map(|(index, disk)| {
                        Row::new(vec![
                            index.to_string(),
                            disk.format.to_string(),
                            format!("{} GiB", disk.size),
                        ])
                    });

                    let disks = Table::new(disks, widths)
                        .header(
                            Row::new(vec!["", "Format", "Size"])
                                .style(Style::new().bold())
                                .bottom_margin(1),
                        )
                        .flex(ratatui::layout::Flex::SpaceBetween)
                        .row_highlight_style(Style::new().on_dark_gray())
                        .column_spacing(1);

                    frame.render_stateful_widget(
                        disks,
                        area.inner(Margin {
                            horizontal: 2,
                            vertical: 2,
                        }),
                        &mut self.distk_state,
                    );
                }
                if let Some(new_disk) = &self.new_disk {
                    new_disk.render(frame);
                }
            }
            Section::Network => {
                let network = Text::from("Only User Network backend is supported for this version")
                    .centered();
                frame.render_widget(
                    network,
                    area.inner(Margin {
                        horizontal: 0,
                        vertical: 3,
                    }),
                );
            }
            Section::Summary => {
                let (summary_block, create_block) = {
                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([Constraint::Fill(1), Constraint::Length(3)])
                        .margin(4)
                        .split(area);

                    (chunks[0], chunks[1])
                };
                let items = [
                    ListItem::from(vec![
                        Line::from(vec![
                            Span::from("Name          ").bold(),
                            Span::from(" ".repeat(6)),
                            Span::from(self.name.field.value()),
                        ]),
                        Line::from(""),
                    ]),
                    ListItem::from(vec![
                        Line::from(vec![
                            Span::from("Distro        ").bold(),
                            Span::from(" ".repeat(6)),
                            Span::from(self.distro.to_string()),
                        ]),
                        Line::from(""),
                    ]),
                    ListItem::from(vec![
                        Line::from(vec![
                            Span::from("Arch          ").bold(),
                            Span::from(" ".repeat(6)),
                            Span::from(self.arch.to_string()),
                        ]),
                        Line::from(""),
                    ]),
                    ListItem::from(vec![
                        Line::from(vec![
                            Span::from("vCPU          ").bold(),
                            Span::from(" ".repeat(6)),
                            Span::from(self.vcpus.field.value()),
                        ]),
                        Line::from(""),
                    ]),
                    ListItem::from(vec![
                        Line::from(vec![
                            Span::from("Memory        ").bold(),
                            Span::from(" ".repeat(6)),
                            Span::from(format!("{} MB", self.memory.field.value())),
                        ]),
                        Line::from(""),
                    ]),
                    ListItem::from(vec![
                        Line::from(vec![
                            Span::from("Firmware      ").bold(),
                            Span::from(" ".repeat(6)),
                            Span::from(if self.enable_uefi { "UEFI" } else { "BIOS" }),
                        ]),
                        Line::from(""),
                    ]),
                    ListItem::from(vec![
                        Line::from(vec![
                            Span::from("Network       ").bold(),
                            Span::from(" ".repeat(6)),
                            Span::from(self.network.to_string()),
                        ]),
                        Line::from(""),
                    ]),
                    ListItem::from({
                        let mut lines = Vec::new();
                        if self.disks.is_empty() {
                            vec![
                                Line::from(vec![
                                    Span::from("Disks        ").bold(),
                                    Span::from(" ".repeat(6)),
                                    Span::from(" - "),
                                ]),
                                Line::from(""),
                            ]
                        } else {
                            lines.push(Line::from(vec![
                                Span::from("Disks        ").bold(),
                                Span::from(" ".repeat(6)),
                                Span::from(format!(
                                    " Disk 0: size={}GiB, format={}",
                                    self.disks[0].size, self.disks[0].format
                                )),
                            ]));
                            for (index, disk) in self.disks.iter().skip(1).enumerate() {
                                lines.push(Line::from(vec![
                                    Span::from(" ".repeat(20)),
                                    Span::from(format!(
                                        "Disk {}: size={}GiB, format={}",
                                        index + 1,
                                        disk.size,
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
                            Span::from("Cloudinit Path").bold(),
                            Span::from(" ".repeat(6)),
                            Span::from(if self.cloudinit.field.value().is_empty() {
                                "Not Specified"
                            } else {
                                self.cloudinit.field.value()
                            }),
                        ]),
                        Line::from(""),
                    ]),
                ];

                let list = List::new(items);
                let create = Text::from(vec![Line::from(""), Line::from("CREATE"), Line::from("")])
                    .centered()
                    .black()
                    .on_yellow()
                    .bold();

                frame.render_widget(list, summary_block);

                let create_block = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Fill(1),
                        Constraint::Length(20),
                        Constraint::Fill(1),
                    ])
                    .flex(ratatui::layout::Flex::SpaceBetween)
                    .split(create_block)[1];

                frame.render_widget(create, create_block);
            }
        }

        if let Some(confirmation) = &self.confirmation {
            confirmation.render(frame);
        }
    }
}
