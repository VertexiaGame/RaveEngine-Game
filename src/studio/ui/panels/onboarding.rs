use bevy::prelude::*;
use bevy_egui::egui;
use bevy::pbr::ExtendedMaterial;
use crate::common::game::bricks::data::spawn_brick;
use avian3d::prelude::CollisionLayers;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SelectedTemplate {
    #[default]
    Empty,
    Baseplate,
}

#[derive(Resource, Debug, Clone, PartialEq, Eq)]
pub struct OnboardingData {
    pub selected_template: SelectedTemplate,
    pub name: String,
    pub description: String,
    pub save_path: String,
}

impl Default for OnboardingData {
    fn default() -> Self {
        let save_path = std::env::current_dir()
            .map(|p| p.join("NewProject.rave").to_string_lossy().to_string())
            .unwrap_or_else(|_| "NewProject.rave".to_string());
        Self {
            selected_template: SelectedTemplate::Empty,
            name: "New Project".to_string(),
            description: "".to_string(),
            save_path,
        }
    }
}

pub fn draw_onboarding(
    ctx: &egui::Context,
    next_onboarding_state: &mut ResMut<NextState<crate::studio::tools::OnboardingState>>,
    onboarding_state: &State<crate::studio::tools::OnboardingState>,
    onboarding_data: &mut OnboardingData,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    studs_materials: &mut Assets<ExtendedMaterial<StandardMaterial, crate::common::game::bricks::studs::StudsExtension>>,
    studs_assets: &crate::common::game::bricks::studs::StudsAssets,
    count: &mut crate::common::game::bricks::data::BrickSpawnerCount,
    thumb_empty_tex: egui::TextureId,
    thumb_baseplate_tex: egui::TextureId,
) {
    #[allow(deprecated)]
    let center = ctx.available_rect().center();

    egui::Window::new("Welcome to VERTEXIA")
        .pivot(egui::Align2::CENTER_CENTER)
        .fixed_pos(center)
        .resizable(false)
        .collapsible(false)
        .title_bar(false)
        .frame(egui::Frame::NONE
            .fill(egui::Color32::from_rgb(255, 255, 255))
            .corner_radius(8.0)
            .inner_margin(egui::Margin::same(24))
            .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(212, 212, 212)))
        )
        .show(ctx, |ui| {
            ui.set_max_width(550.0);
            match onboarding_state.get() {
                crate::studio::tools::OnboardingState::TemplateSelection => {
                    ui.vertical_centered(|ui| {
                        ui.label(egui::RichText::new("Welcome!").size(28.0).strong());
                        ui.add_space(8.0);
                        ui.label(egui::RichText::new("Welcome to the VERTEXIA studio. Pick a template to start off, or start with an empty place.")
                            .size(14.0));
                    });
                    ui.add_space(24.0);

                    ui.columns(2, |columns| {
                        columns[0].vertical_centered(|ui| {
                            let card_id = ui.make_persistent_id("empty_card");
                            let is_hovered = ui.rect_contains_pointer(ui.max_rect());

                            let frame = egui::Frame::NONE
                                .fill(if is_hovered { egui::Color32::from_rgb(235, 242, 252) } else { egui::Color32::from_rgb(245, 246, 247) })
                                .corner_radius(12.0)
                                .stroke(egui::Stroke::new(1.0, if is_hovered { egui::Color32::from_rgb(80, 160, 240) } else { egui::Color32::from_rgb(220, 220, 220) }))
                                .inner_margin(egui::Margin::same(16));

                            let inner_res = frame.show(ui, |ui| {
                                ui.set_min_height(250.0);
                                ui.add_space(8.0);
                                ui.add(egui::Image::new((thumb_empty_tex, egui::vec2(220.0, 137.5))).corner_radius(12.0));
                                ui.add_space(12.0);
                                ui.label(egui::RichText::new("Empty").size(18.0).strong());
                                ui.add_space(4.0);
                                ui.label(egui::RichText::new("An empty world. The sky's the limit.")
                                    .size(12.0));
                            });

                            let response = ui.interact(inner_res.response.rect, card_id, egui::Sense::click());
                            if response.hovered() {
                                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                            }
                            if response.clicked() {
                                onboarding_data.selected_template = SelectedTemplate::Empty;
                                next_onboarding_state.set(crate::studio::tools::OnboardingState::BasicInfo);
                            }
                        });

                        columns[1].vertical_centered(|ui| {
                            let card_id = ui.make_persistent_id("baseplate_card");
                            let is_hovered = ui.rect_contains_pointer(ui.max_rect());

                            let frame = egui::Frame::NONE
                                .fill(if is_hovered { egui::Color32::from_rgb(235, 242, 252) } else { egui::Color32::from_rgb(245, 246, 247) })
                                .corner_radius(12.0)
                                .stroke(egui::Stroke::new(1.0, if is_hovered { egui::Color32::from_rgb(80, 160, 240) } else { egui::Color32::from_rgb(220, 220, 220) }))
                                .inner_margin(egui::Margin::same(16));

                            let inner_res = frame.show(ui, |ui| {
                                ui.set_min_height(250.0);
                                ui.add_space(8.0);
                                ui.add(egui::Image::new((thumb_baseplate_tex, egui::vec2(220.0, 137.5))).corner_radius(12.0));
                                ui.add_space(12.0);
                                ui.label(egui::RichText::new("Baseplate").size(18.0).strong());
                                ui.add_space(4.0);
                                ui.label(egui::RichText::new("A baseplate to get ya started!")
                                    .size(12.0));
                            });

                            let response = ui.interact(inner_res.response.rect, card_id, egui::Sense::click());
                            if response.hovered() {
                                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                            }
                            if response.clicked() {
                                onboarding_data.selected_template = SelectedTemplate::Baseplate;
                                next_onboarding_state.set(crate::studio::tools::OnboardingState::BasicInfo);
                            }
                        });
                    });
                }
                crate::studio::tools::OnboardingState::BasicInfo => {
                    ui.vertical_centered(|ui| {
                        ui.label(egui::RichText::new("Basic Info").size(28.0).strong());
                        ui.add_space(8.0);
                        ui.label(egui::RichText::new("Setup your game's basic information!")
                            .size(14.0));
                    });
                    ui.add_space(20.0);

                    ui.visuals_mut().extreme_bg_color = egui::Color32::from_rgb(245, 246, 247);

                    ui.horizontal(|ui| {
                        let window_width = ui.available_width();
                        let column_width = 340.0;
                        let side_space = ((window_width - column_width) / 2.0).max(0.0);
                        ui.add_space(side_space);

                        ui.vertical(|ui| {
                            let label_size = 11.0;

                            ui.label(egui::RichText::new("NAME").size(label_size).strong());
                            ui.add_space(4.0);
                            ui.add(egui::TextEdit::singleline(&mut onboarding_data.name).desired_width(column_width));
                            ui.add_space(14.0);

                            ui.label(egui::RichText::new("DESCRIPTION").size(label_size).strong());
                            ui.add_space(4.0);
                            ui.add(egui::TextEdit::multiline(&mut onboarding_data.description).desired_width(column_width).desired_rows(3));
                            ui.add_space(14.0);

                            ui.label(egui::RichText::new("SAVE PATH").size(label_size).strong());
                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                ui.add(egui::TextEdit::singleline(&mut onboarding_data.save_path).desired_width(column_width - 85.0));
                                if ui.button("Browse...").clicked() {
                                    if let Some(path) = rfd::FileDialog::new()
                                        .add_filter("Rave Project", &["vrtx"])
                                        .set_directory(std::env::current_dir().unwrap_or_default())
                                        .save_file() {
                                            onboarding_data.save_path = path.display().to_string();
                                        }
                                }
                            });
                        });
                    });

                    ui.add_space(28.0);
                    ui.vertical_centered(|ui| {
                        ui.visuals_mut().widgets.hovered.bg_fill = egui::Color32::from_rgb(224, 238, 249);
                        ui.visuals_mut().widgets.hovered.weak_bg_fill = egui::Color32::from_rgb(224, 238, 249);
                        ui.visuals_mut().widgets.hovered.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(190, 220, 240));
                        ui.visuals_mut().widgets.hovered.fg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(20, 20, 20));

                        let create_btn = ui.add(
                            egui::Button::new(
                                egui::RichText::new("Create!")
                                    .size(16.0)
                                    .strong()
                            )
                            .min_size(egui::vec2(160.0, 42.0))
                        );

                        if create_btn.clicked() {
                            next_onboarding_state.set(crate::studio::tools::OnboardingState::Login);
                            
                            let mut bricks = Vec::new();
                            if onboarding_data.selected_template == SelectedTemplate::Baseplate {
                                bricks.push(crate::common::core::vrtx::VrtxBrick {
                                    name: "Baseplate".to_string(),
                                    transform: Transform::from_xyz(0.0, -0.14, 0.0).with_scale(Vec3::new(25.0, 1.0, 50.0)),
                                    shape: crate::common::game::bricks::components::BrickShape::Block,
                                    color: Color::Srgba(Srgba::new(0.28, 0.62, 0.32, 1.0)),
                                    physics_enabled: false,
                                    bounciness: 0.3,
                                    player_can_collide: true,
                                    friction: 0.3,
                                    gravity_scale: 1.0,
                                    mass: 1.0,
                                });
                                bricks.push(crate::common::core::vrtx::VrtxBrick {
                                    name: "Part0".to_string(),
                                    transform: Transform::from_xyz(0.0, 0.14, 0.0),
                                    shape: crate::common::game::bricks::components::BrickShape::Block,
                                    color: Color::Srgba(Srgba::new(0.84, 0.24, 0.16, 1.0)),
                                    physics_enabled: true,
                                    bounciness: 0.3,
                                    player_can_collide: true,
                                    friction: 0.3,
                                    gravity_scale: 1.0,
                                    mass: 1.0,
                                });
                            }

                            let state = crate::common::core::vrtx::VrtxFileState {
                                version: 3,
                                gravity: Vec3::new(0.0, -186.9 * 0.28, 0.0),
                                settings: crate::common::core::vrtx::VrtxSettings {
                                    ssao: false,
                                    contact_shadows: false,
                                    bloom: true,
                                },
                                camera_transform: Transform::from_xyz(-10.0, 10.0, -10.0).looking_at(Vec3::ZERO, Vec3::Y),
                                bricks,
                            };

                            let _ = state.save_to_file(&onboarding_data.save_path);

                            if onboarding_data.selected_template == SelectedTemplate::Baseplate {
                                commands.spawn((
                                    Mesh3d(meshes.add(Cuboid::new(4.0 * 0.28, 1.0 * 0.28, 2.0 * 0.28))),
                                    MeshMaterial3d(studs_materials.add(ExtendedMaterial {
                                        base: StandardMaterial {
                                            base_color: Color::srgb(0.28, 0.62, 0.32),
                                            perceptual_roughness: 0.95,
                                            reflectance: 0.08,
                                            metallic: 0.0,
                                            ..default()
                                        },
                                        extension: crate::common::game::bricks::studs::StudsExtension {
                                            stud_texture: studs_assets.stud.clone(),
                                            inlet_texture: studs_assets.inlet.clone(),
                                        },
                                    })),
                                    Transform::from_xyz(0.0, -0.14, 0.0).with_scale(Vec3::new(25.0, 1.0, 50.0)),
                                    crate::common::game::bricks::components::Brick,
                                    crate::common::game::bricks::components::BrickShapeComponent { shape: crate::common::game::bricks::components::BrickShape::Block },
                                    crate::common::game::bricks::components::BrickPhysics {
                                        enabled: false,
                                        bounciness: 0.3,
                                        player_can_collide: true,
                                        friction: 0.3,
                                        gravity_scale: 1.0,
                                        mass: 1.0,
                                    },
                                    crate::common::game::bricks::components::BrickColor { color: Color::srgb(0.28, 0.62, 0.32) },
                                    CollisionLayers::from_bits(0b0001, 0xFFFF_FFFF),
                                    Pickable::default(),
                                    Name::new("Baseplate"),
                                ));

                                spawn_brick(commands, meshes, studs_materials, studs_assets, count, Vec3::new(0.0, 0.14, 0.0), crate::common::game::bricks::components::BrickShape::Block);
                            }
                        }
                    });
                }
                crate::studio::tools::OnboardingState::Login => {
                    ui.vertical_centered(|ui| {
                        ui.label(egui::RichText::new("Login to VERTEXIA!").size(28.0).strong());
                        ui.add_space(8.0);
                        ui.label(egui::RichText::new("To save your game and publish it to the website, you will need to login with your account")
                            .size(14.0));
                    });
                    ui.add_space(24.0);

                    ui.vertical_centered(|ui| {
                        let login_btn = ui.scope(|ui| {
                            let purple_normal = egui::Color32::from_rgb(116, 35, 203);
                            let purple_hover = egui::Color32::from_rgb(138, 55, 225);
                            let purple_active = egui::Color32::from_rgb(96, 25, 183);
                            let white_text = egui::Color32::from_rgb(255, 255, 255);

                            ui.visuals_mut().widgets.inactive.bg_fill = purple_normal;
                            ui.visuals_mut().widgets.inactive.weak_bg_fill = purple_normal;
                            ui.visuals_mut().widgets.inactive.fg_stroke = egui::Stroke::new(1.5, white_text);
                            ui.visuals_mut().widgets.inactive.bg_stroke = egui::Stroke::NONE;

                            ui.visuals_mut().widgets.hovered.bg_fill = purple_hover;
                            ui.visuals_mut().widgets.hovered.weak_bg_fill = purple_hover;
                            ui.visuals_mut().widgets.hovered.fg_stroke = egui::Stroke::new(1.5, white_text);
                            ui.visuals_mut().widgets.hovered.bg_stroke = egui::Stroke::NONE;

                            ui.visuals_mut().widgets.active.bg_fill = purple_active;
                            ui.visuals_mut().widgets.active.weak_bg_fill = purple_active;
                            ui.visuals_mut().widgets.active.fg_stroke = egui::Stroke::new(1.5, white_text);
                            ui.visuals_mut().widgets.active.bg_stroke = egui::Stroke::NONE;

                            ui.add(
                                egui::Button::new(
                                    egui::RichText::new("Login to VERTEXIA ↗")
                                        .size(16.0)
                                        .strong()
                                )
                                .min_size(egui::vec2(220.0, 42.0))
                            )
                        }).inner;

                        if login_btn.clicked() {
                        }

                        ui.add_space(14.0);

                        let skip_btn = ui.add(
                            egui::Label::new(
                                egui::RichText::new("No thanks, use without login")
                                    .size(12.0)
                                    .underline()
                            )
                            .sense(egui::Sense::click())
                        );

                        if skip_btn.hovered() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        }

                        if skip_btn.clicked() {
                            next_onboarding_state.set(crate::studio::tools::OnboardingState::Inactive);
                        }
                    });
                }
                _ => {}
            }
        });
}