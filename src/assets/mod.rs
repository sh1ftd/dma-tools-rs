//! Module for handling assets like images and icons
use eframe::egui;

// Default size for rendering icons
const DEFAULT_ICON_SIZE: u32 = 512;

// SVG icon definitions
mod svg {
    pub const CHECKMARK_ICON: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 448 512">
    <path d="M438.6 105.4c12.5 12.5 12.5 32.8 0 45.3l-256 256c-12.5 12.5-32.8 12.5-45.3 0l-128-128c-12.5-12.5-12.5-32.8 0-45.3s32.8-12.5 45.3 0L160 338.7 393.4 105.4c12.5-12.5 32.8-12.5 45.3 0z" fill="#22AA22"/>
    </svg>"##;

    pub const X_ICON: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 384 512">
    <path d="M376.6 84.5c11.3-13.6 9.5-33.8-4.1-45.1s-33.8-9.5-45.1 4.1L192 206 56.6 43.5C45.3 29.9 25.1 28.1 11.5 39.4S-3.9 70.9 7.4 84.5L150.3 256 7.4 427.5c-11.3 13.6-9.5 33.8 4.1 45.1s33.8 9.5 45.1-4.1L192 306 327.4 468.5c11.3 13.6 31.5 15.4 45.1 4.1s15.4-31.5 4.1-45.1L233.7 256 376.6 84.5z" fill="#FF4646"/>
    </svg>"##;

    // Discord
    pub const DISCORD_ICON: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 640 512"><!--!Font Awesome Free 6.6.0 by @fontawesome - https://fontawesome.com License - https://fontawesome.com/license/free Copyright 2024 Fonticons, Inc.--><path fill="#FFFFFF" d="M524.5 69.8a1.5 1.5 0 0 0 -.8-.7A485.1 485.1 0 0 0 404.1 32a1.8 1.8 0 0 0 -1.9 .9 337.5 337.5 0 0 0 -14.9 30.6 447.8 447.8 0 0 0 -134.4 0 309.5 309.5 0 0 0 -15.1-30.6 1.9 1.9 0 0 0 -1.9-.9A483.7 483.7 0 0 0 116.1 69.1a1.7 1.7 0 0 0 -.8 .7C39.1 183.7 18.2 294.7 28.4 404.4a2 2 0 0 0 .8 1.4A487.7 487.7 0 0 0 176 479.9a1.9 1.9 0 0 0 2.1-.7A348.2 348.2 0 0 0 208.1 430.4a1.9 1.9 0 0 0 -1-2.6 321.2 321.2 0 0 1 -45.9-21.9 1.9 1.9 0 0 1 -.2-3.1c3.1-2.3 6.2-4.7 9.1-7.1a1.8 1.8 0 0 1 1.9-.3c96.2 43.9 200.4 43.9 295.5 0a1.8 1.8 0 0 1 1.9 .2c2.9 2.4 6 4.9 9.1 7.2a1.9 1.9 0 0 1 -.2 3.1 301.4 301.4 0 0 1 -45.9 21.8 1.9 1.9 0 0 0 -1 2.6 391.1 391.1 0 0 0 30 48.8 1.9 1.9 0 0 0 2.1 .7A486 486 0 0 0 610.7 405.7a1.9 1.9 0 0 0 .8-1.4C623.7 277.6 590.9 167.5 524.5 69.8zM222.5 337.6c-29 0-52.8-26.6-52.8-59.2S193.1 219.1 222.5 219.1c29.7 0 53.3 26.8 52.8 59.2C275.3 311 251.9 337.6 222.5 337.6zm195.4 0c-29 0-52.8-26.6-52.8-59.2S388.4 219.1 417.9 219.1c29.7 0 53.3 26.8 52.8 59.2C470.7 311 447.5 337.6 417.9 337.6z"/></svg>"##;

    // Telegram
    pub const TELEGRAM_ICON: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 496 512"><!--!Font Awesome Free 6.6.0 by @fontawesome - https://fontawesome.com License - https://fontawesome.com/license/free Copyright 2024 Fonticons, Inc.--><path fill="#FFFFFF" d="M248 8C111 8 0 119 0 256S111 504 248 504 496 393 496 256 385 8 248 8zM363 176.7c-3.7 39.2-19.9 134.4-28.1 178.3-3.5 18.6-10.3 24.8-16.9 25.4-14.4 1.3-25.3-9.5-39.3-18.7-21.8-14.3-34.2-23.2-55.3-37.2-24.5-16.1-8.6-25 5.3-39.5 3.7-3.8 67.1-61.5 68.3-66.7 .2-.7 .3-3.1-1.2-4.4s-3.6-.8-5.1-.5q-3.3 .7-104.6 69.1-14.8 10.2-26.9 9.9c-8.9-.2-25.9-5-38.6-9.1-15.5-5-27.9-7.7-26.8-16.3q.8-6.7 18.5-13.7 108.4-47.2 144.6-62.3c68.9-28.6 83.2-33.6 92.5-33.8 2.1 0 6.6 .5 9.6 2.9a10.5 10.5 0 0 1 3.5 6.7A43.8 43.8 0 0 1 363 176.7z"/></svg>"##;

    // Wechat
    pub const WECHAT_ICON: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 576 512"><!--!Font Awesome Free 6.6.0 by @fontawesome - https://fontawesome.com License - https://fontawesome.com/license/free Copyright 2024 Fonticons, Inc.--><path fill="#FFFFFF" d="M385.2 167.6c6.4 0 12.6 .3 18.8 1.1C387.4 90.3 303.3 32 207.7 32 100.5 32 13 104.8 13 197.4c0 53.4 29.3 97.5 77.9 131.6l-19.3 58.6 68-34.1c24.4 4.8 43.8 9.7 68.2 9.7 6.2 0 12.1-.3 18.3-.8-4-12.9-6.2-26.6-6.2-40.8-.1-84.9 72.9-154 165.3-154zm-104.5-52.9c14.5 0 24.2 9.7 24.2 24.4 0 14.5-9.7 24.2-24.2 24.2-14.8 0-29.3-9.7-29.3-24.2 .1-14.7 14.6-24.4 29.3-24.4zm-136.4 48.6c-14.5 0-29.3-9.7-29.3-24.2 0-14.8 14.8-24.4 29.3-24.4 14.8 0 24.4 9.7 24.4 24.4 0 14.6-9.6 24.2-24.4 24.2zM563 319.4c0-77.9-77.9-141.3-165.4-141.3-92.7 0-165.4 63.4-165.4 141.3S305 460.7 397.6 460.7c19.3 0 38.9-5.1 58.6-9.9l53.4 29.3-14.8-48.6C534 402.1 563 363.2 563 319.4zm-219.1-24.5c-9.7 0-19.3-9.7-19.3-19.6 0-9.7 9.7-19.3 19.3-19.3 14.8 0 24.4 9.7 24.4 19.3 0 10-9.7 19.6-24.4 19.6zm107.1 0c-9.7 0-19.3-9.7-19.3-19.6 0-9.7 9.7-19.3 19.3-19.3 14.5 0 24.4 9.7 24.4 19.3 .1 10-9.9 19.6-24.4 19.6z"/></svg>"##;
}

/// Icon manager to load and maintain SVG icons
pub struct IconManager {
    // copy_icon: Option<egui::TextureHandle>,
    checkmark_icon: Option<egui::TextureHandle>,
    x_icon: Option<egui::TextureHandle>,
    discord_icon: Option<egui::TextureHandle>,
    telegram_icon: Option<egui::TextureHandle>,
    wechat_icon: Option<egui::TextureHandle>,
}

impl IconManager {
    /// Creates a new IconManager with no icons loaded yet
    pub fn new() -> Self {
        Self {
            // copy_icon: None,
            checkmark_icon: None,
            x_icon: None,
            discord_icon: None,
            telegram_icon: None,
            wechat_icon: None,
        }
    }

    /// Loads icons into GPU memory if not already loaded
    pub fn ensure_loaded(&mut self, ctx: &egui::Context) {
        if self.checkmark_icon.is_none() {
            self.checkmark_icon = Some(load_svg_icon(ctx, svg::CHECKMARK_ICON));
        }
        if self.x_icon.is_none() {
            self.x_icon = Some(load_svg_icon(ctx, svg::X_ICON));
        }
        if self.discord_icon.is_none() {
            self.discord_icon = Some(load_svg_icon(ctx, svg::DISCORD_ICON));
        }
        if self.telegram_icon.is_none() {
            self.telegram_icon = Some(load_svg_icon(ctx, svg::TELEGRAM_ICON));
        }
        if self.wechat_icon.is_none() {
            self.wechat_icon = Some(load_svg_icon(ctx, svg::WECHAT_ICON));
        }
    }

    /// Get the checkmark icon texture
    pub fn checkmark_icon(&self) -> Option<&egui::TextureHandle> {
        self.checkmark_icon.as_ref()
    }

    /// Get the X icon texture
    pub fn x_icon(&self) -> Option<&egui::TextureHandle> {
        self.x_icon.as_ref()
    }

    /// Get the Discord icon texture
    pub fn discord_icon(&self) -> Option<&egui::TextureHandle> {
        self.discord_icon.as_ref()
    }

    /// Get the Telegram icon texture
    pub fn telegram_icon(&self) -> Option<&egui::TextureHandle> {
        self.telegram_icon.as_ref()
    }

    /// Get the WeChat icon texture
    pub fn wechat_icon(&self) -> Option<&egui::TextureHandle> {
        self.wechat_icon.as_ref()
    }
}

/// Load an SVG icon and convert to an egui texture
fn load_svg_icon(ctx: &egui::Context, svg_data: &str) -> egui::TextureHandle {
    let opt = usvg::Options::default();
    let tree = usvg::Tree::from_str(svg_data, &opt).expect("Failed to parse SVG data");

    let mut pixmap = tiny_skia::Pixmap::new(DEFAULT_ICON_SIZE, DEFAULT_ICON_SIZE)
        .expect("Failed to create pixmap for icon");

    // Render the SVG to pixmap
    let tree_size = tree.size();
    let width = tree_size.width();
    let height = tree_size.height();

    let scale_x = DEFAULT_ICON_SIZE as f32 / width;
    let scale_y = DEFAULT_ICON_SIZE as f32 / height;
    
    // Use the smaller scale to fit entirely within the box
    let scale = scale_x.min(scale_y);

    // Calculate centering offsets
    let dx = (DEFAULT_ICON_SIZE as f32 - width * scale) / 2.0;
    let dy = (DEFAULT_ICON_SIZE as f32 - height * scale) / 2.0;

    let transform = tiny_skia::Transform::from_scale(scale, scale)
        .post_translate(dx, dy);

    resvg::render(&tree, transform, &mut pixmap.as_mut());

    let image = egui::ColorImage::from_rgba_unmultiplied(
        [DEFAULT_ICON_SIZE as usize, DEFAULT_ICON_SIZE as usize],
        pixmap.data(),
    );

    // Create unique ID for the texture
    let texture_id = format!(
        "icon_{}",
        std::time::SystemTime::now()
            .elapsed()
            .unwrap_or_default()
            .as_nanos()
    );

    ctx.load_texture(&texture_id, image, egui::TextureOptions::default())
}
