use ratatui::{
    Frame,
    layout::Rect,
    style::Stylize,
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::{
    app::{
        App,
        FocusedSection::{self},
    },
    vmbuilder, vmedit,
};

#[derive(Debug)]
pub struct Help;

impl Help {
    pub fn render(&self, frame: &mut Frame, app: &App, block: Rect) {
        let message = match app.focused_section {
            FocusedSection::Main => {
                if block.width >= 113 {
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
                        Span::from("󱁐 ").bold(),
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
            FocusedSection::NewVM => {
                let vm_builder = &&app.new_vm.clone().unwrap();
                if vm_builder.cancel_confirmation.is_some() {
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
                    match vm_builder.focused_section {
                        vmbuilder::Section::Disk => {
                            if vm_builder.new_disk.is_some() {
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
                            } else {
                                vec![Line::from(vec![
                                    Span::from("↑").bold(),
                                    Span::from("  Up"),
                                    Span::from(" | "),
                                    Span::from("↓").bold(),
                                    Span::from("  Down"),
                                    Span::from(" | "),
                                    Span::from("n").bold(),
                                    Span::from("  New Disk"),
                                    Span::from(" | "),
                                    Span::from("d").bold(),
                                    Span::from("  Delete"),
                                    Span::from(" | "),
                                    Span::from("Esc").bold(),
                                    Span::from(" Cancel"),
                                    Span::from(" | "),
                                    Span::from("⇄").bold(),
                                    Span::from(" Nav"),
                                ])]
                            }
                        }
                        vmbuilder::Section::Summary => {
                            vec![Line::from(vec![
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
                        vmbuilder::Section::Network => {
                            vec![Line::from(vec![
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
                        _ => {
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
                                Span::from("⇄").bold(),
                                Span::from(" Nav"),
                            ])]
                        }
                    }
                }
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
                let edit_vm = &app.edit_vm.clone().unwrap();
                if matches!(edit_vm.focused_section, vmedit::Section::Hardware(_)) {
                    vec![Line::from(vec![
                        Span::from("↑").bold(),
                        Span::from("  Up"),
                        Span::from(" | "),
                        Span::from("↓").bold(),
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
                } else {
                    if edit_vm.new_disk.is_some() {
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
                    } else {
                        if block.width >= 113 {
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
                                Span::from("  New Disk"),
                                Span::from(" | "),
                                Span::from("d").bold(),
                                Span::from("  Delete"),
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
                                    Span::from("  New Disk"),
                                    Span::from(" | "),
                                    Span::from("d").bold(),
                                    Span::from("  Delete"),
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
        };

        let message = Paragraph::new(message).centered().blue();

        frame.render_widget(message, block);
    }
}
