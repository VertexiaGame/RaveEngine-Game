use bevy::prelude::*;
use bevy_egui::egui;

#[derive(Resource, Default)]
pub struct CameraSpeedIndicator {
    pub visible_timer: f32,
    pub current_speed: f32,
}

#[derive(Resource, Default)]
pub struct FovIndicator {
    pub visible_timer: f32,
    pub current_fov: f32,
    pub interacting: bool,
}

pub fn updatecameraspeedindicator(
    keys: Res<ButtonInput<KeyCode>>,
    mut indicator: ResMut<CameraSpeedIndicator>,
    cameraquery: Query<(
        &bevy::camera_controller::free_camera::FreeCamera,
        &bevy::camera_controller::free_camera::FreeCameraState,
    )>,
    mut scroll: MessageReader<bevy::input::mouse::MouseWheel>,
    time: Res<Time>,
) {
    let ctrl_pressed = keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);

    let mut scrolled = false;
    for _ in scroll.read() {
        scrolled = true;
    }

    if scrolled && !ctrl_pressed {
        if let Some((free_camera, free_camera_state)) = cameraquery.iter().next() {
            indicator.current_speed = free_camera.walk_speed * free_camera_state.speed_multiplier;
            indicator.visible_timer = 2.0;
        }
    } else if indicator.visible_timer > 0.0 {
        indicator.visible_timer -= time.delta_secs();
        if let Some((free_camera, free_camera_state)) = cameraquery.iter().next() {
            indicator.current_speed = free_camera.walk_speed * free_camera_state.speed_multiplier;
        }
    }
}

pub fn update_camera_fov(
    keys: Res<ButtonInput<KeyCode>>,
    mut indicator: ResMut<FovIndicator>,
    mut camera_query: Query<(
        &mut bevy::camera_controller::free_camera::FreeCamera,
        &mut Projection,
    ), With<Camera3d>>,
    mut scroll: MessageReader<bevy::input::mouse::MouseWheel>,
    time: Res<Time>,
) {
    let ctrl_pressed = keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);

    let mut scroll_val = 0.0;
    for ev in scroll.read() {
        scroll_val += ev.y;
    }

    if let Ok((mut free_camera, mut projection)) = camera_query.single_mut() {
        if ctrl_pressed {
            free_camera.scroll_factor = 0.0;
            if scroll_val != 0.0 {
                if let Projection::Perspective(ref mut perspective) = *projection {
                    let mut fov_deg = perspective.fov.to_degrees();
                    fov_deg = (fov_deg - scroll_val * 2.0).clamp(10.0, 120.0);
                    perspective.fov = fov_deg.to_radians();
                    indicator.current_fov = fov_deg;
                    indicator.visible_timer = 2.0;
                }
            }
        } else {
            free_camera.scroll_factor = 0.1;
        }
    }

    if indicator.visible_timer > 0.0 {
        indicator.visible_timer -= time.delta_secs();
        if !indicator.interacting {
            if let Ok((_, projection)) = camera_query.single() {
                if let Projection::Perspective(perspective) = projection {
                    indicator.current_fov = perspective.fov.to_degrees();
                }
            }
        }
    }
}

pub fn draw_indicator(
    ctx: &egui::Context,
    cameraindicator: &mut CameraSpeedIndicator,
    cameraquery: &mut Query<(
        &bevy::camera_controller::free_camera::FreeCamera,
        &mut bevy::camera_controller::free_camera::FreeCameraState,
    )>,
) {
    if cameraindicator.visible_timer > 0.0 {
        let alphafactor = if cameraindicator.visible_timer < 1.0 {
            cameraindicator.visible_timer.clamp(0.0, 1.0)
        } else {
            1.0
        };

        let mut innerhovered = false;
        let mut slideractive = false;
        let arearesponse = egui::Area::new(egui::Id::new("camera_speed_indicator"))
            .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -40.0))
            .show(ctx, |ui| {
                ui.set_opacity(alphafactor);
                let frameres = egui::Frame::new()
                    .fill(egui::Color32::from_rgba_unmultiplied(240, 240, 240, 230))
                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(180, 180, 180)))
                    .corner_radius(6.0)
                    .inner_margin(egui::Margin::symmetric(16, 8))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let mut speed = cameraindicator.current_speed;
                            let sliderres = ui.add(
                                egui::Slider::new(&mut speed, 0.1..=100.0)
                                    .text("Camera Speed")
                            );
                            if sliderres.changed() {
                                if let Some((free_camera, mut free_camera_state)) = cameraquery.iter_mut().next() {
                                    free_camera_state.speed_multiplier = speed / free_camera.walk_speed;
                                    cameraindicator.current_speed = speed;
                                }
                            }
                            slideractive = sliderres.dragged() || sliderres.has_focus() || sliderres.hovered();
                        });
                    });

                if let Some(pos) = ctx.input(|i| i.pointer.latest_pos()) {
                    if frameres.response.rect.contains(pos) {
                        innerhovered = true;
                    }
                }
            });

        if arearesponse.response.hovered() || innerhovered || slideractive {
            cameraindicator.visible_timer = 2.0;
        }
    }
}

pub fn draw_fov_indicator(
    ctx: &egui::Context,
    fov_indicator: &mut FovIndicator,
    camera_query: &mut Query<&mut Projection, With<Camera3d>>,
) {
    if fov_indicator.visible_timer > 0.0 {
        let alphafactor = if fov_indicator.visible_timer < 1.0 {
            fov_indicator.visible_timer.clamp(0.0, 1.0)
        } else {
            1.0
        };

        let mut innerhovered = false;
        let mut slideractive = false;
        let arearesponse = egui::Area::new(egui::Id::new("camera_fov_indicator"))
            .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -40.0))
            .show(ctx, |ui| {
                ui.set_opacity(alphafactor);
                let frameres = egui::Frame::new()
                    .fill(egui::Color32::from_rgba_unmultiplied(240, 240, 240, 230))
                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(180, 180, 180)))
                    .corner_radius(6.0)
                    .inner_margin(egui::Margin::symmetric(16, 8))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let mut fov = fov_indicator.current_fov;
                            let sliderres = ui.add(
                                egui::Slider::new(&mut fov, 10.0..=120.0)
                                    .text("Camera FOV")
                            );
                            if sliderres.changed() {
                                if let Ok(mut projection) = camera_query.single_mut() {
                                    if let Projection::Perspective(ref mut perspective) = *projection {
                                        perspective.fov = fov.to_radians();
                                        fov_indicator.current_fov = fov;
                                    }
                                }
                            }
                            slideractive = sliderres.dragged() || sliderres.has_focus() || sliderres.hovered();
                        });
                    });

                if let Some(pos) = ctx.input(|i| i.pointer.latest_pos()) {
                    if frameres.response.rect.contains(pos) {
                        innerhovered = true;
                    }
                }
            });

        if arearesponse.response.hovered() || innerhovered || slideractive {
            fov_indicator.visible_timer = 2.0;
        }
        fov_indicator.interacting = slideractive;
    }
}