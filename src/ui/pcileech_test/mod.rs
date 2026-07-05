pub mod types;

use crate::device_programmer::CREATE_NO_WINDOW;
use crate::ui::common::{self, palette};
use crate::utils::localization::{TextKey, translate};
use eframe::egui::{self, Color32, RichText};
use std::io::{BufRead, BufReader};
use std::process::{Child, ChildStderr, ChildStdout, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use std::os::windows::process::CommandExt;

const PCILEECH_TOOL_PATH: &str = "tools\\memflow-base\\memflow-base.exe";
const PCILEECH_IDLE_TIMEOUT: Duration = Duration::from_secs(20);
const PCILEECH_POLL_INTERVAL: Duration = Duration::from_millis(50);
const STATUS_CARD_WIDTH: f32 = 440.0;
const STATUS_CARD_RADIUS: u8 = 10;
const STATUS_CARD_FILL: Color32 = palette::SURFACE_RECESSED;
const STATUS_CARD_INNER_MARGIN_X: i8 = 18;
const STATUS_CARD_INNER_MARGIN_Y: i8 = 16;
const STATUS_CARD_STROKE_WIDTH: f32 = 1.0;
const STATUS_ICON_SIZE: f32 = 38.0;
const STATUS_TITLE_SIZE: f32 = 18.0;
const STATUS_TEXT_SIZE: f32 = 14.0;
const SUCCESS_COLOR: Color32 = palette::SUCCESS;
const ERROR_COLOR: Color32 = palette::ERROR;
const BODY_TEXT_COLOR: Color32 = palette::TEXT;
const MUTED_TEXT_COLOR: Color32 = palette::TEXT_MUTED;

pub fn render_pcileech_test(
    ui: &mut egui::Ui,
    test_state: &mut Option<Arc<Mutex<types::PcileechTestState>>>,
    on_back: &mut dyn FnMut(),
    lang: &crate::utils::localization::Language,
) {
    ui.vertical_centered(|ui| {
        ui.heading(translate(TextKey::TestPcileechTitle, lang));
        ui.add_space(30.0);

        // Initialize background worker thread
        if test_state.is_none() {
            let shared_state = Arc::new(Mutex::new(types::PcileechTestState::Running));
            *test_state = Some(Arc::clone(&shared_state));

            thread::spawn(move || {
                *shared_state.lock().unwrap() = run_pcileech_test();
            });
        }

        let state_clone = test_state
            .as_ref()
            .and_then(|state| state.lock().ok().map(|state| state.clone()))
            .unwrap_or_else(|| {
                types::PcileechTestState::Failed("Internal test state unavailable".to_string())
            });

        match state_clone {
            types::PcileechTestState::Running => {
                render_running_state(ui, lang);
                // Request continuous UI updates during polling
                ui.ctx().request_repaint();
            }
            types::PcileechTestState::Success(line) => {
                render_success_state(ui, &line, lang);
            }
            types::PcileechTestState::Failed(msg) => {
                render_error_state(ui, &msg, lang);
            }
        }
    });

    ui.add_space(40.0);
    ui.separator();
    ui.add_space(15.0);

    ui.horizontal(|ui| {
        let available_width = ui.available_width();
        let button_count = 2.0;
        let spacing = 12.0 * (button_count - 1.0);
        let button_width = (available_width - spacing) / button_count;

        if common::secondary_button(
            ui,
            translate(TextKey::MainMenu, lang),
            egui::vec2(button_width, 32.0),
        )
        .clicked()
        {
            // Reset state
            *test_state = None;
            on_back();
        }

        ui.add_space(12.0);

        if common::primary_button(
            ui,
            translate(TextKey::TryAgainButton, lang),
            egui::vec2(button_width, 32.0),
        )
        .clicked()
        {
            // Reset state to trigger re-execution
            *test_state = None;
        }
    });

    ui.add_space(15.0);
}

fn render_running_state(ui: &mut egui::Ui, lang: &crate::utils::localization::Language) {
    ui.add_space(10.0);
    render_status_card(ui, palette::INFO, |ui| {
        ui.vertical_centered(|ui| {
            ui.spinner();
            ui.add_space(12.0);
            ui.label(
                RichText::new(translate(TextKey::TestingConnection, lang))
                    .size(STATUS_TEXT_SIZE + 1.0)
                    .color(BODY_TEXT_COLOR),
            );
        });
    });
}

fn render_success_state(
    ui: &mut egui::Ui,
    line: &str,
    lang: &crate::utils::localization::Language,
) {
    ui.add_space(10.0);
    render_status_card(ui, SUCCESS_COLOR, |ui| {
        ui.vertical_centered(|ui| {
            render_status_icon(ui, egui_phosphor::regular::CHECK_CIRCLE, SUCCESS_COLOR);
            ui.add_space(8.0);
            ui.label(
                RichText::new(translate(TextKey::TestSuccess, lang))
                    .strong()
                    .size(STATUS_TITLE_SIZE)
                    .color(SUCCESS_COLOR),
            );
            ui.add_space(12.0);
            ui.label(
                RichText::new(line)
                    .size(STATUS_TEXT_SIZE)
                    .color(BODY_TEXT_COLOR),
            );
        });
    });
}

fn render_error_state(ui: &mut egui::Ui, msg: &str, lang: &crate::utils::localization::Language) {
    ui.add_space(10.0);
    render_status_card(ui, ERROR_COLOR, |ui| {
        ui.vertical_centered(|ui| {
            render_status_icon(ui, egui_phosphor::regular::X_CIRCLE, ERROR_COLOR);
            ui.add_space(8.0);
            ui.label(
                RichText::new(translate(TextKey::TestFailed, lang))
                    .strong()
                    .size(STATUS_TITLE_SIZE)
                    .color(ERROR_COLOR),
            );
        });

        ui.add_space(14.0);
        render_error_message(ui, msg);
        ui.add_space(14.0);
        render_connection_fixes(ui, lang);
    });
}

fn render_status_icon(ui: &mut egui::Ui, icon: &str, color: Color32) {
    ui.label(RichText::new(icon).color(color).size(STATUS_ICON_SIZE));
}

fn render_status_card(
    ui: &mut egui::Ui,
    border_color: Color32,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    let card_width = ui.available_width().min(STATUS_CARD_WIDTH);
    let content_width = (card_width - f32::from(STATUS_CARD_INNER_MARGIN_X) * 2.0).max(0.0);

    egui::Frame::NONE
        .fill(STATUS_CARD_FILL)
        .corner_radius(egui::CornerRadius::same(STATUS_CARD_RADIUS))
        .stroke(egui::Stroke::new(STATUS_CARD_STROKE_WIDTH, border_color))
        .inner_margin(egui::Margin::symmetric(
            STATUS_CARD_INNER_MARGIN_X,
            STATUS_CARD_INNER_MARGIN_Y,
        ))
        .show(ui, |ui| {
            ui.set_min_width(content_width);
            add_contents(ui);
        });
}

fn render_error_message(ui: &mut egui::Ui, msg: &str) {
    let message_width = (ui.available_width() - 24.0).max(0.0);

    egui::Frame::NONE
        .fill(Color32::from_rgb(17, 18, 21))
        .corner_radius(egui::CornerRadius::same(8))
        .inner_margin(egui::Margin::symmetric(12, 10))
        .show(ui, |ui| {
            ui.set_min_width(message_width);
            ui.label(
                RichText::new(msg)
                    .size(STATUS_TEXT_SIZE)
                    .color(Color32::from_rgb(255, 145, 150)),
            );
        });
}

fn render_connection_fixes(ui: &mut egui::Ui, lang: &crate::utils::localization::Language) {
    ui.vertical_centered(|ui| {
        for (index, line) in translate(TextKey::ConnectionError, lang)
            .lines()
            .enumerate()
        {
            let mut text = RichText::new(line)
                .size(STATUS_TEXT_SIZE)
                .color(MUTED_TEXT_COLOR);

            if index == 0 {
                text = text.strong().color(BODY_TEXT_COLOR);
            }

            ui.label(text);
        }
    });
}

fn run_pcileech_test() -> types::PcileechTestState {
    let mut child = match spawn_pcileech_tool() {
        Ok(child) => child,
        Err(error) => return types::PcileechTestState::Failed(error),
    };

    let (stdout, stderr) = match take_process_streams(&mut child) {
        Ok(streams) => streams,
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            return types::PcileechTestState::Failed(error);
        }
    };

    let output = Arc::new(Mutex::new(String::new()));
    let stdout_thread = spawn_output_reader(stdout, Arc::clone(&output));
    let stderr_thread = spawn_output_reader(stderr, Arc::clone(&output));

    let mut last_output_at = Instant::now();
    let mut last_output_len = 0;
    let mut success_line = None;
    let mut process_error = None;

    loop {
        let snapshot = output.lock().unwrap().clone();

        if snapshot.len() != last_output_len {
            last_output_at = Instant::now();
            last_output_len = snapshot.len();
        }

        if let Some(line) = find_success_line(&snapshot) {
            success_line = Some(line);
            let _ = child.kill();
            break;
        }

        if process_error.is_none() {
            process_error = find_error_message(&snapshot);
        }

        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() && process_error.is_none() {
                    process_error = Some(format!(
                        "PCILeech test exited with code: {:?}",
                        status.code()
                    ));
                }
                break;
            }
            Ok(None) => {}
            Err(error) => {
                process_error = Some(format!("Failed to monitor test tool: {error}"));
                let _ = child.kill();
                break;
            }
        }

        if last_output_at.elapsed() >= PCILEECH_IDLE_TIMEOUT {
            process_error.get_or_insert_with(|| {
                format!(
                    "PCILeech test timed out after {} seconds without output",
                    PCILEECH_IDLE_TIMEOUT.as_secs()
                )
            });
            let _ = child.kill();
            break;
        }

        thread::sleep(PCILEECH_POLL_INTERVAL);
    }

    let _ = child.wait();
    let _ = stdout_thread.join();
    let _ = stderr_thread.join();

    let final_output = output.lock().unwrap().clone();
    finalize_pcileech_result(&final_output, success_line, process_error)
}

fn spawn_pcileech_tool() -> Result<Child, String> {
    let mut command = Command::new(PCILEECH_TOOL_PATH);
    command
        .args(["-c", "pcileech", "--headless"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(CREATE_NO_WINDOW);

    command
        .spawn()
        .map_err(|error| format!("Failed to start test tool: {error}"))
}

fn take_process_streams(child: &mut Child) -> Result<(ChildStdout, ChildStderr), String> {
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Failed to capture test tool stdout".to_string())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "Failed to capture test tool stderr".to_string())?;

    Ok((stdout, stderr))
}

fn spawn_output_reader<T>(stream: T, output: Arc<Mutex<String>>) -> thread::JoinHandle<()>
where
    T: std::io::Read + Send + 'static,
{
    thread::spawn(move || {
        let reader = BufReader::new(stream);
        for line in reader.lines().map_while(Result::ok) {
            let mut output = output.lock().unwrap();
            output.push_str(&line);
            output.push('\n');
        }
    })
}

fn find_success_line(output: &str) -> Option<String> {
    output
        .lines()
        .find(|line| line.contains("ntdll.dll base address:"))
        .map(|line| line.trim().to_string())
}

fn find_error_message(output: &str) -> Option<String> {
    output
        .lines()
        .find(|line| line.contains("Error:"))
        .or_else(|| {
            output
                .lines()
                .find(|line| line.to_ascii_lowercase().contains("error"))
        })
        .map(|line| line.trim().to_string())
}

fn finalize_pcileech_result(
    output: &str,
    success_line: Option<String>,
    process_error: Option<String>,
) -> types::PcileechTestState {
    if let Some(line) = success_line.or_else(|| find_success_line(output)) {
        return types::PcileechTestState::Success(line);
    }

    types::PcileechTestState::Failed(
        process_error
            .or_else(|| find_error_message(output))
            .unwrap_or_else(|| "Unknown error".to_string()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_pcileech_success_signature() {
        let output = "memflow init\nntdll.dll base address: 0x7ffa0000\n";
        assert_eq!(
            find_success_line(output),
            Some("ntdll.dll base address: 0x7ffa0000".to_string())
        );
    }

    #[test]
    fn extracts_first_error_line() {
        let output = "startup\nError: failed to initialize connector\nmore detail\n";
        assert_eq!(
            find_error_message(output),
            Some("Error: failed to initialize connector".to_string())
        );
    }

    #[test]
    fn detects_lowercase_error_line() {
        let output = "connector error: device not found\n";
        assert_eq!(
            find_error_message(output),
            Some("connector error: device not found".to_string())
        );
    }

    #[test]
    fn final_output_success_wins_over_prior_error() {
        let output = "Error: transient connector warning\nntdll.dll base address: 0x7ffa0000\n";
        match finalize_pcileech_result(
            output,
            None,
            Some("Error: transient connector warning".to_string()),
        ) {
            types::PcileechTestState::Success(line) => {
                assert_eq!(line, "ntdll.dll base address: 0x7ffa0000");
            }
            state => panic!("expected success, got {state:?}"),
        }
    }

    #[test]
    fn final_output_preserves_error_when_no_success_signature() {
        let output = "startup\n";
        match finalize_pcileech_result(output, None, Some("Error: failed".to_string())) {
            types::PcileechTestState::Failed(error) => assert_eq!(error, "Error: failed"),
            state => panic!("expected failure, got {state:?}"),
        }
    }
}
