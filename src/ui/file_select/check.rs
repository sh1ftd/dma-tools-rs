use super::FileCheckRenderContext;
use crate::APP_TITLE;
use crate::ui::common::{self, palette};
use crate::ui::file_select::components::render_missing_file;
use crate::utils::file_checker::{CheckStatus, FileCheckResult, SUCCESS_TRANSITION_DELAY};
use crate::utils::localization::{TextKey, translate};
use eframe::egui::{self, Color32, CornerRadius, Margin, RichText, Sense, Stroke, Ui, Vec2};

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
const SPINNER_OFFSET: f32 = 10.0;

// Text sizes
const TEXT_SIZE_NORMAL: f32 = 16.0;
const TEXT_SIZE_MEDIUM: f32 = 18.0;
const TEXT_SIZE_LARGE: f32 = 20.0;

// Colors
const COLOR_SUCCESS: Color32 = palette::SUCCESS;
const COLOR_WARNING: Color32 = palette::WARNING;
const COLOR_BORDER: Color32 = palette::STROKE;
const COLOR_WARNING_BORDER: Color32 = palette::WARNING;
const COLOR_WARNING_BG: Color32 = Color32::from_rgba_premultiplied(218, 156, 72, 35);
const COLOR_WARNING_TEXT: Color32 = Color32::from_rgb(255, 216, 156);
const ACTION_BUTTON_WIDTH: f32 = 220.0;
const ACTION_BUTTON_HEIGHT: f32 = 34.0;
const ACTION_BUTTON_SPACING: f32 = 12.0;
const COUNTDOWN_SPINNER_WIDTH: f32 = 20.0;
const COUNTDOWN_TEXT_WIDTH_FACTOR: f32 = 0.46;

/// Internal UI context for rendering sub-components
struct FileCheckUiContext<'a> {
    ui: &'a mut Ui,
    check_status: &'a CheckStatus,
    language: &'a crate::utils::localization::Language,
}

pub fn render_file_check(render_ctx: &mut FileCheckRenderContext<'_>) {
    let mut ui_ctx = FileCheckUiContext {
        ui: render_ctx.ui,
        check_status: render_ctx.check_status,
        language: render_ctx.language,
    };

    render_file_check_internal(&mut ui_ctx, render_ctx.on_continue, render_ctx.on_rescan);
}

fn render_file_check_internal(
    ctx: &mut FileCheckUiContext<'_>,
    on_continue: &mut dyn FnMut(bool),
    on_rescan: &mut dyn FnMut(),
) {
    ctx.ui.vertical_centered(|ui| {
        ui.heading(translate(TextKey::SystemCheck, ctx.language));

        match ctx.check_status {
            CheckStatus::NotStarted => render_not_started(ui, ctx.language),
            CheckStatus::Checking(current_file) => render_checking(ui, current_file, ctx.language),
            CheckStatus::Success(success_time) => {
                render_success_state(ui, success_time, ctx.language)
            }
            CheckStatus::Complete(result) => {
                if result.error_count > 0 {
                    render_check_failed(ui, result, on_continue, on_rescan, ctx.language);
                }
            }
            CheckStatus::ReadyToTransition => {
                // No need to render anything as we're about to transition
            }
        }
    });
}

// Status rendering functions
fn render_not_started(ui: &mut Ui, lang: &crate::utils::localization::Language) {
    ui.add_space(SPACING_XXLARGE);
    ui.add_space(SPACING_XXLARGE);
    ui.label(translate(TextKey::WelcomeMessage, lang).replace("{}", APP_TITLE));
    ui.add_space(SPACING_LARGE);
    ui.label(translate(TextKey::CheckingFiles, lang));
    ui.add_space(SPACING_LARGE);
    render_centered_spinner(ui);
}

fn render_checking(ui: &mut Ui, current_file: &str, lang: &crate::utils::localization::Language) {
    ui.add_space(SPACING_XXLARGE);
    ui.label(RichText::new(translate(TextKey::CheckingFiles, lang)).size(TEXT_SIZE_NORMAL));
    ui.add_space(SPACING_LARGE);
    render_centered_spinner(ui);
    ui.add_space(SPACING_XLARGE);
    ui.vertical_centered(|ui| {
        ui.label(
            RichText::new(translate(TextKey::CheckingItem, lang).replace("{}", current_file))
                .monospace(),
        );
    });
    ui.add_space(SPACING_XXLARGE);
}

fn render_success_state(
    ui: &mut Ui,
    success_time: &std::time::Instant,
    lang: &crate::utils::localization::Language,
) {
    // Container frame for better spacing control
    egui::Frame::NONE
        .inner_margin(Margin::symmetric(0, SPACING_SECTION as i8))
        .show(ui, |ui| {
            ui.vertical_centered(|ui| {
                // Checkmark circle
                render_checkmark(ui);

                ui.add_space(SPACING_LARGE);

                // Success message
                ui.colored_label(
                    COLOR_SUCCESS,
                    RichText::new(translate(TextKey::FileCheckSuccess, lang))
                        .size(TEXT_SIZE_MEDIUM)
                        .strong(),
                );

                // Countdown message
                render_countdown(ui, success_time, lang);
            });
        });
}

fn render_check_failed(
    ui: &mut Ui,
    check_result: &FileCheckResult,
    on_continue: &mut dyn FnMut(bool),
    on_rescan: &mut dyn FnMut(),
    lang: &crate::utils::localization::Language,
) {
    ui.vertical_centered(|ui| {
        ui.colored_label(
            COLOR_WARNING,
            RichText::new(format!(
                "{} {}",
                translate(TextKey::MissingFiles, lang),
                check_result.error_count
            ))
            .size(TEXT_SIZE_LARGE)
            .strong(),
        );

        ui.add_space(SPACING_LARGE);
        ui.add_space(SPACING_LARGE);
        render_missing_files_list(ui, check_result, lang);
        ui.separator();
        render_action_buttons(ui, on_continue, on_rescan, lang);
    });
}

// UI Components
fn render_checkmark(ui: &mut Ui) {
    let (rect, _) =
        ui.allocate_exact_size(Vec2::new(CHECKMARK_SIZE, CHECKMARK_SIZE), Sense::hover());
    let center = rect.center();

    let painter = ui.painter();
    painter.circle_filled(center, CHECKMARK_RADIUS, COLOR_SUCCESS);

    let stroke = Stroke::new(3.0_f32, palette::TEXT);
    let points = [
        center + Vec2::new(-CHECKMARK_RADIUS * 0.5_f32, 0.0_f32),
        center + Vec2::new(-CHECKMARK_RADIUS * 0.1_f32, CHECKMARK_RADIUS * 0.4_f32),
        center + Vec2::new(CHECKMARK_RADIUS * 0.5_f32, -CHECKMARK_RADIUS * 0.4_f32),
    ];
    painter.line_segment([points[0], points[1]], stroke);
    painter.line_segment([points[1], points[2]], stroke);
}

fn render_countdown(
    ui: &mut Ui,
    success_time: &std::time::Instant,
    lang: &crate::utils::localization::Language,
) {
    let elapsed = success_time.elapsed().as_secs();

    #[allow(clippy::absurd_extreme_comparisons)]
    if elapsed <= SUCCESS_TRANSITION_DELAY {
        let remaining = SUCCESS_TRANSITION_DELAY - elapsed;
        ui.add_space(SPACING_MEDIUM);

        let s_text = if remaining == 1 { "" } else { "s" };
        let countdown_text = translate(TextKey::CountdownMessage, lang)
            .replacen("{}", &remaining.to_string(), 1)
            .replacen("{}", s_text, 1);

        ui.horizontal(|ui| {
            let estimated_text_width = countdown_text.chars().count() as f32
                * TEXT_SIZE_NORMAL
                * COUNTDOWN_TEXT_WIDTH_FACTOR;
            let estimated_row_width =
                estimated_text_width + SPACING_SMALL + COUNTDOWN_SPINNER_WIDTH;
            ui.add_space(((ui.available_width() - estimated_row_width) / 2.0).max(0.0));
            ui.spacing_mut().item_spacing.x = SPACING_SMALL;
            ui.label(
                RichText::new(countdown_text)
                    .italics()
                    .size(TEXT_SIZE_NORMAL)
                    .color(palette::TEXT_MUTED),
            );
            ui.spinner();
        });
    }
}

fn render_centered_spinner(ui: &mut Ui) {
    ui.horizontal(|ui| {
        ui.add_space(ui.available_width() / 2.0_f32 - SPINNER_OFFSET);
        ui.spinner();
    });
}

fn render_missing_files_list(
    ui: &mut Ui,
    check_result: &FileCheckResult,
    lang: &crate::utils::localization::Language,
) {
    egui::Frame::dark_canvas(ui.style())
        .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
        .corner_radius(CornerRadius::same(SPACING_LARGE as u8))
        .inner_margin(Margin::same(SPACING_LARGE as i8))
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .max_height(MISSING_FILES_MAX_HEIGHT)
                .show(ui, |ui| {
                    render_file_groups(ui, &check_result.missing_files, lang);
                });
        });
}

fn render_action_buttons(
    ui: &mut Ui,
    on_continue: &mut dyn FnMut(bool),
    on_rescan: &mut dyn FnMut(),
    lang: &crate::utils::localization::Language,
) {
    render_warning_box(ui, lang);
    ui.add_space(SPACING_LARGE);

    ui.horizontal(|ui| {
        let available_width = ui.available_width();
        let button_width =
            ((available_width - ACTION_BUTTON_SPACING) / 2.0).min(ACTION_BUTTON_WIDTH);
        let leading_space =
            ((available_width - button_width * 2.0 - ACTION_BUTTON_SPACING) / 2.0).max(0.0);

        ui.add_space(leading_space);

        if common::secondary_icon_button(
            ui,
            Some(egui_phosphor::regular::ARROWS_CLOCKWISE),
            translate(TextKey::Rescan, lang),
            Vec2::new(button_width, ACTION_BUTTON_HEIGHT),
        )
        .clicked()
        {
            on_rescan();
        }

        ui.add_space(ACTION_BUTTON_SPACING);

        if common::primary_icon_button(
            ui,
            Some(egui_phosphor::regular::ARROW_RIGHT),
            translate(TextKey::ContinueAnyway, lang),
            Vec2::new(button_width, ACTION_BUTTON_HEIGHT),
        )
        .clicked()
        {
            on_continue(true);
        }
    });
}

fn render_warning_box(ui: &mut Ui, lang: &crate::utils::localization::Language) {
    egui::Frame::NONE
        .fill(COLOR_WARNING_BG)
        .stroke(Stroke::new(1.0_f32, COLOR_WARNING_BORDER))
        .corner_radius(CornerRadius::same(SPACING_LARGE as u8))
        .inner_margin(Margin::same(SPACING_LARGE as i8))
        .show(ui, |ui| {
            ui.colored_label(
                COLOR_WARNING_TEXT,
                RichText::new(translate(TextKey::MissingFilesWarning, lang))
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

fn render_file_groups(ui: &mut Ui, files: &[String], lang: &crate::utils::localization::Language) {
    let groups = group_files(files);

    if !groups.executables.is_empty() {
        render_file_group(
            ui,
            translate(TextKey::GroupExecutables, lang),
            &groups.executables,
        );
    }
    if !groups.libraries.is_empty() {
        render_file_group(
            ui,
            translate(TextKey::GroupLibraries, lang),
            &groups.libraries,
        );
    }
    if !groups.bitstreams.is_empty() {
        render_file_group(
            ui,
            translate(TextKey::GroupBitstreams, lang),
            &groups.bitstreams,
        );
    }
    if !groups.configs.is_empty() {
        render_file_group(ui, translate(TextKey::GroupConfigs, lang), &groups.configs);
    }
    if !groups.others.is_empty() {
        render_file_group(ui, translate(TextKey::GroupOther, lang), &groups.others);
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
