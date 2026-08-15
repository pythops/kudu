use crossterm::event::KeyEvent;

use ratatui::{Frame, layout::Rect, widgets::ListItem};

use crate::access::{self, vnc::VncBuilder};

#[derive(Debug, Default, Clone)]
pub struct RemoteAccessBuilder {
    vnc: VncBuilder,
}

impl RemoteAccessBuilder {
    pub fn new() -> Self {
        RemoteAccessBuilder::default()
    }

    pub fn summary(&self) -> Vec<ListItem<'_>> {
        self.vnc.summary()
    }

    pub fn validate(&mut self) -> bool {
        self.vnc.validate()
    }

    pub fn handle_key_events(&mut self, key_event: KeyEvent) {
        self.vnc.handle_key_events(key_event);
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect, cancel_popup: bool) {
        self.vnc.render(frame, area, cancel_popup);
    }

    pub fn access(&self) -> Option<access::RemoteAccess> {
        if let Some(vnc) = self.vnc.build() {
            return Some(access::RemoteAccess::Vnc(vnc));
        }
        None
    }
}
