use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent};

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::Stylize,
    text::{Line, Span, Text},
    widgets::ListItem,
};
use tui_input::{Input, backend::crossterm::EventHandler};

use crate::{
    BootOption,
    os::{Os, debian::DebianRelease, freebsd::FreebsdRelease, ubuntu::UbuntuRelease},
    storage::{self, Interface},
};

#[derive(Debug, Default, Clone, PartialEq)]
enum Section {
    #[default]
    Name,
    BootOption,
    LocalFile,
    StorageInterface,
    OS,
    Release,
    Kernel,
    Initrd,
    AppendOptions,
    Cloudinit,
}

#[derive(Debug, Clone, Default)]
pub struct UserInputField {
    field: Input,
    error: Option<String>,
}

#[derive(Debug, Default, Clone, PartialEq)]
enum BootOptionChoice {
    #[default]
    CloudImage,
    LocalFile,
    DirectLinuxBoot,
}

#[derive(Debug, Default, Clone)]
pub struct Overview {
    section: Section,
    name: UserInputField,
    boot_option_choice: BootOptionChoice,
    cloudinit: UserInputField,
    boot_file: UserInputField,
    local_file_interface: storage::Interface,
    os: Os,
    ubuntu_release: UbuntuRelease,
    debian_release: DebianRelease,
    freebsd_release: FreebsdRelease,
    kernel: UserInputField,
    initrd: UserInputField,
    appendoptions: UserInputField,
}

impl Overview {
    pub fn new() -> Self {
        Overview::default()
    }

    pub fn boot_option(&self) -> BootOption {
        match self.boot_option_choice {
            BootOptionChoice::CloudImage => BootOption::CloudImage(self.os()),
            BootOptionChoice::LocalFile => BootOption::LocalFile,
            BootOptionChoice::DirectLinuxBoot => BootOption::DirectLinuxBoot((
                self.kernel.field.value().into(),
                {
                    let v = self.initrd.field.value();
                    if v.is_empty() { None } else { Some(v.into()) }
                },
                {
                    let v = self.appendoptions.field.value();
                    if v.is_empty() { None } else { Some(v.into()) }
                },
            )),
        }
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
    pub fn boot_file(&self) -> Option<(PathBuf, Interface)> {
        if self.boot_file.field.value().is_empty() {
            None
        } else {
            Some((
                PathBuf::from(self.boot_file.field.value()),
                self.local_file_interface,
            ))
        }
    }

    pub fn os(&self) -> Os {
        match self.os {
            Os::Debian(_) => Os::Debian(self.debian_release),
            Os::Ubuntu(_) => Os::Ubuntu(self.ubuntu_release),
            Os::ArchLinux => Os::ArchLinux,
            Os::Freebsd(_) => Os::Freebsd(self.freebsd_release),
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
                KeyCode::Down | KeyCode::Char('j') => match self.boot_option_choice {
                    BootOptionChoice::CloudImage => {
                        self.section = Section::OS;
                    }
                    BootOptionChoice::LocalFile => {
                        self.section = Section::LocalFile;
                    }
                    BootOptionChoice::DirectLinuxBoot => {
                        self.section = Section::Kernel;
                    }
                },
                KeyCode::Right | KeyCode::Char('l') => match self.boot_option_choice {
                    BootOptionChoice::CloudImage => {
                        self.boot_option_choice = BootOptionChoice::LocalFile;
                    }
                    BootOptionChoice::LocalFile => {
                        self.boot_option_choice = BootOptionChoice::DirectLinuxBoot;
                    }
                    BootOptionChoice::DirectLinuxBoot => {
                        self.boot_option_choice = BootOptionChoice::CloudImage;
                    }
                },
                KeyCode::Left | KeyCode::Char('h') => match self.boot_option_choice {
                    BootOptionChoice::CloudImage => {
                        self.boot_option_choice = BootOptionChoice::DirectLinuxBoot;
                    }
                    BootOptionChoice::LocalFile => {
                        self.boot_option_choice = BootOptionChoice::CloudImage;
                    }
                    BootOptionChoice::DirectLinuxBoot => {
                        self.boot_option_choice = BootOptionChoice::LocalFile;
                    }
                },
                _ => {}
            },
            Section::LocalFile => match key_event.code {
                KeyCode::Up => {
                    self.section = Section::BootOption;
                }
                KeyCode::Down => {
                    self.section = Section::StorageInterface;
                }
                _ => {
                    self.boot_file
                        .field
                        .handle_event(&crossterm::event::Event::Key(key_event));
                }
            },
            Section::StorageInterface => match key_event.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    self.section = Section::LocalFile;
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.section = Section::Cloudinit;
                }
                KeyCode::Left | KeyCode::Char('h') | KeyCode::Right | KeyCode::Char('l') => {
                    if self.local_file_interface == Interface::Virtio {
                        self.local_file_interface = Interface::Ide;
                    } else {
                        self.local_file_interface = Interface::Virtio;
                    }
                }
                _ => {}
            },
            Section::OS => match key_event.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    self.section = Section::BootOption;
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.section = Section::Release;
                }
                KeyCode::Right | KeyCode::Char('l') => match self.os {
                    Os::Debian(_) => {
                        self.os = Os::Ubuntu(UbuntuRelease::default());
                    }
                    Os::Ubuntu(_) => {
                        self.os = Os::ArchLinux;
                    }
                    Os::ArchLinux => {
                        self.os = Os::Freebsd(FreebsdRelease::default());
                    }
                    Os::Freebsd(_) => {
                        self.os = Os::Debian(DebianRelease::default());
                    }
                },
                KeyCode::Left | KeyCode::Char('h') => match self.os {
                    Os::Debian(_) => {
                        self.os = Os::Freebsd(FreebsdRelease::default());
                    }
                    Os::Ubuntu(_) => {
                        self.os = Os::ArchLinux;
                    }
                    Os::ArchLinux => {
                        self.os = Os::Ubuntu(UbuntuRelease::default());
                    }
                    Os::Freebsd(_) => {
                        self.os = Os::ArchLinux;
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
                    Os::ArchLinux => {}
                    Os::Debian(_) => match self.debian_release {
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
                    Os::Ubuntu(_) => match self.ubuntu_release {
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
                    Os::Freebsd(_) => match self.freebsd_release {
                        FreebsdRelease::V14 => {
                            self.freebsd_release = FreebsdRelease::V15;
                        }
                        FreebsdRelease::V15 => {
                            self.freebsd_release = FreebsdRelease::V16;
                        }
                        FreebsdRelease::V16 => {
                            self.freebsd_release = FreebsdRelease::V14;
                        }
                    },
                },
                KeyCode::Left | KeyCode::Char('h') => match self.os {
                    Os::ArchLinux => {}
                    Os::Debian(_) => match self.debian_release {
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
                    Os::Ubuntu(_) => match self.ubuntu_release {
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
                    Os::Freebsd(_) => match self.freebsd_release {
                        FreebsdRelease::V14 => {
                            self.freebsd_release = FreebsdRelease::V16;
                        }
                        FreebsdRelease::V15 => {
                            self.freebsd_release = FreebsdRelease::V14;
                        }
                        FreebsdRelease::V16 => {
                            self.freebsd_release = FreebsdRelease::V15;
                        }
                    },
                },
                _ => {}
            },
            Section::Kernel => match key_event.code {
                KeyCode::Up => {
                    self.section = Section::BootOption;
                }
                KeyCode::Down => {
                    self.section = Section::Initrd;
                }
                _ => {
                    self.kernel
                        .field
                        .handle_event(&crossterm::event::Event::Key(key_event));
                }
            },
            Section::Initrd => match key_event.code {
                KeyCode::Up => {
                    self.section = Section::Kernel;
                }
                KeyCode::Down => {
                    self.section = Section::AppendOptions;
                }
                _ => {
                    self.initrd
                        .field
                        .handle_event(&crossterm::event::Event::Key(key_event));
                }
            },
            Section::AppendOptions => match key_event.code {
                KeyCode::Up => {
                    self.section = Section::Initrd;
                }
                KeyCode::Down => {
                    self.section = Section::Cloudinit;
                }
                _ => {
                    self.appendoptions
                        .field
                        .handle_event(&crossterm::event::Event::Key(key_event));
                }
            },
            Section::Cloudinit => match key_event.code {
                KeyCode::Up => match self.boot_option_choice {
                    BootOptionChoice::CloudImage => {
                        self.section = Section::Release;
                    }
                    BootOptionChoice::LocalFile => {
                        self.section = Section::StorageInterface;
                    }
                    BootOptionChoice::DirectLinuxBoot => {
                        self.section = Section::AppendOptions;
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

    //TODO: add validate by field
    pub fn validate(&mut self) -> bool {
        let mut valid = true;

        self.name.error = None;
        self.cloudinit.error = None;
        self.boot_file.error = None;
        self.kernel.error = None;
        self.initrd.error = None;
        self.appendoptions.error = None;

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

        match self.boot_option_choice {
            BootOptionChoice::LocalFile => {
                if self.boot_file.field.value().is_empty() {
                    self.boot_file.error = Some("Field required".into());
                    return false;
                }

                if !PathBuf::from(self.boot_file.field.value()).exists() {
                    self.boot_file.error = Some("Boot file does not exists".into());
                    valid = false;
                }
            }
            BootOptionChoice::DirectLinuxBoot => {
                let kernel = self.kernel.field.value();
                let initrd = self.initrd.field.value();

                if kernel.is_empty() {
                    self.kernel.error = Some("Field required".into());
                    valid = false;
                } else {
                    if !PathBuf::from(kernel).exists() {
                        self.kernel.error = Some("Unvalid path".into());
                        valid = false;
                    }
                }
                if !initrd.is_empty() && !PathBuf::from(initrd).exists() {
                    self.initrd.error = Some("Unvalid path".into());
                    valid = false;
                }
            }
            _ => {}
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

        match self.boot_option_choice {
            BootOptionChoice::CloudImage => {
                items.push(ListItem::from(vec![
                    Line::from(vec![
                        Span::from("OS").bold(),
                        Span::from(" ".repeat(18)),
                        Span::from(self.os.to_string()),
                    ]),
                    Line::from(""),
                ]));
            }
            BootOptionChoice::LocalFile => {
                items.push(ListItem::from(vec![
                    Line::from(vec![
                        Span::from("Boot file").bold(),
                        Span::from(" ".repeat(11)),
                        Span::from(self.boot_file.field.value()),
                    ]),
                    Line::from(""),
                ]));
            }
            BootOptionChoice::DirectLinuxBoot => {
                //TODO:
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

    pub fn render(&self, frame: &mut Frame, area: Rect, cancel_confirmation_popup: bool) {
        let area = area.inner(Margin {
            horizontal: 0,
            vertical: 1,
        });

        let (
            name_block,
            boot_block,
            os_block,
            release_block,
            file_path_block,
            storage_interface_block,
            kernel_block,
            initrd_block,
            append_options_block,
            cloudinit_block,
        ) = {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(4), // Name
                    Constraint::Length(4), // Boot Option
                    Constraint::Length(
                        if self.boot_option_choice == BootOptionChoice::CloudImage {
                            4
                        } else {
                            0
                        },
                    ), // os
                    Constraint::Length(
                        if self.boot_option_choice == BootOptionChoice::CloudImage {
                            4
                        } else {
                            0
                        },
                    ), // release
                    Constraint::Length(if self.boot_option_choice == BootOptionChoice::LocalFile {
                        4
                    } else {
                        0
                    }), // File
                    Constraint::Length(if self.boot_option_choice == BootOptionChoice::LocalFile {
                        4
                    } else {
                        0
                    }), // Interface
                    Constraint::Length(
                        if self.boot_option_choice == BootOptionChoice::DirectLinuxBoot {
                            4
                        } else {
                            0
                        },
                    ), // Kernel
                    Constraint::Length(
                        if self.boot_option_choice == BootOptionChoice::DirectLinuxBoot {
                            4
                        } else {
                            0
                        },
                    ), // Initramfs
                    Constraint::Length(
                        if self.boot_option_choice == BootOptionChoice::DirectLinuxBoot {
                            4
                        } else {
                            0
                        },
                    ), // Append
                    Constraint::Length(4), // Cloudinit
                ])
                .split(area);

            (
                chunks[0], chunks[1], chunks[2], chunks[3], chunks[4], chunks[5], chunks[6],
                chunks[7], chunks[8], chunks[9],
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
                match self.boot_option_choice {
                    BootOptionChoice::CloudImage => {
                        "[x] CloudImage          [ ] Local File          [ ] Direct Boot "
                    }
                    BootOptionChoice::LocalFile => {
                        "[ ] CloudImage          [x] Local File          [ ] Direct Boot "
                    }
                    BootOptionChoice::DirectLinuxBoot => {
                        "[ ] CloudImage          [ ] Local File          [x] Direct Boot "
                    }
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
                    Os::Ubuntu(_) => format!(
                        "< {} - {} >",
                        self.ubuntu_release,
                        self.ubuntu_release.get_version_number()
                    ),
                    Os::Debian(_) => {
                        format!(
                            "< {} - {} >",
                            self.debian_release,
                            self.debian_release.get_version_number(),
                        )
                    }
                    Os::Freebsd(_) => {
                        format!("< {} >", self.freebsd_release,)
                    }
                    Os::ArchLinux => "-".to_string(),
                }
            }),
        ]);

        let boot_file = vec![
            Line::from(vec![
                {
                    if self.section == Section::LocalFile {
                        Span::from("> File Path ").bold()
                    } else {
                        Span::from("  File Path ")
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

        let local_file_interface = Line::from(vec![
            {
                if self.section == Section::StorageInterface {
                    Span::from("> Interface ").bold()
                } else {
                    Span::from("  Interface ")
                }
            },
            Span::from(" ".repeat(2)),
            Span::from(format!("< {} >", self.local_file_interface)),
        ]);

        let kernel = vec![
            Line::from(vec![
                {
                    if self.section == Section::Kernel {
                        Span::from("> Kernel     ").bold()
                    } else {
                        Span::from("  Kernel     ")
                    }
                },
                Span::from(" "),
                Span::from({
                    let original_length = self.kernel.field.to_string().len();
                    let target_length = 65_usize;

                    self.kernel
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
                Span::from(self.kernel.clone().error.unwrap_or("".to_string())).red(),
            ]),
        ];

        let initrd = vec![
            Line::from(vec![
                {
                    if self.section == Section::Initrd {
                        Span::from("> Initrd  ").bold()
                    } else {
                        Span::from("  Initrd  ")
                    }
                },
                Span::from(" ".repeat(4)),
                Span::from({
                    let original_length = self.initrd.field.to_string().len();
                    let target_length = 65_usize;

                    self.initrd
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
                Span::from(self.initrd.clone().error.unwrap_or("".to_string())).red(),
            ]),
        ];

        let append_options = vec![
            Line::from(vec![
                {
                    if self.section == Section::AppendOptions {
                        Span::from("> Append     ").bold()
                    } else {
                        Span::from("  Append     ")
                    }
                },
                Span::from(" "),
                Span::from({
                    let original_length = self.appendoptions.field.to_string().len();
                    let target_length = 65_usize;

                    self.appendoptions
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
                Span::from(self.appendoptions.clone().error.unwrap_or("".to_string())).red(),
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
        match self.boot_option_choice {
            BootOptionChoice::CloudImage => {
                frame.render_widget(os, os_block);
                frame.render_widget(release, release_block);
            }
            BootOptionChoice::LocalFile => {
                frame.render_widget(Text::from(boot_file), file_path_block);
                frame.render_widget(Text::from(local_file_interface), storage_interface_block);
            }
            BootOptionChoice::DirectLinuxBoot => {
                frame.render_widget(Text::from(kernel), kernel_block);
                frame.render_widget(Text::from(initrd), initrd_block);
                frame.render_widget(Text::from(append_options), append_options_block);
            }
        }
        frame.render_widget(Text::from(cloudinit), cloudinit_block);

        // FIX: cursor shows on the confirmation popup
        if !cancel_confirmation_popup {
            match self.section {
                Section::Name if self.name.field.visual_cursor() < 61 => {
                    let x = area.x + self.name.field.visual_cursor() as u16 + 14;
                    let y = area.y;
                    frame.set_cursor_position((x, y));
                }
                Section::LocalFile if self.boot_file.field.visual_cursor() <= 61 => {
                    let x = area.x + self.boot_file.field.visual_cursor() as u16 + 14;
                    let y = area.y + 8;
                    frame.set_cursor_position((x, y));
                }
                Section::Kernel if self.kernel.field.visual_cursor() <= 61 => {
                    let x = area.x + self.kernel.field.visual_cursor() as u16 + 14;
                    let y = area.y + 8;
                    frame.set_cursor_position((x, y));
                }
                Section::Initrd if self.initrd.field.visual_cursor() <= 61 => {
                    let x = area.x + self.initrd.field.visual_cursor() as u16 + 14;
                    let y = area.y + 12;
                    frame.set_cursor_position((x, y));
                }
                Section::AppendOptions if self.appendoptions.field.visual_cursor() <= 61 => {
                    let x = area.x + self.appendoptions.field.visual_cursor() as u16 + 14;
                    let y = area.y + 16;
                    frame.set_cursor_position((x, y));
                }
                Section::Cloudinit if self.cloudinit.field.visual_cursor() <= 61 => {
                    if self.boot_option_choice == BootOptionChoice::DirectLinuxBoot {
                        let x = area.x + self.cloudinit.field.visual_cursor() as u16 + 14;
                        let y = area.y + 20;
                        frame.set_cursor_position((x, y));
                    } else {
                        let x = area.x + self.cloudinit.field.visual_cursor() as u16 + 14;
                        let y = area.y + 16;
                        frame.set_cursor_position((x, y));
                    }
                }
                _ => {}
            }
        }
    }
}
