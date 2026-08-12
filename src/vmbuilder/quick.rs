use anyhow::Result;
use std::{path::PathBuf, sync::mpsc::Sender};

use tui_input::{Input, backend::crossterm::EventHandler};

use crate::{
    Arch, BootOption,
    cloudinit::Cloudinit,
    distro::{LinuxDistro, debian::DebianRelease, ubuntu::UbuntuRelease},
    event::Event,
    network,
    vmbuilder::VMBuildData,
};
use crossterm::event::{KeyCode, KeyEvent};

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Margin},
    style::{Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Clear},
};

#[derive(Debug, Clone, Default, PartialEq)]
pub enum Section {
    #[default]
    Name,
    Os,
    Release,
    Cpu,
    Memory,
    Username,
    Password,
    Create,
}

#[derive(Debug, Clone, Default)]
pub struct UserInputField {
    field: Input,
    error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Quick {
    section: Section,
    name: UserInputField,
    os: LinuxDistro,
    vcpu: UserInputField,
    memory: UserInputField,
    username: UserInputField,
    password: UserInputField,
    ubuntu_release: UbuntuRelease,
    debian_release: DebianRelease,
}

impl Default for Quick {
    fn default() -> Self {
        Self {
            section: Section::default(),
            name: UserInputField::default(),
            os: LinuxDistro::default(),
            vcpu: UserInputField {
                field: Input::from("1"),
                error: None,
            },
            memory: UserInputField {
                field: Input::from("512"),
                error: None,
            },
            username: UserInputField {
                field: Input::from("kudu"),
                error: None,
            },
            password: UserInputField {
                field: Input::from("kudu"),
                error: None,
            },
            ubuntu_release: UbuntuRelease::default(),
            debian_release: DebianRelease::default(),
        }
    }
}

impl Quick {
    pub fn new() -> Self {
        Quick::default()
    }

    pub fn name(&self) -> String {
        self.name.field.value().to_string()
    }
    pub fn os(&self) -> LinuxDistro {
        self.os
    }

    pub fn vcpu(&self) -> u16 {
        self.vcpu.field.value().parse::<u16>().unwrap()
    }

    pub fn memory(&self) -> u32 {
        self.memory.field.value().parse::<u32>().unwrap()
    }

    pub fn arch(&self) -> Arch {
        Arch::try_from(std::env::consts::ARCH).unwrap_or_default()
    }

    pub fn username(&self) -> String {
        self.username.field.value().to_string()
    }
    pub fn password(&self) -> String {
        self.password.field.value().to_string()
    }

    pub fn cloudinit(&self) -> Result<PathBuf> {
        Cloudinit::create_userdata(self.username(), self.password())
    }

    pub fn handle_key_events(&mut self, key_event: KeyEvent, sender: Sender<Event>) {
        match key_event.code {
            KeyCode::Tab | KeyCode::Down => match self.section {
                Section::Name => {
                    self.section = Section::Os;
                }
                Section::Os => {
                    self.section = Section::Release;
                }
                Section::Release => {
                    self.section = Section::Cpu;
                }
                Section::Cpu => {
                    self.section = Section::Memory;
                }
                Section::Memory => {
                    self.section = Section::Username;
                }
                Section::Username => {
                    self.section = Section::Password;
                }
                Section::Password => {
                    self.section = Section::Create;
                }
                Section::Create => {
                    self.section = Section::Name;
                }
            },
            KeyCode::BackTab | KeyCode::Up => match self.section {
                Section::Name => {
                    self.section = Section::Create;
                }
                Section::Os => {
                    self.section = Section::Name;
                }
                Section::Release => {
                    self.section = Section::Os;
                }
                Section::Cpu => {
                    self.section = Section::Release;
                }
                Section::Memory => {
                    self.section = Section::Cpu;
                }
                Section::Username => {
                    self.section = Section::Memory;
                }
                Section::Password => {
                    self.section = Section::Username;
                }
                Section::Create => {
                    self.section = Section::Password;
                }
            },
            _ => match &self.section {
                Section::Name => {
                    self.name
                        .field
                        .handle_event(&crossterm::event::Event::Key(key_event));
                }
                Section::Os => match key_event.code {
                    KeyCode::Left | KeyCode::Char('h') => match self.os {
                        LinuxDistro::Debian(_) => {
                            self.os = LinuxDistro::Ubuntu(UbuntuRelease::default());
                        }
                        LinuxDistro::Ubuntu(_) => {
                            self.os = LinuxDistro::ArchLinux;
                        }
                        LinuxDistro::ArchLinux => {
                            self.os = LinuxDistro::Debian(DebianRelease::default());
                        }
                        _ => {}
                    },
                    KeyCode::Right | KeyCode::Char('l') => match self.os {
                        LinuxDistro::Debian(_) => {
                            self.os = LinuxDistro::ArchLinux;
                        }
                        LinuxDistro::Ubuntu(_) => {
                            self.os = LinuxDistro::Debian(DebianRelease::default());
                        }
                        LinuxDistro::ArchLinux => {
                            self.os = LinuxDistro::Ubuntu(UbuntuRelease::default());
                        }
                        _ => {}
                    },
                    _ => {}
                },
                Section::Release => match key_event.code {
                    KeyCode::Right | KeyCode::Char('l') => match self.os {
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
                        _ => {}
                    },
                    KeyCode::Left | KeyCode::Char('h') => match self.os {
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
                        _ => {}
                    },
                    _ => {}
                },
                Section::Cpu => {
                    self.vcpu
                        .field
                        .handle_event(&crossterm::event::Event::Key(key_event));
                }
                Section::Memory => {
                    self.memory
                        .field
                        .handle_event(&crossterm::event::Event::Key(key_event));
                }
                Section::Username => {
                    self.username
                        .field
                        .handle_event(&crossterm::event::Event::Key(key_event));
                }
                Section::Password => {
                    self.password
                        .field
                        .handle_event(&crossterm::event::Event::Key(key_event));
                }
                Section::Create => {
                    if KeyCode::Enter == key_event.code && self.validate() {
                        let vm_build_data = self.build();
                        let _ = sender.send(Event::VMCreated(vm_build_data));
                    }
                }
            },
        }
    }

    pub fn build(&self) -> VMBuildData {
        VMBuildData {
            boot_option: BootOption::CloudImage,
            name: self.name(),
            boot_file: None,
            os: Some(self.os()),
            vcpu: self.vcpu(),
            memory: self.memory(),
            arch: self.arch(),
            enable_uefi: true,
            network_backend: network::NetworkBackend::default(),
            disks: Vec::new(),
            port_mappings: Vec::new(),
            cloudinit: Some(self.cloudinit().unwrap()), //FIX:
        }
    }

    pub fn validate(&mut self) -> bool {
        let mut valid = true;

        self.name.error = None;

        if self.name.field.value().is_empty() {
            self.name.error = Some("Field required".into());
            valid = false;
        }

        if self.vcpu.field.value().is_empty() {
            self.vcpu.error = Some("Field required".into());
            valid = false;
        } else {
            match self.vcpu.field.value().parse::<u16>() {
                Ok(v) => {
                    if v == 0 {
                        self.vcpu.error = Some("vcpu value can not be 0".into());
                        valid = false;
                    }
                }
                Err(_) => {
                    self.vcpu.error = Some("vcpu value should be a number".into());
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

    pub fn render(&self, frame: &mut Frame, cancel_popup: bool) {
        let area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Fill(1),
                Constraint::Length(42),
                Constraint::Fill(1),
            ])
            .margin(1)
            .split(frame.area())[1];

        let area = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Fill(1),
                Constraint::Length(130),
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
                .border_type(if cancel_popup {
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

        let area = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Fill(1),
                Constraint::Length(80),
                Constraint::Fill(1),
            ])
            .flex(ratatui::layout::Flex::Center)
            .split(area)[1];

        let (
            name_block,
            os_block,
            release_block,
            cpu_block,
            memory_block,
            username_block,
            password_block,
            create_block,
        ) = {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(4),
                    Constraint::Length(4),
                    Constraint::Length(4),
                    Constraint::Length(4),
                    Constraint::Length(4),
                    Constraint::Length(4),
                    Constraint::Length(4),
                    Constraint::Length(3),
                ])
                .margin(2)
                .split(area);

            (
                chunks[0], chunks[1], chunks[2], chunks[3], chunks[4], chunks[5], chunks[6],
                chunks[7],
            )
        };

        let name = vec![
            Line::from(vec![
                {
                    if self.section == Section::Name {
                        Span::from("> Name   ").bold()
                    } else {
                        Span::from("  Name   ")
                    }
                },
                Span::from(" ".repeat(5)),
                Span::from({
                    let original_length = self.name.field.to_string().len();
                    let target_length = 65_usize;

                    self.name
                        .field
                        .to_string()
                        .chars()
                        .chain(std::iter::repeat_n(
                            ' ',
                            target_length.saturating_sub(original_length),
                        ))
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
                if self.section == Section::Os {
                    Span::from("> OS     ").bold()
                } else {
                    Span::from("  OS     ")
                }
            },
            Span::from(" ".repeat(5)),
            Span::from(format!("< {} >", self.os)),
        ]);

        let release = Line::from(vec![
            {
                if self.section == Section::Release {
                    Span::from("> Release").bold()
                } else {
                    Span::from("  Release")
                }
            },
            Span::from(" ".repeat(5)),
            Span::from({
                match self.os {
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
                    _ => "-".to_string(),
                }
            }),
        ]);

        let cpu = vec![
            Line::from(vec![
                {
                    if self.section == Section::Cpu {
                        Span::from("> vCPU  ").bold()
                    } else {
                        Span::from("  vCPU  ")
                    }
                },
                Span::from(" ".repeat(6)),
                Span::from({
                    let original_length = self.vcpu.field.to_string().len();
                    let target_length = 65_usize;

                    self.vcpu
                        .field
                        .to_string()
                        .chars()
                        .chain(std::iter::repeat_n(
                            ' ',
                            target_length.saturating_sub(original_length),
                        ))
                        .collect::<String>()
                })
                .on_dark_gray(),
            ]),
            Line::from(vec![
                Span::from(" ".repeat(14)),
                Span::from(self.vcpu.clone().error.unwrap_or("".to_string())).red(),
            ]),
        ];

        let memory = vec![
            Line::from(vec![
                {
                    if self.section == Section::Memory {
                        Span::from("> Memory").bold()
                    } else {
                        Span::from("  Memory")
                    }
                },
                Span::from(" ".repeat(6)),
                Span::from({
                    let original_length = self.memory.field.to_string().len();
                    let target_length = 65_usize;

                    self.memory
                        .field
                        .to_string()
                        .chars()
                        .chain(std::iter::repeat_n(
                            ' ',
                            target_length.saturating_sub(original_length),
                        ))
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

        let username = vec![
            Line::from(vec![
                {
                    if self.section == Section::Username {
                        Span::from("> Username").bold()
                    } else {
                        Span::from("  Username")
                    }
                },
                Span::from(" ".repeat(4)),
                Span::from({
                    let original_length = self.username.field.to_string().len();
                    let target_length = 65_usize;

                    self.username
                        .field
                        .to_string()
                        .chars()
                        .chain(std::iter::repeat_n(
                            ' ',
                            target_length.saturating_sub(original_length),
                        ))
                        .collect::<String>()
                })
                .on_dark_gray(),
            ]),
            Line::from(vec![
                Span::from(" ".repeat(14)),
                Span::from(self.username.clone().error.unwrap_or("".to_string())).red(),
            ]),
        ];

        let password = vec![
            Line::from(vec![
                {
                    if self.section == Section::Password {
                        Span::from("> Password").bold()
                    } else {
                        Span::from("  Password")
                    }
                },
                Span::from(" ".repeat(4)),
                Span::from({
                    let original_length = self.password.field.to_string().len();
                    let target_length = 65_usize;

                    self.password
                        .field
                        .to_string()
                        .chars()
                        .chain(std::iter::repeat_n(
                            ' ',
                            target_length.saturating_sub(original_length),
                        ))
                        .collect::<String>()
                })
                .on_dark_gray(),
            ]),
            Line::from(vec![
                Span::from(" ".repeat(14)),
                Span::from(self.password.clone().error.unwrap_or("".to_string())).red(),
            ]),
        ];

        let create_block = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Fill(1),
                Constraint::Length(12),
                Constraint::Fill(1),
            ])
            .flex(ratatui::layout::Flex::SpaceBetween)
            .split(create_block)[1];

        let create = Text::from(vec![Line::from(""), Line::from("CREATE"), Line::from("")])
            .centered()
            .style({
                if self.section == Section::Create {
                    Style::default().black().on_yellow()
                } else {
                    Style::new()
                }
            });

        frame.render_widget(Text::from(name), name_block);
        frame.render_widget(os, os_block);
        frame.render_widget(release, release_block);
        frame.render_widget(Text::from(cpu), cpu_block);
        frame.render_widget(Text::from(memory), memory_block);
        frame.render_widget(Text::from(username), username_block);
        frame.render_widget(Text::from(password), password_block);
        frame.render_widget(create, create_block);

        // FIX: cursor shows on the confirmation popup
        if !cancel_popup {
            match self.section {
                Section::Name if self.name.field.visual_cursor() < 65 => {
                    let x = area.x + self.name.field.visual_cursor() as u16 + 16;
                    let y = area.y + 2;
                    frame.set_cursor_position((x, y));
                }
                Section::Cpu if self.vcpu.field.visual_cursor() < 50 => {
                    let x = area.x + self.vcpu.field.visual_cursor() as u16 + 16;
                    let y = area.y + 14;
                    frame.set_cursor_position((x, y));
                }
                Section::Memory if self.memory.field.visual_cursor() < 50 => {
                    let x = area.x + self.memory.field.visual_cursor() as u16 + 16;
                    let y = area.y + 18;
                    frame.set_cursor_position((x, y));
                }
                Section::Username if self.username.field.visual_cursor() < 50 => {
                    let x = area.x + self.username.field.visual_cursor() as u16 + 16;
                    let y = area.y + 22;
                    frame.set_cursor_position((x, y));
                }
                Section::Password if self.password.field.visual_cursor() < 50 => {
                    let x = area.x + self.password.field.visual_cursor() as u16 + 16;
                    let y = area.y + 26;
                    frame.set_cursor_position((x, y));
                }
                _ => {}
            }
        }
    }

    pub fn help(&self) -> Vec<Line<'static>> {
        vec![Line::from(vec![
            Span::from("↑").bold(),
            Span::from("  Up"),
            Span::from(" | "),
            Span::from("↓").bold(),
            Span::from("  Down"),
            Span::from(" | "),
            Span::from("h,←").bold(),
            Span::from("  Left"),
            Span::from(" | "),
            Span::from("l,→").bold(),
            Span::from("  Right"),
            Span::from(" | "),
            Span::from("⇄").bold(),
            Span::from(" Nav"),
            Span::from(" | "),
            Span::from("Esc").bold(),
            Span::from(" Cancel"),
            Span::from(" | "),
            Span::from("Enter").bold(),
            Span::from(" Confirm"),
        ])]
    }
}
