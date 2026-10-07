use crate::common::core::performance::{
    CloudQuality, GraphicsSettings, MsaaQuality, ShadowFilter, ShadowQuality, ViewDistance,
    VsyncMode,
};
use crate::studio::ui::resources::{SettingsWindow, StudioSettings, VrtxSaveSettings};
use bevy_egui::egui;

#[derive(PartialEq, Clone, Copy, Default)]
pub enum SettingsTab {
    #[default]
    Graphics,
    Vrtx,
    Studio,
    Account,
}

struct GraphicsPreset {
    name: &'static str,
    tooltip: &'static str,
    msaa: MsaaQuality,
    shadow_quality: ShadowQuality,
    shadow_filter: ShadowFilter,
    view_distance: ViewDistance,
    cloud_quality: CloudQuality,
    ssao: bool,
    contact_shadows: bool,
    bloom: bool,
}

const QUALITY_PRESETS: [GraphicsPreset; 3] = [
    GraphicsPreset {
        name: "Low",
        tooltip: "Integrated graphics / low-end: disables heavy effects, short draw distance.",
        msaa: MsaaQuality::Off,
        shadow_quality: ShadowQuality::Off,
        shadow_filter: ShadowFilter::Fast,
        view_distance: ViewDistance::Low,
        cloud_quality: CloudQuality::Off,
        ssao: false,
        contact_shadows: false,
        bloom: false,
    },
    GraphicsPreset {
        name: "Medium",
        tooltip: "Balanced quality and performance.",
        msaa: MsaaQuality::Sample2,
        shadow_quality: ShadowQuality::Medium,
        shadow_filter: ShadowFilter::Fast,
        view_distance: ViewDistance::Medium,
        cloud_quality: CloudQuality::Medium,
        ssao: false,
        contact_shadows: false,
        bloom: true,
    },
    GraphicsPreset {
        name: "High",
        tooltip: "Best visual quality — suited for dedicated GPUs.",
        msaa: MsaaQuality::Sample4,
        shadow_quality: ShadowQuality::High,
        shadow_filter: ShadowFilter::HighQuality,
        view_distance: ViewDistance::High,
        cloud_quality: CloudQuality::High,
        ssao: true,
        contact_shadows: true,
        bloom: true,
    },
];

impl GraphicsPreset {
    fn matches(&self, g: &GraphicsSettings) -> bool {
        self.msaa == g.msaa
            && self.shadow_quality == g.shadow_quality
            && self.shadow_filter == g.shadow_filter
            && self.view_distance == g.view_distance
            && self.cloud_quality == g.cloud_quality
            && self.ssao == g.ssao
            && self.contact_shadows == g.contact_shadows
            && self.bloom == g.bloom
    }

    fn apply(&self, g: &mut GraphicsSettings) {
        g.msaa = self.msaa;
        g.shadow_quality = self.shadow_quality;
        g.shadow_filter = self.shadow_filter;
        g.view_distance = self.view_distance;
        g.cloud_quality = self.cloud_quality;
        g.ssao = self.ssao;
        g.contact_shadows = self.contact_shadows;
        g.bloom = self.bloom;
    }
}

fn muted(text: impl Into<String>) -> egui::RichText {
    egui::RichText::new(text.into())
        .size(11.0)
        .color(egui::Color32::from_rgb(110, 110, 110))
}

fn section_title(ui: &mut egui::Ui, title: &str) {
    ui.add_space(6.0);
    ui.label(egui::RichText::new(title).strong().size(13.0));
    ui.add_space(2.0);
}

fn msaa_label(quality: MsaaQuality) -> &'static str {
    match quality {
        MsaaQuality::Off => "Off",
        MsaaQuality::Sample2 => "2x",
        MsaaQuality::Sample4 => "4x",
        MsaaQuality::Sample8 => "8x",
    }
}

fn shadow_quality_label(quality: ShadowQuality) -> &'static str {
    match quality {
        ShadowQuality::Off => "Off",
        ShadowQuality::Low => "Low",
        ShadowQuality::Medium => "Medium",
        ShadowQuality::High => "High",
    }
}

fn view_distance_label(distance: ViewDistance) -> &'static str {
    match distance {
        ViewDistance::Low => "Low",
        ViewDistance::Medium => "Medium",
        ViewDistance::High => "High",
    }
}

fn vsync_label(mode: VsyncMode) -> &'static str {
    match mode {
        VsyncMode::Off => "Off",
        VsyncMode::On => "On",
        VsyncMode::Adaptive => "Adaptive",
    }
}

fn shadow_filter_label(filter: ShadowFilter) -> &'static str {
    match filter {
        ShadowFilter::Fast => "Fast (Hardware)",
        ShadowFilter::HighQuality => "High Quality",
    }
}

fn cloud_quality_label(quality: CloudQuality) -> &'static str {
    match quality {
        CloudQuality::Off => "Off",
        CloudQuality::Low => "Low",
        CloudQuality::Medium => "Medium",
        CloudQuality::High => "High",
    }
}

fn setting_combo<T: PartialEq + Copy>(
    ui: &mut egui::Ui,
    id: &str,
    value: &mut T,
    options: &[T],
    to_label: fn(T) -> &'static str,
) {
    egui::ComboBox::from_id_salt(id)
        .selected_text(to_label(*value))
        .width(140.0)
        .show_ui(ui, |ui| {
            for &opt in options {
                ui.selectable_value(value, opt, to_label(opt));
            }
        });
}

fn tab_button(ui: &mut egui::Ui, selected: &mut SettingsTab, tab: SettingsTab, label: &str) {
    let response = ui.selectable_label(*selected == tab, label);
    if response.clicked() {
        *selected = tab;
    }
}

fn draw_graphics_tab(ui: &mut egui::Ui, graphics_settings: &mut GraphicsSettings) {
    let active_preset_name: Option<&'static str> = QUALITY_PRESETS
        .iter()
        .find(|p| p.matches(graphics_settings))
        .map(|p| p.name);

    ui.label(egui::RichText::new("Quality Preset").strong().size(13.0));
    ui.horizontal(|ui| {
        for preset in &QUALITY_PRESETS {
            if ui
                .button(preset.name)
                .on_hover_text(preset.tooltip)
                .clicked()
            {
                preset.apply(graphics_settings);
            }
        }
    });
    ui.label(muted(format!(
        "Current configuration: {}",
        active_preset_name.unwrap_or("Custom")
    )));

    section_title(ui, "Display");
    egui::Grid::new("settings_display_grid")
        .num_columns(2)
        .spacing([12.0, 8.0])
        .show(ui, |ui| {
            ui.label("V-Sync");
            setting_combo(
                ui,
                "vsync_mode",
                &mut graphics_settings.vsync,
                &[VsyncMode::Off, VsyncMode::On, VsyncMode::Adaptive],
                vsync_label,
            );
            ui.end_row();
        });

    section_title(ui, "Anti-Aliasing");
    egui::Grid::new("settings_aa_grid")
        .num_columns(2)
        .spacing([12.0, 8.0])
        .show(ui, |ui| {
            ui.label("MSAA");
            setting_combo(
                ui,
                "msaa_quality",
                &mut graphics_settings.msaa,
                &[
                    MsaaQuality::Off,
                    MsaaQuality::Sample2,
                    MsaaQuality::Sample4,
                    MsaaQuality::Sample8,
                ],
                msaa_label,
            );
            ui.end_row();
        });

    section_title(ui, "Shadows");
    egui::Grid::new("settings_shadow_grid")
        .num_columns(2)
        .spacing([12.0, 8.0])
        .show(ui, |ui| {
            ui.label("Quality");
            setting_combo(
                ui,
                "shadow_quality",
                &mut graphics_settings.shadow_quality,
                &[
                    ShadowQuality::Off,
                    ShadowQuality::Low,
                    ShadowQuality::Medium,
                    ShadowQuality::High,
                ],
                shadow_quality_label,
            );
            ui.end_row();

            ui.label("Filtering");
            setting_combo(
                ui,
                "shadow_filter",
                &mut graphics_settings.shadow_filter,
                &[ShadowFilter::Fast, ShadowFilter::HighQuality],
                shadow_filter_label,
            );
            ui.end_row();
        });

    section_title(ui, "World Detail");
    egui::Grid::new("settings_world_grid")
        .num_columns(2)
        .spacing([12.0, 8.0])
        .show(ui, |ui| {
            ui.label("View Distance");
            setting_combo(
                ui,
                "view_distance",
                &mut graphics_settings.view_distance,
                &[ViewDistance::Low, ViewDistance::Medium, ViewDistance::High],
                view_distance_label,
            );
            ui.end_row();

            ui.label("Clouds");
            setting_combo(
                ui,
                "cloud_quality",
                &mut graphics_settings.cloud_quality,
                &[
                    CloudQuality::Off,
                    CloudQuality::Low,
                    CloudQuality::Medium,
                    CloudQuality::High,
                ],
                cloud_quality_label,
            );
            ui.end_row();
        });

    section_title(ui, "Effects");
    ui.checkbox(
        &mut graphics_settings.ssao,
        "Screen Space Ambient Occlusion (SSAO)",
    )
    .on_hover_text("Depth shading where surfaces meet.");
    ui.checkbox(&mut graphics_settings.contact_shadows, "Contact Shadows")
        .on_hover_text("Tight shadows where objects touch surfaces.");
    ui.checkbox(&mut graphics_settings.bloom, "Bloom")
        .on_hover_text("Soft glow around bright areas.");

    ui.add_space(8.0);
    ui.label(muted(
        "Tip: Disabling SSAO, Contact Shadows and lowering quality is highly recommended on integrated graphics to minimize GPU usage.",
    ));
}

fn draw_vrtx_tab(ui: &mut egui::Ui, vrtx_save_settings: &mut VrtxSaveSettings) {
    ui.label(egui::RichText::new("VRTX Save Options").strong().size(13.0));
    ui.checkbox(
        &mut vrtx_save_settings.include_camera_position,
        "Include Camera Position",
    )
    .on_hover_text("Store the editor camera transform.");
    ui.checkbox(
        &mut vrtx_save_settings.include_graphics_settings,
        "Include Graphics Settings (SSAO, Contact Shadows, Bloom)",
    );

    ui.add_space(8.0);
    ui.label(muted(
        "Controls what gets written when saving a .vrtx map file.",
    ));
}

fn draw_studio_tab(ui: &mut egui::Ui, studio_settings: &mut StudioSettings) {
    ui.label(egui::RichText::new("Camera").strong().size(13.0));
    ui.add(
        egui::Slider::new(&mut studio_settings.mouse_sensitivity, 0.0..=1.0)
            .text("Mouse Sensitivity")
            .custom_formatter(|v, _| format!("{:.0}%", v * 100.0)),
    );
    ui.add(
        egui::Slider::new(&mut studio_settings.camera_speed, 0.5..=50.0).text("Camera Speed"),
    );
    ui.add(
        egui::Slider::new(&mut studio_settings.fov, 10.0..=120.0)
            .text("Field of View")
            .suffix("°"),
    );
}

fn draw_account_tab(
    ui: &mut egui::Ui,
    auth_flow: &mut crate::studio::auth::StudioAuthFlow,
    auth_store: &mut crate::studio::auth::StudioAuthStore,
) {
    if let Some(creds) = auth_store.credentials.clone() {
        ui.label(
            egui::RichText::new(format!("Logged in as {} (ID {})", creds.username, creds.uid))
                .strong()
                .size(13.0),
        );
        ui.add_space(4.0);
        if ui.button("Logout").clicked() {
            crate::studio::auth::clear_studio_credentials();
            auth_store.credentials = None;
            auth_flow.state = None;
            auth_flow.redirect_uri = None;
            auth_flow.receiver = None;
            auth_flow.error = None;
            auth_flow.waiting_since = None;
            auth_flow.is_polling = false;
        }
    } else if auth_flow.state.is_some() {
        ui.horizontal(|ui| {
            ui.add(egui::Spinner::new().size(16.0));
            ui.label("Waiting for browser authentication...");
        });
        ui.label(muted("Complete login in your browser, then return here."));
        ui.add_space(4.0);
        if ui.button("Cancel").clicked() {
            auth_flow.state = None;
            auth_flow.redirect_uri = None;
            auth_flow.receiver = None;
            auth_flow.error = None;
            auth_flow.waiting_since = None;
            auth_flow.is_polling = false;
        }
    } else {
        ui.label(egui::RichText::new("VERTEXIA Account").strong().size(13.0));
        ui.label(muted("Login to save your game and publish it to the website."));
        ui.add_space(4.0);
        if ui.button("Login to VERTEXIA").clicked() {
            if let Err(e) = crate::studio::auth::start_studio_auth_flow(auth_flow) {
                auth_flow.error = Some(e);
            }
        }
        if let Some(err) = auth_flow.error.clone() {
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(err)
                    .size(11.0)
                    .color(egui::Color32::from_rgb(200, 40, 40)),
            );
        }
    }
}

pub fn draw_settings_window(
    ctx: &egui::Context,
    window: &mut SettingsWindow,
    graphics_settings: &mut GraphicsSettings,
    vrtx_save_settings: &mut VrtxSaveSettings,
    studio_settings: &mut StudioSettings,
    auth_flow: &mut crate::studio::auth::StudioAuthFlow,
    auth_store: &mut crate::studio::auth::StudioAuthStore,
) {
    egui::Window::new("Settings")
        .open(&mut window.open)
        .pivot(egui::Align2::CENTER_CENTER)
        .default_pos(ctx.content_rect().center())
        .default_size(egui::vec2(420.0, 520.0))
        .resizable(true)
        .collapsible(false)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                tab_button(ui, &mut window.tab, SettingsTab::Graphics, "Graphics");
                tab_button(ui, &mut window.tab, SettingsTab::Vrtx, "VRTX Save");
                tab_button(ui, &mut window.tab, SettingsTab::Studio, "Studio");
                tab_button(ui, &mut window.tab, SettingsTab::Account, "Account");
            });
            ui.separator();
            ui.add_space(4.0);

            egui::ScrollArea::vertical()
                .id_salt("settings_content_scroll")
                .max_height(ui.available_height() - 36.0)
                .show(ui, |ui| match window.tab {
                    SettingsTab::Graphics => draw_graphics_tab(ui, graphics_settings),
                    SettingsTab::Vrtx => draw_vrtx_tab(ui, vrtx_save_settings),
                    SettingsTab::Studio => draw_studio_tab(ui, studio_settings),
                    SettingsTab::Account => draw_account_tab(ui, auth_flow, auth_store),
                });

            ui.separator();
            if ui
                .button("Reset to Defaults")
                .on_hover_text("Restore this page's settings to their defaults.")
                .clicked()
            {
                match window.tab {
                    SettingsTab::Graphics => *graphics_settings = GraphicsSettings::default(),
                    SettingsTab::Vrtx => *vrtx_save_settings = VrtxSaveSettings::default(),
                    SettingsTab::Studio => *studio_settings = StudioSettings::default(),
                    SettingsTab::Account => {}
                }
            }
        });
}
