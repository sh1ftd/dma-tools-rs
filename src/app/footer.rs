use super::FirmwareToolApp;
use crate::utils::localization::{Language, TextKey, translate};
use eframe::egui;
use std::time::{Duration, Instant};

#[cfg(not(feature = "branding"))]
use crate::utils::contact;

const FOOTER_VERTICAL_PADDING: f32 = 4.0;
const BASE_FLAG_SIZE: f32 = 28.0;
const CONTACT_BUTTON_SIZE: f32 = 34.0;
const CONTACT_ICON_SIZE: f32 = 28.0;
const COPY_NOTIFICATION_DURATION_SECS: u64 = 2;
const COPY_NOTIFICATION_MAX_WIDTH: f32 = 110.0;

impl FirmwareToolApp {
    fn render_language_flag(
        ui: &mut egui::Ui,
        flag_icon: Option<egui::TextureHandle>,
        lang: Language,
        tooltip: &str,
        button_px: f32,
        base_flag_size: f32,
        current_language: &mut Language,
    ) {
        let button_size = egui::Vec2::splat(button_px);
        let (rect, response) = ui.allocate_exact_size(button_size, egui::Sense::click());

        if response.clicked() {
            *current_language = lang;
        }
        let response = response.on_hover_text(tooltip);

        let target_scale = if response.hovered() { 1.15 } else { 1.0 };
        let scale =
            ui.ctx()
                .animate_value_with_time(response.id.with("flag_scale"), target_scale, 0.1);

        if ui.is_rect_visible(rect)
            && let Some(icon) = flag_icon
        {
            let flag_size_vec = egui::Vec2::splat(base_flag_size * scale);

            let tint = if *current_language == lang {
                egui::Color32::WHITE
            } else {
                egui::Color32::from_white_alpha(100)
            };

            let image = egui::Image::new(&icon)
                .fit_to_exact_size(flag_size_vec)
                .tint(tint);
            ui.put(rect, image);
        }
    }

    fn render_contact_icon(
        ui: &mut egui::Ui,
        icon_glyph: &str,
        copy_text: &str,
        tooltip: &str,
        notification_msg: String,
        notification: &mut Option<(String, Instant)>,
    ) {
        let button_size = egui::Vec2::splat(CONTACT_BUTTON_SIZE);
        let (rect, response) = ui.allocate_exact_size(button_size, egui::Sense::click());

        if response.clicked() {
            ui.ctx().copy_text(copy_text.to_string());
            *notification = Some((notification_msg, Instant::now()));
        }
        let response = response.on_hover_text(tooltip);

        let is_hovered = response.hovered();

        let target_scale = if is_hovered { 1.15 } else { 1.0 };
        let scale = ui
            .ctx()
            .animate_value_with_time(response.id.with("scale"), target_scale, 0.1);

        if ui.is_rect_visible(rect) {
            let icon_size = CONTACT_ICON_SIZE * scale;
            let _icon_rect =
                egui::Rect::from_center_size(rect.center(), egui::Vec2::splat(icon_size));

            let tint = if is_hovered {
                egui::Color32::WHITE
            } else {
                egui::Color32::LIGHT_GRAY
            };

            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                icon_glyph,
                egui::FontId::proportional(icon_size * 0.8),
                tint,
            );
        }
    }

    pub(super) fn render_contact_info(&mut self, ui: &mut egui::Ui) {
        ui.add_space(FOOTER_VERTICAL_PADDING);

        ui.horizontal(|ui| {
            {
                Self::render_language_flag(
                    ui,
                    self.icon_manager.us_flag().cloned(),
                    Language::English,
                    "English",
                    34.0,
                    BASE_FLAG_SIZE,
                    &mut self.language,
                );
            }

            ui.add_space(8.0);

            {
                Self::render_language_flag(
                    ui,
                    self.icon_manager.cn_flag().cloned(),
                    Language::Chinese,
                    "中文",
                    38.0,
                    BASE_FLAG_SIZE,
                    &mut self.language,
                );
            }

            ui.add_space(8.0);

            {
                Self::render_language_flag(
                    ui,
                    self.icon_manager.de_flag().cloned(),
                    Language::German,
                    "Deutsch",
                    34.0,
                    BASE_FLAG_SIZE,
                    &mut self.language,
                );
            }

            ui.add_space(8.0);

            {
                Self::render_language_flag(
                    ui,
                    self.icon_manager.br_flag().cloned(),
                    Language::Portuguese,
                    "Português",
                    34.0,
                    BASE_FLAG_SIZE,
                    &mut self.language,
                );
            }

            ui.add_space(8.0);

            {
                Self::render_language_flag(
                    ui,
                    self.icon_manager.ar_flag().cloned(),
                    Language::Arabic,
                    "العربية",
                    34.0,
                    BASE_FLAG_SIZE,
                    &mut self.language,
                );
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(10.0);

                #[cfg(feature = "branding")]
                let (show_tg, tg_contact, show_wc, wc_contact, show_dc, dc_contact) = (
                    crate::branding::SHOW_TELEGRAM,
                    crate::branding::TELEGRAM_CONTACT,
                    crate::branding::SHOW_WECHAT,
                    crate::branding::WECHAT_CONTACT,
                    crate::branding::SHOW_DISCORD,
                    crate::branding::DISCORD_CONTACT,
                );

                #[cfg(not(feature = "branding"))]
                let (show_tg, tg_contact, show_wc, wc_contact, show_dc, dc_contact) = (
                    contact::SHOW_TELEGRAM,
                    contact::TELEGRAM_CONTACT,
                    contact::SHOW_WECHAT,
                    contact::WECHAT_CONTACT,
                    contact::SHOW_DISCORD,
                    contact::DISCORD_CONTACT,
                );

                if show_tg {
                    Self::render_contact_icon(
                        ui,
                        egui_phosphor::regular::TELEGRAM_LOGO,
                        tg_contact,
                        translate(TextKey::CopyTelegram, &self.language),
                        translate(TextKey::Copied, &self.language)
                            .replace("{}", translate(TextKey::TelegramLink, &self.language)),
                        &mut self.contact_copy_notification,
                    );
                }

                if show_tg && (show_wc || show_dc) {
                    ui.add_space(4.0);
                }

                if show_wc {
                    Self::render_contact_icon(
                        ui,
                        egui_phosphor::regular::WECHAT_LOGO,
                        wc_contact,
                        translate(TextKey::CopyWeChat, &self.language),
                        translate(TextKey::Copied, &self.language)
                            .replace("{}", translate(TextKey::WeChatID, &self.language)),
                        &mut self.contact_copy_notification,
                    );
                }

                if show_wc && show_dc {
                    ui.add_space(4.0);
                }

                if show_dc {
                    Self::render_contact_icon(
                        ui,
                        egui_phosphor::regular::DISCORD_LOGO,
                        dc_contact,
                        translate(TextKey::CopyDiscord, &self.language),
                        translate(TextKey::Copied, &self.language)
                            .replace("{}", translate(TextKey::DiscordID, &self.language)),
                        &mut self.contact_copy_notification,
                    );
                }

                if show_tg || show_wc || show_dc {
                    ui.label(translate(TextKey::Contact, &self.language));
                }
            });
        });
        ui.add_space(FOOTER_VERTICAL_PADDING);
    }

    pub(super) fn render_contact_copy_notification(&mut self, ctx: &egui::Context) {
        if let Some((msg, time)) = &self.contact_copy_notification
            && time.elapsed() < Duration::from_secs(COPY_NOTIFICATION_DURATION_SECS)
        {
            egui::Area::new(egui::Id::new("copy_notification"))
                .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-20.0, -85.0))
                .show(ctx, |ui| {
                    egui::Frame::NONE
                        .fill(egui::Color32::from_black_alpha(192))
                        .corner_radius(6.0)
                        .inner_margin(8.0)
                        .show(ui, |ui| {
                            ui.set_max_width(COPY_NOTIFICATION_MAX_WIDTH);
                            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                                ui.label(
                                    egui::RichText::new(msg)
                                        .color(egui::Color32::GREEN)
                                        .size(14.0),
                                );
                            });
                        });
                });
            ctx.request_repaint();
        }
    }
}
