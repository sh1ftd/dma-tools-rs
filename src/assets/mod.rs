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
}

/// Icon manager to load and maintain SVG icons
pub struct IconManager {
    // copy_icon: Option<egui::TextureHandle>,
    checkmark_icon: Option<egui::TextureHandle>,
    x_icon: Option<egui::TextureHandle>,
}

impl IconManager {
    /// Creates a new IconManager with no icons loaded yet
    pub fn new() -> Self {
        Self {
            // copy_icon: None,
            checkmark_icon: None,
            x_icon: None,
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
    }

    /// Get the checkmark icon texture
    pub fn checkmark_icon(&self) -> Option<&egui::TextureHandle> {
        self.checkmark_icon.as_ref()
    }

    /// Get the X icon texture
    pub fn x_icon(&self) -> Option<&egui::TextureHandle> {
        self.x_icon.as_ref()
    }
}

/// Load an SVG icon and convert to an egui texture
fn load_svg_icon(ctx: &egui::Context, svg_data: &str) -> egui::TextureHandle {
    let opt = usvg::Options::default();
    let tree = usvg::Tree::from_str(svg_data, &opt).expect("Failed to parse SVG data");

    let mut pixmap = tiny_skia::Pixmap::new(DEFAULT_ICON_SIZE, DEFAULT_ICON_SIZE)
        .expect("Failed to create pixmap for icon");

    // Render the SVG to pixmap
    let transform = tiny_skia::Transform::identity();
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
