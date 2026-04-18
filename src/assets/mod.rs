//! Module for handling assets like images and icons
use eframe::egui;

const DEFAULT_ICON_SIZE: u32 = 32;

// SVG icon definitions
mod svg {
    // Flags
    pub const US_FLAG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="1235" height="650" viewBox="0 0 7410 3900"><path d="M0,0h7410v3900H0" fill="#b31942"/><path d="M0,450H7410m0,600H0m0,600H7410m0,600H0m0,600H7410m0,600H0" stroke="#FFF" stroke-width="300"/><path d="M0,0h2964v2100H0" fill="#0a3161"/><g fill="#FFF"><g id="s18"><g id="s9"><g id="s5"><g id="s4"><path id="s" d="M247,90 317.534230,307.082039 132.873218,172.917961H361.126782L176.465770,307.082039z"/><use xlink:href="#s" y="420"/><use xlink:href="#s" y="840"/><use xlink:href="#s" y="1260"/></g><use xlink:href="#s" y="1680"/></g><use xlink:href="#s4" x="247" y="210"/></g><use xlink:href="#s9" x="494"/></g><use xlink:href="#s18" x="988"/><use xlink:href="#s9" x="1976"/><use xlink:href="#s5" x="2470"/></g></svg>"##;

    pub const CN_FLAG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="900" height="600"><path fill="#EE1C25" d="M0 0h900v600H0"/><g transform="matrix(3 0 0 3 150 150)"><path id="a" d="m0-30 17.634 54.27-46.166-33.54h57.064l-46.166 33.54Z" fill="#FF0"/></g><use xlink:href="#a" transform="rotate(23.036 2.784 766.082)"/><use xlink:href="#a" transform="rotate(45.87 38.201 485.396)"/><use xlink:href="#a" transform="rotate(69.945 29.892 362.328)"/><use xlink:href="#a" transform="rotate(20.66 -590.66 957.955)"/></svg>"##;

    pub const DE_FLAG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="1000" height="600" viewBox="0 0 5 3"><desc>Flag of Germany</desc><rect id="black_stripe" width="5" height="3" y="0" x="0" fill="#000"/><rect id="red_stripe" width="5" height="2" y="1" x="0" fill="#D00"/><rect id="gold_stripe" width="5" height="1" y="2" x="0" fill="#FFCE00"/></svg>"##;

    pub const BR_FLAG: &str = r##"<svg width="1000" height="700" viewBox="-2100 -1470 4200 2940" xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink"><defs><g id="G"><clipPath id="g"><path d="m-31.5 0v-70h63v70zm31.5-47v12h31.5v-12z"/></clipPath><use clip-path="url(#g)" xlink:href="#O"/><path d="M5-35H31.5V-25H5z"/><path d="m21.5-35h10v35h-10z"/></g><g id="R"><use xlink:href="#P"/><path d="m28 0c0-10 0-32-15-32h-19c22 0 22 22 22 32"/></g><g id="s" fill="#fff"><g id="c"><path id="t" transform="rotate(18,0,-1)" d="m0-1v1h0.5"/><use transform="scale(-1,1)" xlink:href="#t"/></g><use transform="rotate(72)" xlink:href="#c"/><use transform="rotate(-72)" xlink:href="#c"/><use transform="rotate(144)" xlink:href="#c"/><use transform="rotate(216)" xlink:href="#c"/></g><g id="a"><use transform="scale(31.5)" xlink:href="#s"/></g><g id="b"><use transform="scale(26.25)" xlink:href="#s"/></g><g id="f"><use transform="scale(21)" xlink:href="#s"/></g><g id="h"><use transform="scale(15)" xlink:href="#s"/></g><g id="i"><use transform="scale(10.5)" xlink:href="#s"/></g><path id="D" d="m-31.5 0h33a30 30 0 0 0 30-30v-10a30 30 0 0 0-30-30h-33zm13-13h19a19 19 0 0 0 19-19v-6a19 19 0 0 0-19-19h-19z" fill-rule="evenodd"/><path id="E" transform="translate(-31.5)" d="m0 0h63v-13h-51v-18h40v-12h-40v-14h48v-13h-60z"/><path id="e" d="m-26.25 0h52.5v-12h-40.5v-16h33v-12h-33v-11h39.25v-12h-51.25z"/><path id="M" d="m-31.5 0h12v-48l14 48h11l14-48v48h12v-70h-17.5l-14 48-14-48h-17.5z"/><path id="O" d="m0 0a31.5 35 0 0 0 0-70 31.5 35 0 0 0 0 70m0-13a18.5 22 0 0 0 0-44 18.5 22 0 0 0 0 44" fill-rule="evenodd"/><path id="P" d="m-31.5 0h13v-26h28a22 22 0 0 0 0-44h-40zm13-39h27a9 9 0 0 0 0-18h-27z" fill-rule="evenodd"/><path id="S" d="m-15.75-22c0 7 6.75 10.5 16.75 10.5s14.74-3.25 14.75-7.75c0-14.25-46.75-5.25-46.5-30.25 0.25-21.5 24.75-20.5 33.75-20.5s26 4 25.75 21.25h-15.25c0-7.5-7-10.25-15-10.25-7.75 0-13.25 1.25-13.25 8.5-0.25 11.75 46.25 4 46.25 28.75 0 18.25-18 21.75-31.5 21.75-11.5 0-31.55-4.5-31.5-22z"/></defs><clipPath id="B"><circle r="735"/></clipPath><path d="m-2100-1470h4200v2940h-4200z" fill="#009440"/><path d="M -1743,0 0,1113 1743,0 0,-1113 Z" fill="#ffcb00"/><circle r="735" fill="#302681"/><path d="m-2205 1470a1785 1785 0 0 1 3570 0h-105a1680 1680 0 1 0-3360 0z" clip-path="url(#B)" fill="#fff"/><g transform="translate(-420,1470)" fill="#009440"><use transform="rotate(-7)" y="-1697.5" xlink:href="#O"/><use transform="rotate(-4)" y="-1697.5" xlink:href="#R"/><use transform="rotate(-1)" y="-1697.5" xlink:href="#D"/><use transform="rotate(2)" y="-1697.5" xlink:href="#E"/><use transform="rotate(5)" y="-1697.5" xlink:href="#M"/><use transform="rotate(9.75)" y="-1697.5" xlink:href="#e"/><use transform="rotate(14.5)" y="-1697.5" xlink:href="#P"/><use transform="rotate(17.5)" y="-1697.5" xlink:href="#R"/><use transform="rotate(20.5)" y="-1697.5" xlink:href="#O"/><use transform="rotate(23.5)" y="-1697.5" xlink:href="#G"/><use transform="rotate(26.5)" y="-1697.5" xlink:href="#R"/><use transform="rotate(29.5)" y="-1697.5" xlink:href="#E"/><use transform="rotate(32.5)" y="-1697.5" xlink:href="#S"/><use transform="rotate(35.5)" y="-1697.5" xlink:href="#S"/><use transform="rotate(38.5)" y="-1697.5" xlink:href="#O"/></g><use x="-600" y="-132" xlink:href="#a"/><use x="-535" y="177" xlink:href="#a"/><use x="-625" y="243" xlink:href="#b"/><use x="-463" y="132" xlink:href="#h"/><use x="-382" y="250" xlink:href="#b"/><use x="-404" y="323" xlink:href="#f"/><use x="228" y="-228" xlink:href="#a"/><use x="515" y="258" xlink:href="#a"/><use x="617" y="265" xlink:href="#f"/><use x="545" y="323" xlink:href="#b"/><use x="368" y="477" xlink:href="#b"/><use x="367" y="551" xlink:href="#f"/><use x="441" y="419" xlink:href="#f"/><use x="500" y="382" xlink:href="#b"/><use x="365" y="405" xlink:href="#f"/><use x="-280" y="30" xlink:href="#b"/><use x="200" y="-37" xlink:href="#f"/><use y="330" xlink:href="#a"/><use x="85" y="184" xlink:href="#b"/><use y="118" xlink:href="#b"/><use x="-74" y="184" xlink:href="#f"/><use x="-37" y="235" xlink:href="#h"/><use x="220" y="495" xlink:href="#b"/><use x="283" y="430" xlink:href="#f"/><use x="162" y="412" xlink:href="#f"/><use x="-295" y="390" xlink:href="#a"/><use y="575" xlink:href="#i"/></svg>"##;

    pub const AR_FLAG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="900" height="600" viewBox="0 0 11880 7920">><path fill="#fff" d="m0 0h11880v7920H0z"/><path fill="#cd1125" d="m0 0h11880v2640H0z"/><path d="m0 5280h11880v2640H0z"/><path fill="#017b3d" d="m5864 4515H3929a288 248 0 0 1-365 215c271-133 254-268 83-568 95-34 110-43 206-108-68 206 176 181 356 181 0-72 7-154-47-165 70-25 76-33 187-127v277h1335v-190a40 40 0 0 0-80 0v110a30 30 0 0 1-30 30H4554v-180l766-740c-5 38 74 140 107 157-25 4-53-1-71 17l-627 606h695c0-161 150-161 220-218 70 57 220 57 220 218zm145 0V3250c71 39 126 84 214 106-4 50-49 66-49 101v778c98 22 120-35 167-64 12 124 91 246 88 344zm1322-845 155-130v680h110v-773c54-45 124-94 155-151v1219h-975c-14-252-14-511 280-455v-103c0-24-36-5-36-27l201-168v458h110zm-51-348c-19 1-48-103-41-123 7-23 33-23 44-12 18 17 16 134-3 135zm-181 141c-55-32-46-45 2-31 83 25 125 4 185-57l45 23c59 30 95 17 116-55 6-22 24-16 29 9 19 100-57 131-134 103-42-14-49-14-70 2-46 36-112 42-173 6zm797 1052V3250c71 39 126 84 214 106-4 50-49 66-49 101v778c98 22 120-35 167-64 12 124 91 246 88 344zm-3791 140a1 1 0 0 1 118 0 1 1 0 0 1-118 0zm2861-460a45 34 0 0 0 90 0 45 34 0 0 0-90 0z"/>
</svg>"##;
}

/// Icon manager to load and maintain SVG icons
pub struct IconManager {
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
            us_flag: None,
            cn_flag: None,
            de_flag: None,
            br_flag: None,
            ar_flag: None,
        }
    }

    /// Loads icons into GPU memory if not already loaded
    pub fn ensure_loaded(&mut self, ctx: &egui::Context) {
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

    let transform = resvg::tiny_skia::Transform::from_scale(scale, scale).post_translate(dx, dy);

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

    ctx.load_texture(&texture_id, image, egui::TextureOptions::LINEAR)
}
