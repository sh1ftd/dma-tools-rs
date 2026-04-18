pub mod types;

use crate::utils::localization::{TextKey, translate};
use eframe::egui::{self, Color32, RichText};
use std::io::{BufRead, BufReader, Read};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;

use std::os::windows::process::CommandExt;

pub fn render_pcileech_test(
    ui: &mut egui::Ui,
    test_state: &mut Option<Arc<Mutex<types::PcileechTestState>>>,
    on_back: &mut dyn FnMut(),
    lang: &crate::app::Language,
) {
    ui.vertical_centered(|ui| {
        ui.heading(translate(TextKey::TestPcileechTitle, lang));
        ui.add_space(30.0);

        // Initialize background worker thread
        if test_state.is_none() {
            let shared_state = Arc::new(Mutex::new(types::PcileechTestState::Running));
            *test_state = Some(Arc::clone(&shared_state));

            thread::spawn(move || {
                let mut cmd = Command::new("tools\\memflow-base\\memflow-base.exe");
                cmd.args(["-c", "pcileech", "--headless"])
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped());

                {
                    const CREATE_NO_WINDOW: u32 = 0x08000000;
                    cmd.creation_flags(CREATE_NO_WINDOW);
                }

                let mut child = match cmd.spawn() {
                    Ok(c) => c,
                    Err(e) => {
                        *shared_state.lock().unwrap() = types::PcileechTestState::Failed(format!(
                            "Failed to start test tool: {}",
                            e
                        ));
                        return;
                    }
                };

                // Read output
                let stdout = child.stdout.take().unwrap();
                let stderr = child.stderr.take().unwrap();

                let mut reader = BufReader::new(stdout);
                let mut err_reader = BufReader::new(stderr);

                let mut all_output = String::new();
                let mut ntdll_output = None;

                // Poll stdout until success signature is found or EOF is reached
                loop {
                    let mut line = String::new();
                    match reader.read_line(&mut line) {
                        Ok(0) => break, // EOF
                        Ok(_) => {
                            all_output.push_str(&line);
                            if line.contains("ntdll.dll base address:") {
                                ntdll_output = Some(line.trim().to_string());
                                // Target address located; terminate detached process
                                let _ = child.kill();
                                break;
                            }
                            if line.contains("error") || line.contains("Error:") {
                                // Handle explicit error states
                                let _ = child.kill();
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }

                // Reap child process
                let _ = child.wait();

                // Capture remaining stderr
                let mut err_out = String::new();
                let _ = err_reader.read_to_string(&mut err_out);
                all_output.push_str(&err_out);

                let mut final_state = shared_state.lock().unwrap();

                if let Some(ntdll) = ntdll_output {
                    *final_state = types::PcileechTestState::Success(ntdll);
                } else {
                    // Extract error payload from standard output streams
                    let mut err_msg = "Unknown error".to_string();
                    if let Some(idx) = all_output.find("Error:") {
                        err_msg = all_output[idx..]
                            .lines()
                            .next()
                            .unwrap_or("Error")
                            .to_string();
                    } else if all_output.contains("error") {
                        err_msg = all_output;
                    }
                    *final_state = types::PcileechTestState::Failed(err_msg);
                }
            });
        }

        let state_clone = {
            let s = test_state.as_ref().unwrap().lock().unwrap();
            s.clone()
        };

        match state_clone {
            types::PcileechTestState::Running => {
                ui.add_space(20.0);
                ui.spinner();
                ui.add_space(10.0);
                ui.label(RichText::new(translate(TextKey::TestingConnection, lang)).size(16.0));

                // Request continuous UI updates during polling
                ui.ctx().request_repaint();
            }
            types::PcileechTestState::Success(line) => {
                ui.add_space(10.0);
                ui.label(
                    RichText::new(egui_phosphor::regular::CHECK_CIRCLE)
                        .color(Color32::from_rgb(50, 200, 50))
                        .size(40.0),
                );
                ui.add_space(10.0);
                ui.label(
                    RichText::new(translate(TextKey::TestSuccess, lang))
                        .strong()
                        .size(18.0)
                        .color(Color32::from_rgb(50, 200, 50)),
                );
                ui.add_space(10.0);
                ui.label(RichText::new(line).size(16.0));
            }
            types::PcileechTestState::Failed(msg) => {
                ui.add_space(10.0);
                ui.label(
                    RichText::new(egui_phosphor::regular::X_CIRCLE)
                        .color(Color32::from_rgb(200, 50, 50))
                        .size(40.0),
                );
                ui.add_space(10.0);
                ui.label(
                    RichText::new(translate(TextKey::TestFailed, lang))
                        .strong()
                        .size(18.0)
                        .color(Color32::from_rgb(200, 50, 50)),
                );
                ui.add_space(10.0);

                egui::ScrollArea::vertical()
                    .max_height(100.0)
                    .show(ui, |ui| {
                        ui.label(RichText::new(msg).color(Color32::from_rgb(255, 100, 100)));
                    });

                ui.add_space(10.0);
                ui.label(translate(TextKey::ConnectionError, lang));
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

        if ui
            .add(
                egui::Button::new(translate(TextKey::MainMenu, lang))
                    .min_size(egui::vec2(button_width, 32.0)),
            )
            .clicked()
        {
            // Reset state
            *test_state = None;
            on_back();
        }

        ui.add_space(12.0);

        if ui
            .add(
                egui::Button::new(translate(TextKey::TryAgainButton, lang))
                    .min_size(egui::vec2(button_width, 32.0)),
            )
            .clicked()
        {
            // Reset state to trigger re-execution
            *test_state = None;
        }
    });

    ui.add_space(15.0);
}
