use eframe::egui;

pub const BG: egui::Color32 = egui::Color32::from_rgb(0x12, 0x14, 0x18);
pub const SURFACE: egui::Color32 = egui::Color32::from_rgb(0x1A, 0x1D, 0x24);
pub const TEXT: egui::Color32 = egui::Color32::from_rgb(0xE8, 0xE6, 0xE1);
pub const TEAL: egui::Color32 = egui::Color32::from_rgb(0x2B, 0xB8, 0xA8);
pub const EMBER: egui::Color32 = egui::Color32::from_rgb(0xE4, 0x57, 0x4D);
pub const AMBER: egui::Color32 = egui::Color32::from_rgb(0xE0, 0xA8, 0x4A);

/// Load Signal fonts + ink-on-glass visuals into an egui context.
pub fn apply_signal_theme(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "ibm_plex".into(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
            "../../../assets/fonts/IBMPlexSans-Regular.ttf"
        ))),
    );
    fonts.font_data.insert(
        "ibm_plex_medium".into(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
            "../../../assets/fonts/IBMPlexSans-Medium.ttf"
        ))),
    );
    fonts.font_data.insert(
        "jetbrains_mono".into(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
            "../../../assets/fonts/JetBrainsMono-Regular.ttf"
        ))),
    );

    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "ibm_plex".into());
    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .insert(0, "jetbrains_mono".into());

    ctx.set_fonts(fonts);

    let mut visuals = egui::Visuals::dark();
    visuals.window_fill = BG;
    visuals.panel_fill = SURFACE;
    visuals.override_text_color = Some(TEXT);
    visuals.widgets.noninteractive.fg_stroke.color = TEXT;
    visuals.widgets.inactive.fg_stroke.color = TEXT;
    visuals.widgets.hovered.fg_stroke.color = TEXT;
    visuals.widgets.active.fg_stroke.color = TEXT;
    visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(0x24, 0x28, 0x30);
    visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(0x2E, 0x34, 0x3E);
    visuals.widgets.active.bg_fill = egui::Color32::from_rgb(0x2B, 0xB8, 0xA8).gamma_multiply(0.35);
    visuals.selection.bg_fill = TEAL.gamma_multiply(0.35);
    visuals.widgets.noninteractive.bg_stroke.color = egui::Color32::from_rgb(0x2E, 0x34, 0x3E);
    visuals.window_corner_radius = 12.0.into();
    visuals.menu_corner_radius = 8.0.into();
    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(10.0, 8.0);
    style.spacing.button_padding = egui::vec2(12.0, 6.0);
    ctx.set_style(style);
}
