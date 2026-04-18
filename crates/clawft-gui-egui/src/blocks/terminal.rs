use eframe::egui;

use super::{DemoState, TerminalLineKind};

pub fn show(ui: &mut egui::Ui, state: &mut DemoState) {
    ui.heading("Terminal");
    ui.label("Mock REPL — try `help`, `clear`, `echo hi`, anything else errors.");
    ui.separator();

    egui::Frame::none()
        .fill(egui::Color32::from_rgb(8, 10, 14))
        .rounding(4.0)
        .inner_margin(egui::Margin::symmetric(10.0, 8.0))
        .show(ui, |ui| {
            ui.set_min_height(280.0);
            egui::ScrollArea::vertical()
                .stick_to_bottom(true)
                .max_height(280.0)
                .show(ui, |ui| {
                    for (kind, line) in &state.terminal_history {
                        let color = match kind {
                            TerminalLineKind::Input => egui::Color32::from_rgb(120, 200, 255),
                            TerminalLineKind::Output => egui::Color32::from_rgb(210, 210, 210),
                            TerminalLineKind::Error => egui::Color32::from_rgb(240, 120, 120),
                        };
                        let prefix = match kind {
                            TerminalLineKind::Input => "$ ",
                            _ => "  ",
                        };
                        ui.label(
                            egui::RichText::new(format!("{prefix}{line}"))
                                .monospace()
                                .color(color),
                        );
                    }
                });
        });

    ui.horizontal(|ui| {
        ui.label("$");
        let resp = ui.add(
            egui::TextEdit::singleline(&mut state.terminal_input)
                .desired_width(ui.available_width() - 60.0)
                .font(egui::TextStyle::Monospace),
        );
        let submit = ui.button("run").clicked()
            || (resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)));
        if submit {
            let cmd = std::mem::take(&mut state.terminal_input);
            run_command(state, cmd);
            resp.request_focus();
        }
    });
}

fn run_command(state: &mut DemoState, cmd: String) {
    let trimmed = cmd.trim();
    if trimmed.is_empty() {
        return;
    }
    state
        .terminal_history
        .push((TerminalLineKind::Input, cmd.clone()));
    let mut parts = trimmed.splitn(2, char::is_whitespace);
    match (parts.next(), parts.next()) {
        (Some("help"), _) => {
            state.terminal_history.push((
                TerminalLineKind::Output,
                "commands: help, clear, echo <text>".into(),
            ));
        }
        (Some("clear"), _) => {
            state.terminal_history.clear();
        }
        (Some("echo"), Some(rest)) => {
            state
                .terminal_history
                .push((TerminalLineKind::Output, rest.to_string()));
        }
        (Some("echo"), None) => {
            state.terminal_history.push((TerminalLineKind::Output, String::new()));
        }
        (Some(unknown), _) => {
            state.terminal_history.push((
                TerminalLineKind::Error,
                format!("{unknown}: command not found"),
            ));
        }
        _ => {}
    }
}
