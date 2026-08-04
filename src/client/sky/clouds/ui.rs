use bevy::prelude::*;
use bevy_egui::{
    egui::{self, Pos2, Ui},
    EguiContexts,
};

use super::config::CloudsConfig;

pub fn clouds_ui(config: &mut CloudsConfig, ui: &mut Ui) {
    ui.add(egui::Slider::new(&mut config.clouds_raymarch_steps_count, 1..=100).text("March steps"));
    ui.end_row();
    ui.add(
        egui::Slider::new(&mut config.clouds_shadow_raymarch_steps_count, 1..=50)
            .text("Self shadow steps"),
    );
    ui.end_row();
    ui.add(egui::Slider::new(&mut config.planet_radius, 5e4..=1e7).text("Planet radius"));
    ui.end_row();
    ui.add(
        egui::Slider::new(&mut config.clouds_bottom_height, 1.0..=5e3).text("clouds_bottom_height"),
    );
    ui.end_row();
    ui.add(egui::Slider::new(&mut config.clouds_top_height, 1.0..=5e3).text("clouds_top_height"));
    ui.end_row();
    ui.add(egui::Slider::new(&mut config.clouds_coverage, 0.0..=1.0).text("clouds_coverage"));
    ui.end_row();
    ui.add(
        egui::Slider::new(&mut config.clouds_detail_strength, 0.0..=1.0)
            .text("clouds_detail_strength"),
    );
    ui.end_row();
    ui.add(
        egui::Slider::new(&mut config.clouds_base_edge_softness, 0.0..=1.0)
            .text("clouds_base_edge_softness"),
    );
    ui.end_row();
    ui.add(
        egui::Slider::new(&mut config.clouds_bottom_softness, 0.01..=10.0)
            .text("clouds_bottom_softness"),
    );
    ui.end_row();
    ui.add(egui::Slider::new(&mut config.clouds_density, 0.001..=1.0).text("clouds_density"));
    ui.end_row();
    ui.add(
        egui::Slider::new(&mut config.clouds_shadow_raymarch_step_size, 1.0..=100.0)
            .text("clouds_shadow_raymarch_step_size"),
    );
    ui.end_row();
    ui.add(
        egui::Slider::new(&mut config.clouds_shadow_raymarch_step_multiply, 0.1..=10.0)
            .text("clouds_shadow_raymarch_step_multiply"),
    );
    ui.end_row();
    ui.add(
        egui::Slider::new(&mut config.forward_scattering_g, -10.0..=10.0)
            .text("forward_scattering_g"),
    );
    ui.end_row();
    ui.add(
        egui::Slider::new(&mut config.backward_scattering_g, -10.0..=10.0)
            .text("backward_scattering_g"),
    );
    ui.end_row();
    ui.add(egui::Slider::new(&mut config.scattering_lerp, 0.01..=100.0).text("Scattering lerp"));
    ui.end_row();
    ui.add(
        egui::Slider::new(&mut config.clouds_min_transmittance, 0.01..=100.0)
            .text("Min transmittance"),
    );
    ui.end_row();
    ui.add(egui::Slider::new(&mut config.clouds_base_scale, 0.1..=100.0).text("Base scale"));
    ui.end_row();
    ui.add(egui::Slider::new(&mut config.clouds_detail_scale, 1.0..=100.0).text("Detail scale"));
    ui.end_row();
    ui.add(
        egui::Slider::new(&mut config.reprojection_strength, 0.0..=1.0)
            .text("reprojection_strength"),
    );
    ui.end_row();
    ui.add(egui::Label::new("wind_velocity"));
    ui.end_row();
    ui.add(egui::Slider::new(&mut config.wind_velocity.x, -100.0..=100.0).text("x"));
    ui.end_row();
    ui.add(egui::Slider::new(&mut config.wind_velocity.y, -100.0..=100.0).text("y"));
    ui.end_row();
    ui.add(egui::Slider::new(&mut config.wind_velocity.z, -100.0..=100.0).text("z"));
    ui.end_row();

    if ui.button("Reset to defaults").clicked() {
        *config = CloudsConfig::default();
    };
}

pub fn ui_system(
    mut clouds_config: ResMut<CloudsConfig>,
    mut contexts: EguiContexts,
    keys: Res<ButtonInput<KeyCode>>,
) {
    if keys.just_pressed(KeyCode::KeyY) {
        clouds_config.ui_visible = !clouds_config.ui_visible;
    }

    if clouds_config.ui_visible {
        if let Ok(ctx) = contexts.ctx_mut() {
            egui::Window::new("Clouds configuration")
                .current_pos(Pos2 { x: 10.0, y: 320.0 })
                .show(ctx, |ui| {
                    egui::Grid::new("3dworld_grid")
                        .num_columns(2)
                        .spacing([40.0, 4.0])
                        .striped(true)
                        .show(ui, |ui| {
                            clouds_ui(clouds_config.as_mut(), ui);
                        });
                });
        }
    }
}