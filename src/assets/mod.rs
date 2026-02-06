//! Module for handling assets like images and icons
use eframe::egui;

const DEFAULT_ICON_SIZE: u32 = 32;

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

    // Flags
    pub const US_FLAG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="1235" height="650" viewBox="0 0 7410 3900"><path d="M0,0h7410v3900H0" fill="#b31942"/><path d="M0,450H7410m0,600H0m0,600H7410m0,600H0m0,600H7410m0,600H0" stroke="#FFF" stroke-width="300"/><path d="M0,0h2964v2100H0" fill="#0a3161"/><g fill="#FFF"><g id="s18"><g id="s9"><g id="s5"><g id="s4"><path id="s" d="M247,90 317.534230,307.082039 132.873218,172.917961H361.126782L176.465770,307.082039z"/><use xlink:href="#s" y="420"/><use xlink:href="#s" y="840"/><use xlink:href="#s" y="1260"/></g><use xlink:href="#s" y="1680"/></g><use xlink:href="#s4" x="247" y="210"/></g><use xlink:href="#s9" x="494"/></g><use xlink:href="#s18" x="988"/><use xlink:href="#s9" x="1976"/><use xlink:href="#s5" x="2470"/></g></svg>"##;

    pub const CN_FLAG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="900" height="600"><path fill="#EE1C25" d="M0 0h900v600H0"/><g transform="matrix(3 0 0 3 150 150)"><path id="a" d="m0-30 17.634 54.27-46.166-33.54h57.064l-46.166 33.54Z" fill="#FF0"/></g><use xlink:href="#a" transform="rotate(23.036 2.784 766.082)"/><use xlink:href="#a" transform="rotate(45.87 38.201 485.396)"/><use xlink:href="#a" transform="rotate(69.945 29.892 362.328)"/><use xlink:href="#a" transform="rotate(20.66 -590.66 957.955)"/></svg>"##;

    pub const DE_FLAG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="1000" height="600" viewBox="0 0 5 3"><desc>Flag of Germany</desc><rect id="black_stripe" width="5" height="3" y="0" x="0" fill="#000"/><rect id="red_stripe" width="5" height="2" y="1" x="0" fill="#D00"/><rect id="gold_stripe" width="5" height="1" y="2" x="0" fill="#FFCE00"/></svg>"##;

    pub const BR_FLAG: &str = r##"<svg width="1000" height="700" viewBox="-2100 -1470 4200 2940" xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink"><defs><g id="G"><clipPath id="g"><path d="m-31.5 0v-70h63v70zm31.5-47v12h31.5v-12z"/></clipPath><use clip-path="url(#g)" xlink:href="#O"/><path d="M5-35H31.5V-25H5z"/><path d="m21.5-35h10v35h-10z"/></g><g id="R"><use xlink:href="#P"/><path d="m28 0c0-10 0-32-15-32h-19c22 0 22 22 22 32"/></g><g id="s" fill="#fff"><g id="c"><path id="t" transform="rotate(18,0,-1)" d="m0-1v1h0.5"/><use transform="scale(-1,1)" xlink:href="#t"/></g><use transform="rotate(72)" xlink:href="#c"/><use transform="rotate(-72)" xlink:href="#c"/><use transform="rotate(144)" xlink:href="#c"/><use transform="rotate(216)" xlink:href="#c"/></g><g id="a"><use transform="scale(31.5)" xlink:href="#s"/></g><g id="b"><use transform="scale(26.25)" xlink:href="#s"/></g><g id="f"><use transform="scale(21)" xlink:href="#s"/></g><g id="h"><use transform="scale(15)" xlink:href="#s"/></g><g id="i"><use transform="scale(10.5)" xlink:href="#s"/></g><path id="D" d="m-31.5 0h33a30 30 0 0 0 30-30v-10a30 30 0 0 0-30-30h-33zm13-13h19a19 19 0 0 0 19-19v-6a19 19 0 0 0-19-19h-19z" fill-rule="evenodd"/><path id="E" transform="translate(-31.5)" d="m0 0h63v-13h-51v-18h40v-12h-40v-14h48v-13h-60z"/><path id="e" d="m-26.25 0h52.5v-12h-40.5v-16h33v-12h-33v-11h39.25v-12h-51.25z"/><path id="M" d="m-31.5 0h12v-48l14 48h11l14-48v48h12v-70h-17.5l-14 48-14-48h-17.5z"/><path id="O" d="m0 0a31.5 35 0 0 0 0-70 31.5 35 0 0 0 0 70m0-13a18.5 22 0 0 0 0-44 18.5 22 0 0 0 0 44" fill-rule="evenodd"/><path id="P" d="m-31.5 0h13v-26h28a22 22 0 0 0 0-44h-40zm13-39h27a9 9 0 0 0 0-18h-27z" fill-rule="evenodd"/><path id="S" d="m-15.75-22c0 7 6.75 10.5 16.75 10.5s14.74-3.25 14.75-7.75c0-14.25-46.75-5.25-46.5-30.25 0.25-21.5 24.75-20.5 33.75-20.5s26 4 25.75 21.25h-15.25c0-7.5-7-10.25-15-10.25-7.75 0-13.25 1.25-13.25 8.5-0.25 11.75 46.25 4 46.25 28.75 0 18.25-18 21.75-31.5 21.75-11.5 0-31.55-4.5-31.5-22z"/></defs><clipPath id="B"><circle r="735"/></clipPath><path d="m-2100-1470h4200v2940h-4200z" fill="#009440"/><path d="M -1743,0 0,1113 1743,0 0,-1113 Z" fill="#ffcb00"/><circle r="735" fill="#302681"/><path d="m-2205 1470a1785 1785 0 0 1 3570 0h-105a1680 1680 0 1 0-3360 0z" clip-path="url(#B)" fill="#fff"/><g transform="translate(-420,1470)" fill="#009440"><use transform="rotate(-7)" y="-1697.5" xlink:href="#O"/><use transform="rotate(-4)" y="-1697.5" xlink:href="#R"/><use transform="rotate(-1)" y="-1697.5" xlink:href="#D"/><use transform="rotate(2)" y="-1697.5" xlink:href="#E"/><use transform="rotate(5)" y="-1697.5" xlink:href="#M"/><use transform="rotate(9.75)" y="-1697.5" xlink:href="#e"/><use transform="rotate(14.5)" y="-1697.5" xlink:href="#P"/><use transform="rotate(17.5)" y="-1697.5" xlink:href="#R"/><use transform="rotate(20.5)" y="-1697.5" xlink:href="#O"/><use transform="rotate(23.5)" y="-1697.5" xlink:href="#G"/><use transform="rotate(26.5)" y="-1697.5" xlink:href="#R"/><use transform="rotate(29.5)" y="-1697.5" xlink:href="#E"/><use transform="rotate(32.5)" y="-1697.5" xlink:href="#S"/><use transform="rotate(35.5)" y="-1697.5" xlink:href="#S"/><use transform="rotate(38.5)" y="-1697.5" xlink:href="#O"/></g><use x="-600" y="-132" xlink:href="#a"/><use x="-535" y="177" xlink:href="#a"/><use x="-625" y="243" xlink:href="#b"/><use x="-463" y="132" xlink:href="#h"/><use x="-382" y="250" xlink:href="#b"/><use x="-404" y="323" xlink:href="#f"/><use x="228" y="-228" xlink:href="#a"/><use x="515" y="258" xlink:href="#a"/><use x="617" y="265" xlink:href="#f"/><use x="545" y="323" xlink:href="#b"/><use x="368" y="477" xlink:href="#b"/><use x="367" y="551" xlink:href="#f"/><use x="441" y="419" xlink:href="#f"/><use x="500" y="382" xlink:href="#b"/><use x="365" y="405" xlink:href="#f"/><use x="-280" y="30" xlink:href="#b"/><use x="200" y="-37" xlink:href="#f"/><use y="330" xlink:href="#a"/><use x="85" y="184" xlink:href="#b"/><use y="118" xlink:href="#b"/><use x="-74" y="184" xlink:href="#f"/><use x="-37" y="235" xlink:href="#h"/><use x="220" y="495" xlink:href="#b"/><use x="283" y="430" xlink:href="#f"/><use x="162" y="412" xlink:href="#f"/><use x="-295" y="390" xlink:href="#a"/><use y="575" xlink:href="#i"/></svg>"##;

    pub const AR_FLAG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="900" height="600"><path d="M0 0h900v600H0z"/><path fill="#fff" d="M0 0h900v400H0z"/><path fill="#ce1126" d="M0 0h900v200H0z"/></svg>"##;
}

/// Icon manager to load and maintain SVG icons
pub struct IconManager {
    // copy_icon: Option<egui::TextureHandle>,
    checkmark_icon: Option<egui::TextureHandle>,
    x_icon: Option<egui::TextureHandle>,
    discord_icon: Option<egui::TextureHandle>,
    telegram_icon: Option<egui::TextureHandle>,
    wechat_icon: Option<egui::TextureHandle>,
    us_flag: Option<egui::TextureHandle>,
    cn_flag: Option<egui::TextureHandle>,
    de_flag: Option<egui::TextureHandle>,
    br_flag: Option<egui::TextureHandle>,
    ar_flag: Option<egui::TextureHandle>,
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
            us_flag: None,
            cn_flag: None,
            de_flag: None,
            br_flag: None,
            ar_flag: None,
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
        if self.us_flag.is_none() {
            self.us_flag = Some(load_svg_icon(ctx, svg::US_FLAG));
        }
        if self.cn_flag.is_none() {
            self.cn_flag = Some(load_svg_icon(ctx, svg::CN_FLAG));
        }
        if self.de_flag.is_none() {
            self.de_flag = Some(load_svg_icon(ctx, svg::DE_FLAG));
        }
        if self.br_flag.is_none() {
            self.br_flag = Some(load_svg_icon(ctx, svg::BR_FLAG));
        }
        if self.ar_flag.is_none() {
            self.ar_flag = Some(load_svg_icon(ctx, svg::AR_FLAG));
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

    /// Get the US flag texture
    pub fn us_flag(&self) -> Option<&egui::TextureHandle> {
        self.us_flag.as_ref()
    }

    /// Get the Chinese flag texture
    pub fn cn_flag(&self) -> Option<&egui::TextureHandle> {
        self.cn_flag.as_ref()
    }

    /// Get the German flag texture
    pub fn de_flag(&self) -> Option<&egui::TextureHandle> {
        self.de_flag.as_ref()
    }

   /// Get the Brazilian flag texture
    pub fn br_flag(&self) -> Option<&egui::TextureHandle> {
        self.br_flag.as_ref()
    }

    /// Get the Yemen flag texture (Arabic)
    pub fn ar_flag(&self) -> Option<&egui::TextureHandle> {
        self.ar_flag.as_ref()
    }
}

/// Load an SVG icon and convert to an egui texture
fn load_svg_icon(ctx: &egui::Context, svg_data: &str) -> egui::TextureHandle {
    let opt = usvg::Options::default();
    let tree = usvg::Tree::from_str(svg_data, &opt).expect("Failed to parse SVG data");

    let mut pixmap = resvg::tiny_skia::Pixmap::new(DEFAULT_ICON_SIZE, DEFAULT_ICON_SIZE)
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

    let transform = resvg::tiny_skia::Transform::from_scale(scale, scale)
        .post_translate(dx, dy);

    resvg::render(&tree, transform, &mut pixmap.as_mut());

    let image = egui::ColorImage::from_rgba_premultiplied(
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

    ctx.load_texture(
        &texture_id,
        image,
        egui::TextureOptions::LINEAR
    )
}
