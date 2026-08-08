use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent};

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::Stylize,
    text::{Line, Span, Text},
    widgets::ListItem,
};
use tui_input::{Input, backend::crossterm::EventHandler};

use crate::{
    BootOption,
    distro::{LinuxDistro, debian::DebianRelease, ubuntu::UbuntuRelease},
};

#[derive(Debug, Default, Clone, PartialEq)]
enum Section {
    #[default]
    Name,
    BootOption,
    LocalFile,
    OS,
    Release,
    Cloudinit,
}

#[derive(Debug, Clone, Default)]
pub struct UserInputField {
    field: Input,
    error: Option<String>,
}

#[derive(Debug, Default, Clone)]
pub struct Overview {
    section: Section,
    name: UserInputField,
    boot_option: BootOption,
    cloudinit: UserInputField,
    boot_file: UserInputField,
    os: LinuxDistro,
    ubuntu_release: UbuntuRelease,
    debian_release: DebianRelease,
}

impl Overview {
    pub fn new() -> Self {
        Overview::default()
    }

    //TODO: maybe a bool is enough
    pub fn boot_option(&self) -> BootOption {
        self.boot_option
    }

    pub fn name(&self) -> String {
        self.name.field.value().to_string()
    }

    pub fn cloudinit(&self) -> Option<PathBuf> {
        if self.cloudinit.field.value().is_empty() {
            None
        } else {
            Some(PathBuf::from(self.cloudinit.field.value()))
        }
    }
    pub fn boot_file(&self) -> Option<PathBuf> {
        if self.boot_file.field.value().is_empty() {
            None
        } else {
            Some(PathBuf::from(self.boot_file.field.value()))
        }
    }

    pub fn os(&self) -> Option<LinuxDistro> {
        match self.boot_option {
            BootOption::CloudImage => match self.os {
                LinuxDistro::Debian(_) => Some(LinuxDistro::Debian(self.debian_release)),
                LinuxDistro::Ubuntu(_) => Some(LinuxDistro::Ubuntu(self.ubuntu_release)),
                LinuxDistro::ArchLinux => Some(LinuxDistro::ArchLinux),
                LinuxDistro::TempleOS => Some(LinuxDistro::TempleOS),
            },
            BootOption::LocalFile => None,
        }
    }

    pub fn handle_key_events(&mut self, key_event: KeyEvent) {
        match self.section {
            Section::Name => match key_event.code {
                KeyCode::Up => {
                    self.section = Section::Cloudinit;
                }
                KeyCode::Down => {
                    self.section = Section::BootOption;
                }
                _ => {
                    self.name
                        .field
                        .handle_event(&crossterm::event::Event::Key(key_event));
                }
            },
            Section::BootOption => match key_event.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    self.section = Section::Name;
                }
                KeyCode::Down | KeyCode::Char('j') => match self.boot_option {
                    BootOption::CloudImage => {
                        self.section = Section::OS;
                    }
                    BootOption::LocalFile => {
                        self.section = Section::LocalFile;
                    }
                },
                KeyCode::Left | KeyCode::Char('h') | KeyCode::Right | KeyCode::Char('l') => {
                    match self.boot_option {
                        BootOption::CloudImage => {
                            self.boot_option = BootOption::LocalFile;
                        }
                        BootOption::LocalFile => {
                            self.boot_option = BootOption::CloudImage;
                        }
                    }
                }
                _ => {}
            },
            Section::LocalFile => match key_event.code {
                KeyCode::Up => {
                    self.section = Section::BootOption;
                }
                KeyCode::Down => {
                    self.section = Section::Cloudinit;
                }
                _ => {
                    self.boot_file
                        .field
                        .handle_event(&crossterm::event::Event::Key(key_event));
                }
            },
            Section::OS => match key_event.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    self.section = Section::BootOption;
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.section = Section::Release;
                }
                KeyCode::Left | KeyCode::Char('h') => match self.os {
                    LinuxDistro::Debian(_) => {
                        self.os = LinuxDistro::Ubuntu(UbuntuRelease::default());
                    }
                    LinuxDistro::Ubuntu(_) => {
                        self.os = LinuxDistro::ArchLinux;
                    }
                    LinuxDistro::ArchLinux => {
                        self.os = LinuxDistro::TempleOS;
                    }
                    LinuxDistro::TempleOS => {
                        self.os = LinuxDistro::Debian(DebianRelease::default());
                    }
                },
                KeyCode::Right | KeyCode::Char('l') => match self.os {
                    LinuxDistro::Debian(_) => {
                        self.os = LinuxDistro::TempleOS;
                    }
                    LinuxDistro::Ubuntu(_) => {
                        self.os = LinuxDistro::Debian(DebianRelease::default());
                    }
                    LinuxDistro::ArchLinux => {
                        self.os = LinuxDistro::Ubuntu(UbuntuRelease::default());
                    }
                    LinuxDistro::TempleOS => {
                        self.os = LinuxDistro::ArchLinux;
                    }
                },
                _ => {}
            },
            Section::Release => match key_event.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    self.section = Section::OS;
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.section = Section::Cloudinit;
                }
                KeyCode::Right | KeyCode::Char('l') => match self.os {
                    LinuxDistro::ArchLinux | LinuxDistro::TempleOS => {}
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
                KeyCode::Left | KeyCode::Char('h') => match self.os {
                    LinuxDistro::ArchLinux | LinuxDistro::TempleOS => {}
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
            Section::Cloudinit => match key_event.code {
                KeyCode::Up => match self.boot_option {
                    BootOption::CloudImage => {
                        self.section = Section::Release;
                    }
                    BootOption::LocalFile => {
                        self.section = Section::LocalFile;
                    }
                },
                KeyCode::Down => {
                    self.section = Section::Name;
                }
                _ => {
                    self.cloudinit
                        .field
                        .handle_event(&crossterm::event::Event::Key(key_event));
                }
            },
        }
    }

    pub fn validate(&mut self) -> bool {
        let mut valid = true;

        self.name.error = None;
        self.cloudinit.error = None;
        self.boot_file.error = None;

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

        if self.boot_option == BootOption::LocalFile {
            if self.boot_file.field.value().is_empty() {
                self.boot_file.error = Some("Field required".into());
                return false;
            }

            if !PathBuf::from(self.boot_file.field.value()).exists() {
                self.boot_file.error = Some("Boot file does not exists".into());
                valid = false;
            }
        }

        valid
    }

    pub fn summary(&self) -> Vec<ListItem<'_>> {
        let mut items = vec![ListItem::from(vec![
            Line::from(vec![
                Span::from("Name").bold(),
                Span::from(" ".repeat(16)),
                Span::from(self.name()),
            ]),
            Line::from(""),
        ])];

        match self.boot_option {
            BootOption::CloudImage => {
                items.push(ListItem::from(vec![
                    Line::from(vec![
                        Span::from("OS").bold(),
                        Span::from(" ".repeat(18)),
                        Span::from(self.os.to_string()),
                    ]),
                    Line::from(""),
                ]));
            }
            BootOption::LocalFile => {
                items.push(ListItem::from(vec![
                    Line::from(vec![
                        Span::from("Boot file").bold(),
                        Span::from(" ".repeat(11)),
                        Span::from(self.boot_file.field.value()),
                    ]),
                    Line::from(""),
                ]));
            }
        };

        items.push(ListItem::from(vec![
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
        ]));

        items
    }

    pub fn render(&self, frame: &mut Frame, block: Rect, cancel_confirmation_popup: bool) {
        let (name_block, boot_block, os_block, release_block, cloudinit_block) = {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(4),
                    Constraint::Length(4),
                    Constraint::Length(4),
                    Constraint::Length(4),
                    Constraint::Length(4),
                ])
                .margin(2)
                .split(block);

            (chunks[0], chunks[1], chunks[2], chunks[3], chunks[4])
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

        let boot_option = Line::from(vec![
            {
                if self.section == Section::BootOption {
                    Span::from("> Boot   ").bold()
                } else {
                    Span::from("  Boot   ")
                }
            },
            Span::from(" ".repeat(5)),
            Span::from({
                if self.boot_option == BootOption::CloudImage {
                    "[x] CloudImage        [ ] Local File"
                } else {
                    "[ ] CloudImage        [x] Local File"
                }
            }),
        ]);

        let os = Line::from(vec![
            {
                if self.section == Section::OS {
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
                    LinuxDistro::ArchLinux | LinuxDistro::TempleOS => "-".to_string(),
                }
            }),
        ]);

        let boot_file = vec![
            Line::from(vec![
                {
                    if self.section == Section::LocalFile {
                        Span::from("> File Path  ").bold()
                    } else {
                        Span::from("  File Path  ")
                    }
                },
                Span::from(" ".repeat(2)),
                Span::from({
                    let original_length = self.boot_file.field.to_string().len();
                    let target_length = 65_usize;

                    self.boot_file
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
                Span::from(self.boot_file.clone().error.unwrap_or("".to_string())).red(),
            ]),
        ];

        let cloudinit = vec![
            Line::from(vec![
                {
                    if self.section == Section::Cloudinit {
                        Span::from("> Cloudinit  ").bold()
                    } else {
                        Span::from("  Cloudinit  ")
                    }
                },
                Span::from(" "),
                Span::from({
                    let original_length = self.cloudinit.field.to_string().len();
                    let target_length = 65_usize;

                    self.cloudinit
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
                {
                    if self.section == Section::Cloudinit {
                        Span::from("  File Path ").bold()
                    } else {
                        Span::from("  File Path ")
                    }
                },
                Span::from(" ".repeat(2)),
                Span::from(self.cloudinit.clone().error.unwrap_or("".to_string())).red(),
            ]),
        ];

        frame.render_widget(Text::from(name), name_block);
        frame.render_widget(Text::from(boot_option), boot_block);
        match self.boot_option {
            BootOption::CloudImage => {
                frame.render_widget(os, os_block);
                frame.render_widget(release, release_block);
            }
            BootOption::LocalFile => {
                frame.render_widget(Text::from(boot_file), os_block);
            }
        }
        frame.render_widget(Text::from(cloudinit), cloudinit_block);

        // FIX: cursor shows on the confirmation popup
        if !cancel_confirmation_popup {
            match self.section {
                Section::Name if self.name.field.visual_cursor() < 65 => {
                    let x = block.x + self.name.field.visual_cursor() as u16 + 16;
                    let y = block.y + 2;
                    frame.set_cursor_position((x, y));
                }
                Section::LocalFile if self.boot_file.field.visual_cursor() <= 50 => {
                    let x = block.x + self.boot_file.field.visual_cursor() as u16 + 16;
                    let y = block.y + 10;
                    frame.set_cursor_position((x, y));
                }
                Section::Cloudinit if self.cloudinit.field.visual_cursor() <= 50 => {
                    let x = block.x + self.cloudinit.field.visual_cursor() as u16 + 16;
                    let y = block.y + 18;
                    frame.set_cursor_position((x, y));
                }
                _ => {}
            }
        }
    }
}
