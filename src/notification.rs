use std::fmt::Display;

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Text},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
};

#[derive(Debug, Clone)]
pub struct Notification {
    pub message: String,
    pub level: NotificationLevel,
    pub ttl: u16,
}

#[derive(Debug, Clone)]
pub enum NotificationLevel {
    Error,
    Warning,
    Info,
}

impl Notification {
    pub fn new<T: Display>(message: T, level: NotificationLevel) -> Self {
        Self {
            message: message.to_string(),
            level,
            ttl: 10,
        }
    }

    pub fn error<T: Display>(message: T) -> Self {
        Self {
            message: message.to_string(),
            level: NotificationLevel::Error,
            ttl: 10,
        }
    }

    pub fn render(&self, index: usize, frame: &mut Frame) {
        let (color, title) = match self.level {
            NotificationLevel::Info => (Color::Green, "Info"),
            NotificationLevel::Warning => (Color::Yellow, "Warning"),
            NotificationLevel::Error => (Color::Red, "Error"),
        };

        let mut text = Text::from(vec![
            Line::from(title).style(Style::new().fg(color).add_modifier(Modifier::BOLD)),
            Line::from(""),
        ]);

        text.extend(Text::from(self.message.clone()));

        let notification_height = text.height() as u16 + 4;
        let notification_width = text.width() as u16 + 4;

        let text = Paragraph::new(text)
            .wrap(Wrap { trim: false })
            .alignment(Alignment::Center);

        let area = notification_rect(
            index as u16,
            notification_height,
            notification_width,
            frame.area(),
        );

        frame.render_widget(Clear, area);
        frame.render_widget(
            Block::new()
                .borders(Borders::all())
                .border_type(BorderType::Thick)
                .border_style(Style::default().fg(Color::Yellow)),
            area,
        );
        frame.render_widget(
            text,
            area.inner(Margin {
                horizontal: 1,
                vertical: 2,
            }),
        );
    }
}

pub fn notification_rect(offset: u16, height: u16, width: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(height * offset),
                Constraint::Length(height),
                Constraint::Min(1),
            ]
            .as_ref(),
        )
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Min(1),
                Constraint::Length(width),
                Constraint::Length(2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
}
