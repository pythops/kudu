use crossterm::event::KeyEvent;
use ratatui::{
    Frame,
    layout::{Margin, Rect},
    style::Stylize,
    text::{Line, Span, Text},
    widgets::ListItem,
};

use crate::network::NetworkBackend;

#[derive(Debug, Clone, Default)]
pub struct Network {
    backend: NetworkBackend,
}

impl Network {
    pub fn new() -> Self {
        Network::default()
    }

    pub fn backend(&self) -> NetworkBackend {
        self.backend
    }

    pub fn handle_key_events(&mut self, _key_event: KeyEvent) {}

    pub fn summary(&self) -> Vec<ListItem<'_>> {
        vec![ListItem::from(vec![
            Line::from(vec![
                Span::from("Network       ").bold(),
                Span::from(" ".repeat(6)),
                Span::from(self.backend.to_string()),
            ]),
            Line::from(""),
        ])]
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        let network =
            Text::from("Only User Network backend is supported for this version").centered();
        frame.render_widget(
            network,
            area.inner(Margin {
                horizontal: 0,
                vertical: 3,
            }),
        );
    }
}
