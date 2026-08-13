use bevy_egui::egui;
use crate::common::core::performance::{CloudQuality, GraphicsSettings, MsaaQuality, ShadowFilter, ShadowQuality, ViewDistance, VsyncMode};
use crate::studio::ui::resources::{SettingsWindow, StudioSettings, VrtxSaveSettings};

#[derive(PartialEq, Clone, Copy, Default)]
pub enum SettingsTab {
    #[default]
    Graphics,
    Vrtx,
    Studio,
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

fn tab_button(ui: &mut egui::Ui, selected: &mut SettingsTab, tab: SettingsTab, label: &str) {
    let response = ui.selectable_label(*selected == tab, label);
    if response.clicked() {
        *selected = tab;
    }
}

fn draw_tab_bar(ui: &mut egui::Ui, selected: &mut SettingsTab) {
    ui.horizontal(|ui| {
        tab_button(ui, selected, SettingsTab::Graphics, "Graphics");
        tab_button(ui, selected, SettingsTab::Vrtx, "VRTX");
        tab_button(ui, selected, SettingsTab::Studio, "Studio");
    });
    ui.separator();
}

fn draw_graphics_tab(ui: &mut egui::Ui, graphics_settings: &mut GraphicsSettings) {
    ui.label(egui::RichText::new("Display").strong().size(13.0));
    ui.add_space(4.0);

    ui.label("V-Sync");
    egui::ComboBox::from_id_salt("vsync_mode")
        .selected_text(vsync_label(graphics_settings.vsync))
        .width(140.0)
        .show_ui(ui, |ui| {
            for mode in [VsyncMode::Off, VsyncMode::On, VsyncMode::Adaptive] {
                ui.selectable_value(&mut graphics_settings.vsync, mode, vsync_label(mode));
            }
        });

    ui.add_space(8.0);
    ui.label(egui::RichText::new("Anti-Aliasing").strong().size(13.0));
    ui.add_space(4.0);

    ui.label("MSAA");
    egui::ComboBox::from_id_salt("msaa_quality")
        .selected_text(msaa_label(graphics_settings.msaa))
        .width(140.0)
        .show_ui(ui, |ui| {
            for quality in [MsaaQuality::Off, MsaaQuality::Sample2, MsaaQuality::Sample4, MsaaQuality::Sample8] {
                ui.selectable_value(&mut graphics_settings.msaa, quality, msaa_label(quality));
            }
        });

    ui.add_space(8.0);
    ui.label(egui::RichText::new("Shadows").strong().size(13.0));
    ui.add_space(4.0);

    ui.label("Quality");
    egui::ComboBox::from_id_salt("shadow_quality")
        .selected_text(shadow_quality_label(graphics_settings.shadow_quality))
        .width(140.0)
        .show_ui(ui, |ui| {
            for quality in [ShadowQuality::Off, ShadowQuality::Low, ShadowQuality::Medium, ShadowQuality::High] {
                ui.selectable_value(&mut graphics_settings.shadow_quality, quality, shadow_quality_label(quality));
            }
        });

    ui.add_space(4.0);
    ui.label("Filtering");
    egui::ComboBox::from_id_salt("shadow_filter")
        .selected_text(shadow_filter_label(graphics_settings.shadow_filter))
        .width(140.0)
        .show_ui(ui, |ui| {
            for filter in [ShadowFilter::Fast, ShadowFilter::HighQuality] {
                ui.selectable_value(&mut graphics_settings.shadow_filter, filter, shadow_filter_label(filter));
            }
        });

    ui.add_space(8.0);
    ui.label(egui::RichText::new("World Detail").strong().size(13.0));
    ui.add_space(4.0);

    ui.label("View Distance");
    egui::ComboBox::from_id_salt("view_distance")
        .selected_text(view_distance_label(graphics_settings.view_distance))
        .width(140.0)
        .show_ui(ui, |ui| {
            for distance in [ViewDistance::Low, ViewDistance::Medium, ViewDistance::High] {
                ui.selectable_value(&mut graphics_settings.view_distance, distance, view_distance_label(distance));
            }
        });

    ui.add_space(4.0);
    ui.label("Cloud Quality");
    egui::ComboBox::from_id_salt("cloud_quality")
        .selected_text(cloud_quality_label(graphics_settings.cloud_quality))
        .width(140.0)
        .show_ui(ui, |ui| {
            for quality in [CloudQuality::Off, CloudQuality::Low, CloudQuality::Medium, CloudQuality::High] {
                ui.selectable_value(&mut graphics_settings.cloud_quality, quality, cloud_quality_label(quality));
            }
        });

    ui.add_space(8.0);
    ui.label(egui::RichText::new("Effects").strong().size(13.0));
    ui.add_space(4.0);

    ui.checkbox(&mut graphics_settings.ssao, "Screen Space Ambient Occlusion (SSAO)");
    ui.checkbox(&mut graphics_settings.contact_shadows, "Contact Shadows");
    ui.checkbox(&mut graphics_settings.bloom, "Bloom");

    ui.add_space(8.0);
    ui.label(egui::RichText::new("Note: Disabling SSAO, Contact Shadows and lowering Quality is highly recommended on Integrated Graphics to minimize GPU usage.")
        .size(11.0)
        .color(egui::Color32::from_rgb(100, 100, 100)));
}

fn draw_vrtx_tab(ui: &mut egui::Ui, vrtx_save_settings: &mut VrtxSaveSettings) {
    ui.label(egui::RichText::new("VRTX Save Options").strong().size(13.0));
    ui.add_space(4.0);

    ui.checkbox(&mut vrtx_save_settings.include_camera_position, "Include Camera Position");
    ui.checkbox(&mut vrtx_save_settings.include_graphics_settings, "Include Graphics Settings (SSAO, Contact Shadows, Bloom)");
    ui.checkbox(&mut vrtx_save_settings.include_lighting, "Include Lighting (Sky, Clouds, Fog)");

    ui.add_space(8.0);
    ui.label(egui::RichText::new("Controls what is written when saving a .vrtx map file.")
        .size(11.0)
        .color(egui::Color32::from_rgb(100, 100, 100)));
}

fn draw_studio_tab(ui: &mut egui::Ui, studio_settings: &mut StudioSettings) {
    ui.label(egui::RichText::new("Camera").strong().size(13.0));
    ui.add_space(4.0);

    ui.add(egui::Slider::new(&mut studio_settings.mouse_sensitivity, 0.0..=1.0).text("Mouse Sensitivity"));
    ui.add(egui::Slider::new(&mut studio_settings.camera_speed, 0.5..=50.0).text("Camera Speed"));
    ui.add(egui::Slider::new(&mut studio_settings.fov, 10.0..=120.0).text("Camera FOV"));
}

pub fn draw_settings_window(
    ctx: &egui::Context,
    window: &mut SettingsWindow,
    graphics_settings: &mut GraphicsSettings,
    vrtx_save_settings: &mut VrtxSaveSettings,
    studio_settings: &mut StudioSettings,
) {
    egui::Window::new("Settings")
        .open(&mut window.open)
        .pivot(egui::Align2::CENTER_CENTER)
        .default_pos(ctx.content_rect().center())
        .default_size(egui::vec2(320.0, 520.0))
        .resizable(true)
        .collapsible(false)
        .show(ctx, |ui| {
            draw_tab_bar(ui, &mut window.tab);
            ui.add_space(8.0);

            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    match window.tab {
                        SettingsTab::Graphics => draw_graphics_tab(ui, graphics_settings),
                        SettingsTab::Vrtx => draw_vrtx_tab(ui, vrtx_save_settings),
                        SettingsTab::Studio => draw_studio_tab(ui, studio_settings),
                    }
                });
        });
}
