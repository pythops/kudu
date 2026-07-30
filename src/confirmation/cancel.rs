use crossterm::event::{
    KeyCode::{self},
    KeyEvent,
};
use std::sync::mpsc::Sender;

use anyhow::Result;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Style, Stylize},
    text::Text,
    widgets::{Block, BorderType, Borders, Clear},
};

use crate::event::Event;

#[derive(Debug, Clone, Default)]
pub struct CancelConfirmation {
    choice: bool,
}

impl CancelConfirmation {
    pub fn handle_key_events(&mut self, key_event: KeyEvent, sender: Sender<Event>) -> Result<()> {
        match key_event.code {
            KeyCode::Left | KeyCode::Char('h') | KeyCode::Right | KeyCode::Char('l') => {
                self.choice = !self.choice;
            }
            KeyCode::Enter => {
                sender.send(Event::CancelVM(self.choice))?;
            }
            _ => {}
        }

        Ok(())
    }
    pub fn render(&self, frame: &mut Frame) {
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

        let (message_block, choice_block) = {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Length(3)])
                .margin(2)
                .split(area);

            (chunks[0], chunks[1])
        };

        let message = Text::from("Are you sure you want to cancel ?")
            .centered()
            .bold();

        let (yes_block, no_block) = {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(50),
                    Constraint::Length(4),
                    Constraint::Percentage(50),
                ])
                .margin(1)
                .split(choice_block);

            (chunks[0], chunks[2])
        };

        let yes = Text::from("Yes")
            .style({
                if self.choice {
                    Style::default().bold().on_dark_gray()
                } else {
                    Style::default()
                }
            })
            .centered();

        let no = Text::from("No")
            .style({
                if !self.choice {
                    Style::default().bold().on_dark_gray()
                } else {
                    Style::default()
                }
            })
            .centered();

        frame.render_widget(Clear, area);
        frame.render_widget(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Thick)
                .border_style(Style::default().yellow()),
            area,
        );
        frame.render_widget(message, message_block);
        frame.render_widget(no, no_block);
        frame.render_widget(yes, yes_block);
    }
}
