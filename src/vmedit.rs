pub mod network;
pub mod port;
pub mod storage;
use anyhow::Result;
use std::{cell::RefCell, collections::HashMap, path::PathBuf, rc::Rc, sync::mpsc::Sender};

use crossterm::event::{KeyCode, KeyEvent};

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, ListState},
};
use tui_input::{Input, backend::crossterm::EventHandler};

use crate::{
    Arch,
    access::{RemoteAccess, vnc::VncBuilder},
    event::Event,
    network::Network,
    storage::{Disk, Drive, Interface},
    vm::{VM, VmId},
};

#[derive(Debug, Clone, Default)]
struct UserInputField {
    field: Input,
    error: Option<String>,
}

#[repr(u8)]
#[derive(Debug, Clone, PartialEq)]
enum Section {
    Hardware(HardwareSection),
    Storage,
    Network,
    PortForwarding,
    RemoteAccess,
}

impl Section {
    pub fn as_usize(&self) -> usize {
        match self {
            Section::Hardware(_) => 0,
            Section::Storage => 1,
            Section::Network => 2,
            Section::PortForwarding => 3,
            Section::RemoteAccess => 4,
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub enum HardwareSection {
    #[default]
    Cpu,
    Memory,
}

#[derive(Debug, Clone)]
pub struct EditVM {
    section: Section,
    section_state: ListState,
    vcpu: UserInputField,
    memory: UserInputField,
    storage: storage::StorageEdit,
    network: network::NetworkEdit,
    port_forwarding: port::PortForwarding,
    vnc: VncBuilder,
    pub vm: VM,
}

#[derive(Debug, Clone)]
pub struct VMEditData {
    pub id: VmId,
    pub deleted_drive_paths: Vec<PathBuf>,
    pub added_disks: Vec<Disk>,
    pub resized_drives: HashMap<PathBuf, String>,
    pub new_vcpu: u16,
    pub new_memory: u32,
    pub networks: Vec<Network>,
    pub remote_access: Option<RemoteAccess>,
}

impl EditVM {
    pub fn new(vm: &VM) -> Self {
        let vnc = if let Some(RemoteAccess::Vnc(vnc)) = &vm.remote_access {
            VncBuilder::new(true, vnc.host, vnc.password.clone())
        } else {
            VncBuilder {
                enabled: false,
                ..Default::default()
            }
        };

        let networks = Rc::new(RefCell::new(vm.networks.clone()));

        let drives: Vec<Drive> = vm
            .drives
            .clone()
            .into_iter()
            .filter(|drive| drive.interface != Interface::Pflash)
            .collect();

        Self {
            section: Section::Hardware(HardwareSection::Cpu),
            section_state: ListState::default().with_selected(Some(0)),
            vcpu: UserInputField {
                field: Input::from(vm.vcpu.to_string()),
                error: None,
            },
            memory: UserInputField {
                field: Input::from(vm.memory.to_string()),
                error: None,
            },
            storage: storage::StorageEdit::new(drives),
            network: network::NetworkEdit::new(networks.clone()),
            port_forwarding: port::PortForwarding::new(networks),
            vnc,
            vm: vm.clone(),
        }
    }

    pub fn new_popup(&self) -> bool {
        self.port_forwarding.new_mapping_popup()
            | self.storage.new_drive_popup()
            | self.network.new_network_popup()
    }

    fn validate_harware_section(&mut self) -> bool {
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

    pub fn validate_remote_access(&mut self) -> bool {
        self.vnc.validate()
    }

    pub fn handle_key_events(
        &mut self,
        key_event: KeyEvent,
        arch: Arch,
        sender: Sender<Event>,
    ) -> Result<()> {
        if self.storage.new_drive_popup() {
            self.storage.handle_key_events(key_event, arch);
            return Ok(());
        }

        if self.network.new_network_popup() {
            self.network.handle_key_events(key_event);
            return Ok(());
        }

        if self.port_forwarding.new_mapping_popup() {
            self.port_forwarding.handle_key_events(key_event);
            return Ok(());
        }

        match key_event.code {
            KeyCode::Enter => {
                let mut networks = self.network.build();
                for (network_id, mapping) in &self.port_forwarding.added_port_mappings {
                    if let Some(network) = networks.iter_mut().find(|n| &n.id == network_id) {
                        network.port_mappings.push(*mapping);
                    }
                }

                for (network_id, mapping) in &self.port_forwarding.deleted_port_mappings {
                    if let Some(network) = networks.iter_mut().find(|n| &n.id == network_id) {
                        network.port_mappings.retain(|m| m != mapping);
                    }
                }

                let _ = sender.send(Event::VMEdited(VMEditData {
                    id: self.vm.id,
                    deleted_drive_paths: self.storage.deleted_drive_paths(),
                    added_disks: self.storage.added_disks(),
                    resized_drives: self.storage.resized_drives(),
                    new_vcpu: self.vcpu.field.value().parse::<u16>().unwrap(),
                    new_memory: self.memory.field.value().parse::<u32>().unwrap(),
                    networks,
                    remote_access: self.vnc.build().map(RemoteAccess::Vnc),
                }));
            }
            KeyCode::Tab => match self.section {
                Section::Hardware(_) => {
                    if self.validate_harware_section() {
                        self.section = Section::Storage;
                    }
                }
                Section::Storage => {
                    self.section = Section::Network;
                }
                Section::Network => {
                    self.port_forwarding.refresh();
                    self.section = Section::PortForwarding;
                }
                Section::PortForwarding => {
                    self.section = Section::RemoteAccess;
                }
                Section::RemoteAccess => {
                    if self.validate_remote_access() {
                        self.section = Section::Hardware(HardwareSection::default())
                    }
                }
            },
            KeyCode::BackTab => match self.section {
                Section::Hardware(_) => {
                    if self.validate_harware_section() {
                        self.section = Section::RemoteAccess;
                    }
                }
                Section::Storage => self.section = Section::Hardware(HardwareSection::default()),
                Section::Network => {
                    self.port_forwarding.refresh();
                    self.section = Section::Storage;
                }
                Section::PortForwarding => {
                    self.section = Section::Network;
                }

                Section::RemoteAccess => {
                    if self.validate_remote_access() {
                        self.section = Section::PortForwarding;
                    }
                }
            },
            _ => match &self.section {
                Section::Hardware(hardware_section) => match hardware_section {
                    HardwareSection::Cpu => match key_event.code {
                        KeyCode::Up | KeyCode::Char('k') if self.validate_harware_section() => {
                            self.section = Section::Hardware(HardwareSection::Memory);
                        }
                        KeyCode::Down | KeyCode::Char('j') if self.validate_harware_section() => {
                            self.section = Section::Hardware(HardwareSection::Memory);
                        }
                        _ => {
                            self.vcpu
                                .field
                                .handle_event(&crossterm::event::Event::Key(key_event));
                        }
                    },
                    HardwareSection::Memory => match key_event.code {
                        KeyCode::Up | KeyCode::Char('k') if self.validate_harware_section() => {
                            self.section = Section::Hardware(HardwareSection::Cpu);
                        }
                        KeyCode::Down | KeyCode::Char('j') if self.validate_harware_section() => {
                            self.section = Section::Hardware(HardwareSection::Cpu);
                        }
                        _ => {
                            self.memory
                                .field
                                .handle_event(&crossterm::event::Event::Key(key_event));
                        }
                    },
                },
                Section::Storage => {
                    self.storage.handle_key_events(key_event, arch);
                }
                Section::Network => {
                    self.network.handle_key_events(key_event);
                }
                Section::PortForwarding => {
                    self.port_forwarding.handle_key_events(key_event);
                }
                Section::RemoteAccess => {
                    self.vnc.handle_key_events(key_event);
                }
            },
        }

        Ok(())
    }

    fn render_header(&mut self, frame: &mut Frame, block: Rect) {
        self.section_state.select(Some(self.section.as_usize()));

        let sections = vec![
            ListItem::new(vec![
                Line::from(""),
                Line::from(" Hardware   "),
                Line::from(""),
            ]),
            ListItem::new(vec![
                Line::from(""),
                Line::from(" Storage 󱛟  "),
                Line::from(""),
            ]),
            ListItem::new(vec![
                Line::from(""),
                Line::from(" Network 󰛳  "),
                Line::from(""),
            ]),
            ListItem::new(vec![
                Line::from(""),
                Line::from(" Port Forwarding   "),
                Line::from(""),
            ]),
            ListItem::new(vec![
                Line::from(""),
                Line::from(" Remote Access 󰢹  "),
                Line::from(""),
            ]),
        ];

        let list = List::new(sections)
            .highlight_style(Style::default().bg(Color::Yellow).fg(Color::Black).bold());

        frame.render_stateful_widget(list, block, &mut self.section_state);
    }

    pub fn render(&mut self, frame: &mut Frame) {
        let area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Fill(1),
                Constraint::Length(24),
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
            Block::default()
                .borders(Borders::ALL)
                .title(" Edit VM 󰏖  ")
                .title_alignment(Alignment::Center)
                .border_type(BorderType::Thick)
                .border_type(if self.new_popup() {
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
                .direction(Direction::Horizontal)
                .constraints([Constraint::Length(20), Constraint::Fill(1)])
                .flex(ratatui::layout::Flex::SpaceBetween)
                .split(area);

            (chunks[0], chunks[1])
        };

        self.render_header(frame, section_block);

        let area = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Fill(1),
                Constraint::Length(80),
                Constraint::Fill(1),
            ])
            .flex(ratatui::layout::Flex::Center)
            .split(area)[1];

        let area = area.inner(Margin {
            horizontal: 0,
            vertical: 1,
        });

        match &self.section {
            Section::Hardware(hardware_section) => {
                let (cpu_block, memory_block) = {
                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([Constraint::Length(4), Constraint::Length(4)])
                        .split(area);

                    (chunks[0], chunks[1])
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
                            let original_length = self.vcpu.field.to_string().len();
                            let target_length = 60_usize;

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
                            if hardware_section == &HardwareSection::Memory {
                                Span::from("> Memory").bold()
                            } else {
                                Span::from("  Memory")
                            }
                        },
                        Span::from(" ".repeat(6)),
                        Span::from({
                            let original_length = self.memory.field.to_string().len();
                            let target_length = 60_usize;

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
                frame.render_widget(Text::from(cpu), cpu_block);
                frame.render_widget(Text::from(memory), memory_block);

                match hardware_section {
                    HardwareSection::Cpu if self.vcpu.field.visual_cursor() < 50 => {
                        let x = area.x + self.vcpu.field.visual_cursor() as u16 + 14;
                        let y = area.y;
                        frame.set_cursor_position((x, y));
                    }
                    HardwareSection::Memory if self.memory.field.visual_cursor() < 50 => {
                        let x = area.x + self.memory.field.visual_cursor() as u16 + 14;
                        let y = area.y + 4;
                        frame.set_cursor_position((x, y));
                    }
                    _ => {}
                }
            }
            Section::Storage => {
                self.storage.render(frame, area);
            }
            Section::Network => {
                self.network.render(frame, area);
            }
            Section::PortForwarding => {
                self.port_forwarding.render(frame, area);
            }
            Section::RemoteAccess => {
                self.vnc.render(frame, area, false);
            }
        }
    }

    pub fn help(&self, block_width: u16) -> Vec<Line<'static>> {
        match self.section {
            Section::Hardware(_) => {
                vec![Line::from(vec![
                    Span::from("k,↑").bold(),
                    Span::from("  Up"),
                    Span::from(" | "),
                    Span::from("j,↓").bold(),
                    Span::from("  Down"),
                    Span::from(" | "),
                    Span::from("Esc").bold(),
                    Span::from(" Cancel"),
                    Span::from(" | "),
                    Span::from("Enter").bold(),
                    Span::from(" Confirm"),
                    Span::from(" | "),
                    Span::from("⇄").bold(),
                    Span::from(" Nav"),
                ])]
            }
            Section::Storage if self.storage.new_drive_popup() => self.storage.help(),
            _ => {
                if self.new_popup() {
                    vec![Line::from(vec![
                        Span::from("k,↑").bold(),
                        Span::from("  Up"),
                        Span::from(" | "),
                        Span::from("j,↓").bold(),
                        Span::from("  Down"),
                        Span::from(" | "),
                        Span::from("h,←").bold(),
                        Span::from("  Left"),
                        Span::from(" | "),
                        Span::from("l,→").bold(),
                        Span::from("  Right"),
                        Span::from(" | "),
                        Span::from("Esc").bold(),
                        Span::from(" Cancel"),
                        Span::from(" | "),
                        Span::from("Enter").bold(),
                        Span::from(" Confirm"),
                    ])]
                } else {
                    if block_width >= 113 {
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
                            Span::from("n").bold(),
                            Span::from("  Add"),
                            Span::from(" | "),
                            Span::from("d").bold(),
                            Span::from("  Delete"),
                            Span::from(" | "),
                            Span::from("r").bold(),
                            Span::from("  Resize"),
                            Span::from(" | "),
                            Span::from("u").bold(),
                            Span::from("  Undo"),
                            Span::from(" | "),
                            Span::from("Esc").bold(),
                            Span::from(" Cancel"),
                            Span::from(" | "),
                            Span::from("Enter").bold(),
                            Span::from(" Confirm"),
                            Span::from(" | "),
                            Span::from("⇄").bold(),
                            Span::from(" Nav"),
                        ])]
                    } else {
                        vec![
                            Line::from(vec![
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
                            ]),
                            Line::from(vec![
                                Span::from("n").bold(),
                                Span::from("  Add"),
                                Span::from(" | "),
                                Span::from("d").bold(),
                                Span::from("  Delete"),
                                Span::from(" | "),
                                Span::from("r").bold(),
                                Span::from("  Resize"),
                                Span::from(" | "),
                                Span::from("u").bold(),
                                Span::from("  Undo"),
                                Span::from(" | "),
                                Span::from("Esc").bold(),
                                Span::from(" Cancel"),
                                Span::from(" | "),
                                Span::from("Enter").bold(),
                                Span::from(" Confirm"),
                                Span::from(" | "),
                                Span::from("⇄").bold(),
                                Span::from(" Nav"),
                            ]),
                        ]
                    }
                }
            }
        }
    }
}
