use super::FileCheckRenderContext;
use crate::APP_TITLE;
use crate::ui::file_select::components::render_missing_file;
use crate::utils::file_checker::{CheckStatus, FileCheckResult, SUCCESS_TRANSITION_DELAY};
use eframe::egui::{self, Color32, Margin, RichText, Rounding, Sense, Stroke, Ui, Vec2};

// UI Constants
const SPACING_SMALL: f32 = 4.0;
const SPACING_MEDIUM: f32 = 8.0;
const SPACING_LARGE: f32 = 12.0;
const SPACING_XLARGE: f32 = 16.0;
const SPACING_XXLARGE: f32 = 18.0;
const SPACING_SECTION: f32 = 24.0;

// Sizes
const CHECKMARK_SIZE: f32 = 48.0;
const CHECKMARK_RADIUS: f32 = 24.0;
const MISSING_FILES_MAX_HEIGHT: f32 = 250.0;
const BUTTON_SPACER_WIDTH: f32 = 150.0;
const SPINNER_OFFSET: f32 = 10.0;

// Text sizes
const TEXT_SIZE_NORMAL: f32 = 16.0;
const TEXT_SIZE_MEDIUM: f32 = 18.0;
const TEXT_SIZE_LARGE: f32 = 20.0;

// Colors
const COLOR_SUCCESS: Color32 = Color32::from_rgb(100, 200, 100);
const COLOR_WARNING: Color32 = Color32::from_rgb(255, 100, 0);
const COLOR_BORDER: Color32 = Color32::from_rgb(150, 150, 150);
const COLOR_WARNING_BORDER: Color32 = Color32::from_rgb(255, 160, 0);
const COLOR_WARNING_BG: Color32 = Color32::from_rgba_premultiplied(255, 160, 0, 25);

/// Internal UI context for rendering sub-components
struct FileCheckUiContext<'a> {
    ui: &'a mut Ui,
    check_status: &'a CheckStatus,
}

pub fn render_file_check(render_ctx: &mut FileCheckRenderContext<'_>) {
    let mut ui_ctx = FileCheckUiContext {
        ui: render_ctx.ui,
        check_status: render_ctx.check_status,
    };

    render_file_check_internal(&mut ui_ctx, render_ctx.on_continue, render_ctx.on_rescan);
}

fn render_file_check_internal(
    ctx: &mut FileCheckUiContext<'_>,
    on_continue: &mut dyn FnMut(bool),
    on_rescan: &mut dyn FnMut(),
) {
    ctx.ui.vertical_centered(|ui| {
        ui.heading("System Check");

        match ctx.check_status {
            CheckStatus::NotStarted => render_not_started(ui),
            CheckStatus::Checking(current_file) => render_checking(ui, current_file),
            CheckStatus::Success(success_time) => render_success_state(ui, success_time),
            CheckStatus::Complete(result) => {
                if result.error_count > 0 {
                    render_check_failed(ui, result, on_continue, on_rescan);
                }
            }
            CheckStatus::ReadyToTransition => {
                // No need to render anything as we're about to transition
            }
        }
    });
}

// Status rendering functions
fn render_not_started(ui: &mut Ui) {
    ui.add_space(SPACING_XXLARGE);
    ui.label(format!("Welcome to the {APP_TITLE} Tool"));
    ui.add_space(SPACING_LARGE);
    ui.label("Checking system files...");
    ui.add_space(SPACING_LARGE);
    render_centered_spinner(ui);
}

fn render_checking(ui: &mut Ui, current_file: &str) {
    ui.add_space(SPACING_XXLARGE);
    ui.label(RichText::new("Checking required files...").size(TEXT_SIZE_NORMAL));
    ui.add_space(SPACING_LARGE);
    render_centered_spinner(ui);
    ui.add_space(SPACING_XLARGE);
    ui.vertical_centered(|ui| {
        ui.label(RichText::new(format!("Checking: {current_file}")).monospace());
    });
    ui.add_space(SPACING_XXLARGE);
}

fn render_success_state(ui: &mut Ui, success_time: &std::time::Instant) {
    // Container frame for better spacing control
    egui::Frame::none()
        .inner_margin(Margin::symmetric(0.0, SPACING_SECTION))
        .show(ui, |ui| {
            ui.vertical_centered(|ui| {
                // Checkmark circle
                render_checkmark(ui);

                ui.add_space(SPACING_LARGE);

                // Success message
                ui.colored_label(
                    COLOR_SUCCESS,
                    RichText::new("All required files are present!")
                        .size(TEXT_SIZE_MEDIUM)
                        .strong(),
                );

                // Countdown message
                render_countdown(ui, success_time);
            });
        });
}

fn render_check_failed(
    ui: &mut Ui,
    check_result: &FileCheckResult,
    on_continue: &mut dyn FnMut(bool),
    on_rescan: &mut dyn FnMut(),
) {
    ui.vertical_centered(|ui| {
        ui.colored_label(
            COLOR_WARNING,
            RichText::new(format!(
                "Missing {} required files:",
                check_result.error_count
            ))
            .size(TEXT_SIZE_LARGE)
            .strong(),
        );

        ui.add_space(SPACING_LARGE);
        render_missing_files_list(ui, check_result);
        ui.separator();
        render_action_buttons(ui, on_continue, on_rescan);
    });
}

// UI Components
fn render_checkmark(ui: &mut Ui) {
    let (rect, _) =
        ui.allocate_exact_size(Vec2::new(CHECKMARK_SIZE, CHECKMARK_SIZE), Sense::hover());
    let center = rect.center();

    let painter = ui.painter();
    painter.circle_filled(center, CHECKMARK_RADIUS, COLOR_SUCCESS);

    let stroke = Stroke::new(3.0, Color32::WHITE);
    let points = [
        center + Vec2::new(-CHECKMARK_RADIUS * 0.5, 0.0),
        center + Vec2::new(-CHECKMARK_RADIUS * 0.1, CHECKMARK_RADIUS * 0.4),
        center + Vec2::new(CHECKMARK_RADIUS * 0.5, -CHECKMARK_RADIUS * 0.4),
    ];
    painter.line_segment([points[0], points[1]], stroke);
    painter.line_segment([points[1], points[2]], stroke);
}

fn render_countdown(ui: &mut Ui, success_time: &std::time::Instant) {
    let elapsed = success_time.elapsed().as_secs();

    #[allow(clippy::absurd_extreme_comparisons)]
    if elapsed <= SUCCESS_TRANSITION_DELAY {
        let remaining = SUCCESS_TRANSITION_DELAY - elapsed;
        ui.add_space(SPACING_MEDIUM);

        let countdown_text = format!(
            "Continuing automatically in {} second{}...",
            remaining,
            if remaining == 1 { "" } else { "s" }
        );

        ui.with_layout(
            egui::Layout::top_down_justified(egui::Align::Center),
            |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(112.0);
                    ui.label(
                        RichText::new(countdown_text)
                            .italics()
                            .size(TEXT_SIZE_NORMAL),
                    );
                    ui.add_space(SPACING_SMALL);
                    ui.spinner();
                });
            },
        );
    }
}

fn render_centered_spinner(ui: &mut Ui) {
    ui.horizontal(|ui| {
        ui.add_space(ui.available_width() / 2.0 - SPINNER_OFFSET);
        ui.spinner();
    });
}

fn render_missing_files_list(ui: &mut Ui, check_result: &FileCheckResult) {
    egui::Frame::dark_canvas(ui.style())
        .stroke(Stroke::new(1.0, COLOR_BORDER))
        .rounding(Rounding::same(SPACING_LARGE))
        .inner_margin(Margin::same(SPACING_LARGE))
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .max_height(MISSING_FILES_MAX_HEIGHT)
                .show(ui, |ui| {
                    render_file_groups(ui, &check_result.missing_files);
                });
        });
}

fn render_action_buttons(
    ui: &mut Ui,
    on_continue: &mut dyn FnMut(bool),
    on_rescan: &mut dyn FnMut(),
) {
    render_warning_box(ui);
    ui.add_space(SPACING_LARGE);

    ui.horizontal(|ui| {
        if ui
            .button(RichText::new("Exit").size(TEXT_SIZE_MEDIUM))
            .clicked()
        {
            on_continue(false);
        }

        if ui
            .button(RichText::new("Rescan Files").size(TEXT_SIZE_MEDIUM))
            .clicked()
        {
            on_rescan();
        }

        ui.add_space(ui.available_width() - BUTTON_SPACER_WIDTH);

        if ui
            .button(RichText::new("Continue Anyway").size(TEXT_SIZE_MEDIUM))
            .clicked()
        {
            on_continue(true);
        }
    });
}

fn render_warning_box(ui: &mut Ui) {
    egui::Frame::none()
        .fill(COLOR_WARNING_BG)
        .stroke(Stroke::new(1.0, COLOR_WARNING_BORDER))
        .rounding(Rounding::same(SPACING_LARGE))
        .inner_margin(Margin::same(SPACING_LARGE))
        .show(ui, |ui| {
            ui.colored_label(
                Color32::BLACK,
                RichText::new("WARNING: Continuing without required files may cause errors")
                    .size(TEXT_SIZE_MEDIUM)
                    .strong(),
            );
        });
}

// File grouping functionality
#[derive(Default)]
struct FileGroups<'a> {
    executables: Vec<&'a String>,
    libraries: Vec<&'a String>,
    bitstreams: Vec<&'a String>,
    configs: Vec<&'a String>,
    others: Vec<&'a String>,
}

fn group_files(files: &[String]) -> FileGroups<'_> {
    let mut groups = FileGroups::default();

    for file in files {
        match file {
            f if f.ends_with(".exe") => groups.executables.push(file),
            f if f.ends_with(".dll") => groups.libraries.push(file),
            f if f.ends_with(".bit") => groups.bitstreams.push(file),
            f if f.ends_with(".cfg") => groups.configs.push(file),
            _ => groups.others.push(file),
        }
    }

    groups
}

fn render_file_groups(ui: &mut Ui, files: &[String]) {
    let groups = group_files(files);

    if !groups.executables.is_empty() {
        render_file_group(ui, "Executables", &groups.executables);
    }
    if !groups.libraries.is_empty() {
        render_file_group(ui, "Libraries", &groups.libraries);
    }
    if !groups.bitstreams.is_empty() {
        render_file_group(ui, "Bitstreams", &groups.bitstreams);
    }
    if !groups.configs.is_empty() {
        render_file_group(ui, "Configuration Files", &groups.configs);
    }
    if !groups.others.is_empty() {
        render_file_group(ui, "Other Files", &groups.others);
    }
}

fn render_file_group(ui: &mut Ui, title: &str, files: &[&String]) {
    if !files.is_empty() {
        ui.label(RichText::new(title).size(TEXT_SIZE_MEDIUM).strong());
        ui.add_space(SPACING_SMALL);
        for file in files {
            render_missing_file(ui, file, TEXT_SIZE_NORMAL);
        }
        ui.add_space(SPACING_MEDIUM);
    }
}
