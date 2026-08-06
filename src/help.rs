use ratatui::{
    Frame,
    layout::Rect,
    style::Stylize,
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::app::FocusedSection;

#[derive(Debug)]
pub struct Help;

impl Help {
    pub fn render(&self, frame: &mut Frame, focused_section: &FocusedSection, block: Rect) {
        let message = match focused_section {
            FocusedSection::Main => {
                vec![Line::from(vec![
                    Span::from("k,↑").bold(),
                    Span::from("  Up"),
                    Span::from(" | "),
                    Span::from("j,↓").bold(),
                    Span::from("  Down"),
                    Span::from(" | "),
                    Span::from("n").bold(),
                    Span::from(" New"),
                    Span::from(" | "),
                    Span::from("e").bold(),
                    Span::from(" Edit"),
                    Span::from(" | "),
                    Span::from("d").bold(),
                    Span::from(" Delete"),
                    Span::from(" | "),
                    Span::from("s").bold(),
                    Span::from(" Powerdown"),
                    Span::from(" | "),
                    Span::from("x").bold(),
                    Span::from(" Shutdown"),
                    Span::from(" | "),
                    Span::from("q").bold(),
                    Span::from(" Quit"),
                    Span::from(" | "),
                    Span::from("󱁐 ").bold(),
                    Span::from(" Start/Pause/Resume"),
                ])]
            }
            FocusedSection::NewVM => {
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
                    Span::from("Esc").bold(),
                    Span::from(" Cancel"),
                    Span::from(" | "),
                    Span::from("Enter").bold(),
                    Span::from(" Create"),
                    Span::from(" | "),
                    Span::from("⇄").bold(),
                    Span::from(" Nav"),
                ])]
            }
            FocusedSection::DeleteConfirmation => {
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
            }
            FocusedSection::EditVM => {
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
                    Span::from("Esc").bold(),
                    Span::from(" Cancel"),
                    Span::from(" | "),
                    Span::from("Enter").bold(),
                    Span::from(" Confirm"),
                ])]
            }
        };

        let message = Paragraph::new(message).centered().blue();

        frame.render_widget(message, block);
    }
}
