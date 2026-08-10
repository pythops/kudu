use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::Stylize,
    text::{Line, Span, Text},
    widgets::ListItem,
};

use crossterm::event::{KeyCode, KeyEvent};
use tui_input::{Input, backend::crossterm::EventHandler};

use crate::{Arch, distro::LinuxDistro};

#[derive(Debug, Clone, Default)]
struct UserInputField {
    field: Input,
    error: Option<String>,
}
#[derive(Debug, Default, Clone, PartialEq)]
pub enum Section {
    #[default]
    Cpu,
    Arch,
    Memory,
    Uefi,
}

#[derive(Debug, Clone)]
pub struct Hardware {
    section: Section,
    arch: Arch,
    vcpu: UserInputField,
    memory: UserInputField,
    enable_uefi: bool,
}

impl Hardware {
    pub fn new() -> Self {
        Self {
            section: Section::Cpu,
            arch: Arch::try_from(std::env::consts::ARCH).unwrap_or_default(),
            vcpu: UserInputField {
                field: Input::from("1"),
                error: None,
            },
            memory: UserInputField {
                field: Input::from("512"),
                error: None,
            },
            enable_uefi: true,
        }
    }

    pub fn set_arch(&mut self, arch: Arch) {
        self.arch = arch;
    }

    pub fn set_uefi(&mut self, enable: bool) {
        self.enable_uefi = enable;
    }

    pub fn vcpu(&self) -> u16 {
        self.vcpu.field.value().parse::<u16>().unwrap()
    }

    pub fn memory(&self) -> u32 {
        self.memory.field.value().parse::<u32>().unwrap()
    }

    pub fn arch(&self) -> Arch {
        self.arch
    }

    pub fn enable_uefi(&self) -> bool {
        self.enable_uefi
    }

    pub fn validate(&mut self) -> bool {
        let mut valid = true;

        self.vcpu.error = None;
        self.memory.error = None;

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

    pub fn handle_key_events(&mut self, key_event: KeyEvent, os: Option<LinuxDistro>) {
        match self.section {
            Section::Cpu => match key_event.code {
                KeyCode::Up if self.validate() => {
                    self.section = Section::Uefi;
                }
                KeyCode::Down if self.validate() => {
                    self.section = Section::Memory;
                }
                _ => {
                    self.vcpu
                        .field
                        .handle_event(&crossterm::event::Event::Key(key_event));
                }
            },
            Section::Memory => match key_event.code {
                KeyCode::Up if self.validate() => {
                    self.section = Section::Cpu;
                }
                KeyCode::Down if self.validate() => {
                    self.section = Section::Arch;
                }
                _ => {
                    self.memory
                        .field
                        .handle_event(&crossterm::event::Event::Key(key_event));
                }
            },
            Section::Arch => match os {
                Some(LinuxDistro::TempleOS) | Some(LinuxDistro::ArchLinux) => {
                    match key_event.code {
                        KeyCode::Up | KeyCode::Char('k') => {
                            self.section = Section::Memory;
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            self.section = Section::Uefi;
                        }
                        _ => {}
                    }
                }
                _ => match key_event.code {
                    KeyCode::Right | KeyCode::Char('l') => match self.arch {
                        Arch::X86_64 => {
                            self.arch = Arch::Riscv64;
                            self.enable_uefi = true;
                        }
                        Arch::Aarch64 => {
                            self.arch = Arch::X86_64;
                        }
                        Arch::Riscv64 => {
                            self.arch = Arch::Aarch64;
                            self.enable_uefi = true;
                        }
                    },
                    KeyCode::Left | KeyCode::Char('h') => match self.arch {
                        Arch::X86_64 => {
                            self.arch = Arch::Aarch64;
                            self.enable_uefi = true;
                        }
                        Arch::Aarch64 => {
                            self.arch = Arch::Riscv64;
                            self.enable_uefi = true;
                        }
                        Arch::Riscv64 => {
                            self.arch = Arch::X86_64;
                        }
                    },
                    KeyCode::Up | KeyCode::Char('k') => {
                        self.section = Section::Memory;
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        self.section = Section::Uefi;
                    }
                    _ => {}
                },
            },
            Section::Uefi => {
                if os == Some(LinuxDistro::TempleOS) {
                    match key_event.code {
                        KeyCode::Up | KeyCode::Char('k') => {
                            self.section = Section::Arch;
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            self.section = Section::Cpu;
                        }
                        _ => {}
                    }
                } else {
                    match key_event.code {
                        KeyCode::Right
                        | KeyCode::Char('l')
                        | KeyCode::Left
                        | KeyCode::Char('h')
                            if self.arch == Arch::X86_64 =>
                        {
                            self.enable_uefi = !self.enable_uefi;
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            self.section = Section::Arch;
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            self.section = Section::Cpu;
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    pub fn render(
        &self,
        frame: &mut Frame,
        area: Rect,
        os: Option<LinuxDistro>,
        cancel_confirmation_popup: bool,
    ) {
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
        let arch = Line::from(vec![
            {
                if self.section == Section::Arch {
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
                if self.section == Section::Uefi {
                    Span::from("> Uefi  ").bold()
                } else {
                    Span::from("  Uefi  ")
                }
            },
            Span::from(" ".repeat(6)),
            Span::from({
                if self.arch == Arch::X86_64 {
                    if os == Some(LinuxDistro::TempleOS) {
                        "[x] BIOS"
                    } else if self.enable_uefi {
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
        if !cancel_confirmation_popup {
            match self.section {
                Section::Cpu if self.vcpu.field.visual_cursor() < 50 => {
                    let x = area.x + self.vcpu.field.visual_cursor() as u16 + 16;
                    let y = area.y + 2;
                    frame.set_cursor_position((x, y));
                }
                Section::Memory if self.memory.field.visual_cursor() < 50 => {
                    let x = area.x + self.memory.field.visual_cursor() as u16 + 16;
                    let y = area.y + 6;
                    frame.set_cursor_position((x, y));
                }
                _ => {}
            }
        }
    }

    pub fn summary(&self) -> Vec<ListItem<'_>> {
        vec![
            ListItem::from(vec![
                Line::from(vec![
                    Span::from("Arch").bold(),
                    Span::from(" ".repeat(16)),
                    Span::from(self.arch.to_string()),
                ]),
                Line::from(""),
            ]),
            ListItem::from(vec![
                Line::from(vec![
                    Span::from("vCPU").bold(),
                    Span::from(" ".repeat(16)),
                    Span::from(self.vcpu.field.value()),
                ]),
                Line::from(""),
            ]),
            ListItem::from(vec![
                Line::from(vec![
                    Span::from("Memory").bold(),
                    Span::from(" ".repeat(14)),
                    Span::from(format!("{} MB", self.memory.field.value())),
                ]),
                Line::from(""),
            ]),
            ListItem::from(vec![
                Line::from(vec![
                    Span::from("Firmware").bold(),
                    Span::from(" ".repeat(12)),
                    Span::from(if self.enable_uefi { "UEFI" } else { "BIOS" }),
                ]),
                Line::from(""),
            ]),
        ]
    }
}
