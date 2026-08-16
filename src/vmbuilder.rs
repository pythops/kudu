mod access;
mod advanced;
mod hardware;
mod network;
mod overview;
mod port;
mod quick;
mod storage;

use anyhow::Result;
use std::{path::PathBuf, sync::mpsc::Sender};

use crossterm::event::{KeyCode, KeyEvent};

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Margin},
    style::{Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Clear},
};

use crate::{
    Arch, BootOption,
    access::RemoteAccess,
    confirmation::cancel::CancelConfirmation,
    event::Event::{self},
    network::{NetworkBackend, PortMapping},
    os::Os::{self},
    storage::Disk,
    vmbuilder::{advanced::Advanced, quick::Quick},
};

#[derive(Debug, Clone, PartialEq)]
enum Section {
    Quick,
    Advanced,
}

#[derive(Debug, Clone)]
pub struct VMBuilder {
    section: Section,
    pub advanced: Option<Advanced>,
    pub quick: Option<Quick>,
    pub cancel_confirmation: Option<CancelConfirmation>,
}

impl Default for VMBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct VMBuildData {
    pub boot_option: BootOption,
    pub arch: Arch,
    pub name: String,
    pub cloudinit: Option<PathBuf>,
    pub boot_file: Option<PathBuf>,
    pub os: Option<Os>,
    pub vcpu: u16,
    pub memory: u32,
    pub network_backend: NetworkBackend,
    pub enable_uefi: bool,
    pub disks: Vec<Disk>,
    pub port_mappings: Vec<PortMapping>,
    pub remote_access: Option<RemoteAccess>,
}

impl VMBuilder {
    pub fn new() -> VMBuilder {
        Self {
            section: Section::Quick,
            quick: None,
            advanced: None,
            cancel_confirmation: None,
        }
    }

    pub fn build(&self) -> VMBuildData {
        match self.section {
            Section::Quick => self.quick.clone().unwrap().build(),
            Section::Advanced => self.advanced.clone().unwrap().build(),
        }
    }

    pub fn handle_key_events(&mut self, key_event: KeyEvent, sender: Sender<Event>) -> Result<()> {
        if self.cancel_confirmation.is_some() && key_event.code == KeyCode::Esc {
            self.cancel_confirmation = None;
            return Ok(());
        }

        if let Some(confirmation) = &mut self.cancel_confirmation {
            confirmation.handle_key_events(key_event, sender)?;
            return Ok(());
        }

        if let Some(advanced) = &mut self.advanced {
            if advanced.storage.new_disk_popup() {
                advanced
                    .storage
                    .handle_key_events(key_event, advanced.hardware.arch());
                return Ok(());
            }

            if advanced.port_fowrwaring.new_mapping_popup() {
                advanced.port_fowrwaring.handle_key_events(key_event);
                return Ok(());
            }
        }

        if key_event.code == KeyCode::Esc {
            if self.advanced.is_some() | self.quick.is_some() {
                self.cancel_confirmation = Some(CancelConfirmation::default());
                return Ok(());
            } else {
                sender.send(Event::CancelVM(true))?;
                return Ok(());
            }
        }

        if let Some(advanced) = &mut self.advanced {
            advanced.handle_key_events(key_event, sender)?;
        } else if let Some(quick) = &mut self.quick {
            quick.handle_key_events(key_event, sender);
        } else {
            match key_event.code {
                KeyCode::Tab
                | KeyCode::BackTab
                | KeyCode::Char('j')
                | KeyCode::Char('k')
                | KeyCode::Down
                | KeyCode::Up => match self.section {
                    Section::Quick => self.section = Section::Advanced,
                    Section::Advanced => self.section = Section::Quick,
                },
                KeyCode::Enter => match self.section {
                    Section::Quick => {
                        self.quick = Some(Quick::new());
                    }
                    Section::Advanced => {
                        self.advanced = Some(Advanced::new());
                    }
                },
                _ => {}
            }
        }

        Ok(())
    }

    pub fn render(&mut self, frame: &mut Frame) {
        if let Some(advanced) = &mut self.advanced {
            advanced.render(frame, self.cancel_confirmation.is_some());
        } else if let Some(quick) = &mut self.quick {
            quick.render(frame, self.cancel_confirmation.is_some());
        } else {
            self.render_choice(frame);
        }

        if let Some(confirmation) = &self.cancel_confirmation {
            confirmation.render(frame);
        }
    }

    fn render_choice(&self, frame: &mut Frame) {
        let area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Fill(1),
                Constraint::Length(12),
                Constraint::Fill(1),
            ])
            .margin(1)
            .split(frame.area())[1];

        let area = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Fill(1),
                Constraint::Length(60),
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
                .border_type(BorderType::Thick)
                .border_style(Style::default().yellow()),
            area,
        );

        let area = area.inner(Margin {
            horizontal: 5,
            vertical: 2,
        });

        let (quick_block, advanced_block) = {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Fill(1),
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Fill(1),
                ])
                .split(area);

            (chunks[1], chunks[2])
        };

        let quick_block = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Fill(1),
                Constraint::Length(22),
                Constraint::Fill(1),
            ])
            .margin(1)
            .split(quick_block)[1];

        let advanced_block = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Fill(1),
                Constraint::Length(22),
                Constraint::Fill(1),
            ])
            .margin(1)
            .split(advanced_block)[1];

        let quick = if self.section == Section::Quick {
            Line::from(">   Quick Setup").bold()
        } else {
            Line::from("    Quick Setup")
        };

        let advanced = if self.section == Section::Advanced {
            Line::from(">   Advanced Options").bold()
        } else {
            Line::from("    Advanced Options")
        };

        frame.render_widget(Text::from(quick), quick_block);
        frame.render_widget(Text::from(advanced), advanced_block);
    }

    pub fn help(&self) -> Vec<Line<'static>> {
        if self.cancel_confirmation.is_some() {
            vec![Line::from(vec![
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
            if let Some(advanced) = &self.advanced {
                advanced.help()
            } else if let Some(quick) = &self.quick {
                quick.help()
            } else {
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
                ])]
            }
        }
    }
}
