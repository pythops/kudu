use ratatui::{
    Frame,
    layout::Rect,
    style::Stylize,
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::app::{
    App,
    FocusedSection::{self},
};

#[derive(Debug)]
pub struct Help;

impl Help {
    pub fn render(&self, frame: &mut Frame, app: &App, block: Rect) {
        let message = match app.focused_section {
            FocusedSection::Main => {
                if block.width >= 117 {
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
                        Span::from(" Stop"),
                        Span::from(" | "),
                        Span::from("x").bold(),
                        Span::from(" Kill"),
                        Span::from(" | "),
                        Span::from("p").bold(),
                        Span::from(" Preview"),
                        Span::from(" | "),
                        Span::from("q").bold(),
                        Span::from(" Quit"),
                        Span::from(" | "),
                        Span::from("󱁐  or 󰌑 ").bold(),
                        Span::from(" Start/Pause/Resume"),
                    ])]
                } else {
                    vec![
                        Line::from(vec![
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
                        ]),
                        Line::from(vec![
                            Span::from("s").bold(),
                            Span::from(" Stop"),
                            Span::from(" | "),
                            Span::from("x").bold(),
                            Span::from(" Kill"),
                            Span::from(" | "),
                            Span::from("p").bold(),
                            Span::from(" Preview"),
                            Span::from(" | "),
                            Span::from("q").bold(),
                            Span::from(" Quit"),
                            Span::from(" | "),
                            Span::from("󱁐 ").bold(),
                            Span::from(" Start/Pause/Resume"),
                        ]),
                    ]
                }
            }
            FocusedSection::NewVM => app.new_vm.as_ref().unwrap().help(),
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
            FocusedSection::EditVM => app.edit_vm.as_ref().unwrap().help(block.width),
        };

        let message = Paragraph::new(message).centered().blue();

        frame.render_widget(message, block);
    }
}
