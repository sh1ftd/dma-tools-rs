mod components;
mod dna;
mod flash;

use super::types::ResultAction;
use crate::device_programmer::OperationSnapshot;
use eframe::egui::Ui;

pub fn render_result_screen(
    ui: &mut Ui,
    snapshot: &OperationSnapshot,
    on_action: &mut dyn FnMut(ResultAction),
    lang: &crate::utils::localization::Language,
) {
    if snapshot
        .option
        .as_ref()
        .is_some_and(|option| option.is_dna_read())
    {
        dna::render(ui, snapshot, lang);
    } else {
        flash::render(ui, snapshot, lang);
    }

    components::render_action_buttons(ui, on_action, lang, snapshot.safe_to_restart);
}
