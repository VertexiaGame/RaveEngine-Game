use bevy::prelude::*;
use bevy_egui::egui;
use bevy::pbr::ExtendedMaterial;
use avian3d::prelude::CollisionLayers;

pub fn draw_file_window(
    ctx: &egui::Context,
    open: &mut bool,
    onboarding_data: &mut crate::studio::ui::panels::onboarding::OnboardingData,
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    studs_materials: &mut ResMut<Assets<ExtendedMaterial<StandardMaterial, crate::common::game::bricks::studs::StudsExtension>>>,
    studs_assets: &crate::common::game::bricks::studs::StudsAssets,
    graphics_settings: &mut crate::studio::ui::GraphicsSettings,
    gravity: &mut Option<ResMut<avian3d::prelude::Gravity>>,
    camera_transform_query: &mut Query<&mut Transform, With<Camera3d>>,
    entities_query: &mut Query<(
        Entity,
        &mut Transform,
        &mut Name,
        Option<&ChildOf>,
        Option<&Children>,
        Option<&crate::common::game::bricks::components::Brick>,
        Option<&mut crate::common::game::bricks::components::BrickShapeComponent>,
        &GlobalTransform,
        Option<&Mesh3d>,
        Option<&MeshMaterial3d<StandardMaterial>>,
        Option<&MeshMaterial3d<ExtendedMaterial<StandardMaterial, crate::common::game::bricks::studs::StudsExtension>>>,
        Option<&mut crate::common::game::bricks::components::BrickPhysics>,
    ), Without<Camera3d>>,
) {
    egui::Window::new("File")
        .open(open)
        .pivot(egui::Align2::CENTER_CENTER)
        .default_pos(ctx.content_rect().center())
        .default_size(egui::vec2(320.0, 200.0))
        .resizable(false)
        .collapsible(false)
        .show(ctx, |ui| {
            ui.label(egui::RichText::new("Save/Load Project").strong().size(14.0));
            ui.add_space(8.0);

            ui.label("File Path:");
            ui.horizontal(|ui| {
                ui.text_edit_singleline(&mut onboarding_data.save_path);
                if ui.button("Browse...").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Rave Project", &["vrtx"])
                        .set_directory(std::env::current_dir().unwrap_or_default())
                        .save_file() {
                        onboarding_data.save_path = path.display().to_string();
                    }
                }
            });

            ui.add_space(12.0);

            ui.horizontal(|ui| {
                let save_enabled = !onboarding_data.save_path.is_empty();
                if ui.add_enabled(save_enabled, egui::Button::new("Save")).clicked() {
                    let mut bricks_data = Vec::new();
                    for (_, transform, name, _, _, brick_opt, shape_opt, _, _, mat_opt, studs_mat_opt, phys_opt) in entities_query.iter() {
                        if brick_opt.is_some() {
                            let shape = shape_opt.as_ref().map(|s| s.shape).unwrap_or(crate::common::game::bricks::components::BrickShape::Block);
                            let mut current_color = Color::Srgba(Srgba::new(0.84, 0.24, 0.16, 1.0));
                            if let Some(studs_mat_handle) = studs_mat_opt {
                                if let Some(mat) = studs_materials.get(&studs_mat_handle.0) {
                                    current_color = mat.base.base_color;
                                }
                            } else if let Some(mat_handle) = mat_opt {
                                if let Some(mat) = materials.get(&mat_handle.0) {
                                    current_color = mat.base_color;
                                }
                            }
                            let (physics_enabled, bounciness, player_can_collide, friction, gravity_scale, mass) = if let Some(phys) = phys_opt {
                                (phys.enabled, phys.bounciness, phys.player_can_collide, phys.friction, phys.gravity_scale, phys.mass)
                            } else {
                                (true, 0.3, true, 0.3, 1.0, 1.0)
                            };
                            bricks_data.push(crate::common::core::vrtx::VrtxBrick {
                                name: name.to_string(),
                                transform: *transform,
                                shape,
                                color: current_color,
                                physics_enabled,
                                bounciness,
                                player_can_collide,
                                friction,
                                gravity_scale,
                                mass,
                            });
                        }
                    }
                    let gravity_val = if let Some(g) = gravity.as_ref() {
                        g.0
                    } else {
                        Vec3::new(0.0, -186.9 * 0.28, 0.0)
                    };
                    let cam_transform = if let Some(cam_t) = camera_transform_query.iter().next() {
                        *cam_t
                    } else {
                        Transform::IDENTITY
                    };
                    let state = crate::common::core::vrtx::VrtxFileState {
                        version: 3,
                        gravity: gravity_val,
                        settings: crate::common::core::vrtx::VrtxSettings {
                            ssao: graphics_settings.ssao,
                            contact_shadows: graphics_settings.contact_shadows,
                            bloom: graphics_settings.bloom,
                        },
                        camera_transform: cam_transform,
                        bricks: bricks_data,
                    };
                    let _ = state.save_to_file(&onboarding_data.save_path);
                }

                if ui.button("Open...").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Rave Project", &["vrtx"])
                        .set_directory(std::env::current_dir().unwrap_or_default())
                        .pick_file() {
                        let open_path_str = path.display().to_string();
                        if let Ok(state) = crate::common::core::vrtx::VrtxFileState::load_from_file(&open_path_str) {
                            onboarding_data.save_path = open_path_str;
                            for (entity, _, _, _, _, brick_opt, _, _, _, _, _, _) in entities_query.iter() {
                                if brick_opt.is_some() {
                                    commands.entity(entity).try_despawn();
                                }
                            }
                            graphics_settings.ssao = state.settings.ssao;
                            graphics_settings.contact_shadows = state.settings.contact_shadows;
                            graphics_settings.bloom = state.settings.bloom;
                            if let Some(g) = gravity.as_mut() {
                                g.0 = state.gravity;
                            }
                            if let Some(mut cam_t) = camera_transform_query.iter_mut().next() {
                                *cam_t = state.camera_transform;
                            }
                            for brick in state.bricks {
                                let mesh_handle = match brick.shape {
                                    crate::common::game::bricks::components::BrickShape::Block => {
                                        meshes.add(Cuboid::new(4.0 * 0.28, 1.0 * 0.28, 2.0 * 0.28))
                                    }
                                    crate::common::game::bricks::components::BrickShape::Sphere => {
                                        meshes.add(Sphere::new(1.0 * 0.28))
                                    }
                                };
                                let layers = if brick.player_can_collide {
                                    CollisionLayers::from_bits(0b0001, 0xFFFF_FFFF)
                                } else {
                                    CollisionLayers::from_bits(0b0100, 0xFFFF_FFFD)
                                };
                                commands.spawn((
                                    Mesh3d(mesh_handle),
                                    MeshMaterial3d(studs_materials.add(ExtendedMaterial {
                                        base: StandardMaterial {
                                            base_color: brick.color,
                                            perceptual_roughness: 0.95,
                                            reflectance: 0.1,
                                            alpha_mode: if brick.color.alpha() < 1.0 { AlphaMode::Blend } else { AlphaMode::Opaque },
                                            ..default()
                                        },
                                        extension: crate::common::game::bricks::studs::StudsExtension {
                                            stud_texture: studs_assets.stud.clone(),
                                            inlet_texture: studs_assets.inlet.clone(),
                                        },
                                    })),
                                    brick.transform,
                                    crate::common::game::bricks::components::Brick,
                                    crate::common::game::bricks::components::BrickShapeComponent { shape: brick.shape },
                                    crate::common::game::bricks::components::BrickPhysics {
                                        enabled: brick.physics_enabled,
                                        bounciness: brick.bounciness,
                                        player_can_collide: brick.player_can_collide,
                                        friction: brick.friction,
                                        gravity_scale: brick.gravity_scale,
                                        mass: brick.mass,
                                    },
                                    crate::common::game::bricks::components::BrickColor { color: brick.color },
                                    layers,
                                    Pickable::default(),
                                    Name::new(brick.name),
                                ));
                            }
                        }
                    }
                }
            });
        });
}