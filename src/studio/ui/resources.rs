use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::studio::ui::panels::settings::SettingsTab;

#[derive(SystemParam)]
pub struct ServiceResources<'w> {
    pub lighting_config: Option<ResMut<'w, crate::client::sky::LightingConfig>>,
    pub gravity: Option<ResMut<'w, avian3d::prelude::Gravity>>,
    pub players_service: Option<ResMut<'w, crate::studio::tools::PlayersService>>,
}

pub fn pick_file_dialog(filter_label: &str) -> Option<std::path::PathBuf> {
    //fixes the compilation issue on android: we dont care about rfd on android, just null
    #[cfg(not(target_os = "android"))]
    {
        rfd::FileDialog::new()
            .add_filter(filter_label, &["vrtx"])
            .set_directory(std::env::current_dir().unwrap_or_default())
            .pick_file()
    }
    #[cfg(target_os = "android")]
    {
        None
    }
}
pub fn save_file_dialog(filter_label: &str) -> Option<std::path::PathBuf> {
    //fixes the compilation issue on android: we dont care about rfd on android, just null
    #[cfg(not(target_os = "android"))]
    {
        rfd::FileDialog::new()
            .add_filter(filter_label, &["vrtx"])
            .set_directory(std::env::current_dir().unwrap_or_default())
            .save_file()
    }
    #[cfg(target_os = "android")]
    {
        None
    }
}

#[derive(Resource, Default)]
pub struct CopiedEntityBuffer {
    pub transform: Option<Transform>,
    pub mesh: Option<Mesh3d>,
    pub mesh_asset: Option<crate::common::game::assets::components::Mesh>,
    pub texture: Option<crate::common::game::assets::components::Texture>,
    pub material: Option<
        MeshMaterial3d<
            bevy::pbr::ExtendedMaterial<
                StandardMaterial,
                crate::common::game::bricks::studs::ShadowOpacityExtension,
            >,
        >,
    >,
    pub studs_material: Option<
        MeshMaterial3d<
            bevy::pbr::ExtendedMaterial<
                StandardMaterial,
                crate::common::game::bricks::studs::StudsExtension,
            >,
        >,
    >,
    pub name: Option<String>,
    pub is_brick: bool,
    pub shape: crate::common::game::bricks::components::BrickShape,
    pub physics: Option<crate::common::game::bricks::components::BrickPhysics>,
    pub show_studs: bool,
    pub color: Option<Color>,
}

#[derive(Resource, Default)]
pub struct HierarchyDraggedEntity {
    pub entity: Option<Entity>,
}

#[derive(Resource, Default)]
pub struct SettingsWindow {
    pub open: bool,
    pub tab: SettingsTab,
}

#[derive(Resource)]
pub struct VrtxSaveSettings {
    pub include_camera_position: bool,
    pub include_graphics_settings: bool,
}

impl Default for VrtxSaveSettings {
    fn default() -> Self {
        Self {
            include_camera_position: true,
            include_graphics_settings: true,
        }
    }
}

#[derive(Resource)]
pub struct StudioSettings {
    pub mouse_sensitivity: f32,
    pub camera_speed: f32,
    pub fov: f32,
}

impl Default for StudioSettings {
    fn default() -> Self {
        Self {
            mouse_sensitivity: 0.2,
            camera_speed: 5.0,
            fov: 80.0,
        }
    }
}

#[derive(Resource, Default)]
pub struct PlayInClientProcesses {
    pub client_process: Option<std::process::Child>,
}

#[derive(Resource, Default)]
pub struct PlaytestBackup {
    pub bricks: Vec<crate::common::game::bricks::data::BrickData>,
    pub scripts: Vec<crate::common::core::vrtx::VrtxScript>,
    pub images: Vec<crate::common::core::vrtx::VrtxImage>,
    pub meshes: Vec<crate::common::core::vrtx::VrtxMesh>,
    pub textures: Vec<crate::common::core::vrtx::VrtxTexture>,
    pub gravity: Option<Vec3>,
    pub players_service: Option<crate::studio::tools::PlayersService>,
}

#[derive(Component)]
pub struct InEditorPlaytestClient;

#[derive(Resource, Default)]
pub struct ActiveScriptEditor {
    pub entity: Option<Entity>,
    pub open: bool,
    pub buffer: String,
    pub error: Option<String>,
    pub open_entities: Vec<Entity>,
}

pub enum FileDialogResult {
    BrowseSavePath(std::path::PathBuf),
    OpenFile(std::path::PathBuf),
    SaveAs(std::path::PathBuf),
    Cancel,
}

#[derive(Resource)]
pub struct FileDialogState {
    pub tx: std::sync::mpsc::Sender<FileDialogResult>,
    pub rx: std::sync::Mutex<std::sync::mpsc::Receiver<FileDialogResult>>,
    pub is_open: std::sync::atomic::AtomicBool,
}

impl Default for FileDialogState {
    fn default() -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        Self {
            tx,
            rx: std::sync::Mutex::new(rx),
            is_open: std::sync::atomic::AtomicBool::new(false),
        }
    }
}

pub fn cleanup_play_processes_on_exit(
    events: MessageReader<AppExit>,
    mut play_processes: ResMut<PlayInClientProcesses>,
) {
    if !events.is_empty() {
        crate::app::server::bootstrap::request_playtest_server_shutdown();
        if let Some(mut child) = play_processes.client_process.take() {
            let _ = child.kill();
        }
    }
}

pub fn handle_file_dialog_results(
    mut commands: Commands,
    file_dialog_state: Res<FileDialogState>,
    mut onboarding_data: ResMut<crate::studio::ui::panels::onboarding::OnboardingData>,
    mut next_onboarding_state: ResMut<NextState<crate::studio::tools::OnboardingState>>,
    mut graphics_settings: ResMut<crate::common::core::performance::GraphicsSettings>,
    vrtx_save_settings: Res<VrtxSaveSettings>,
    mut service_resources: ServiceResources,
    mut camera_transform_query: Query<&mut Transform, With<Camera3d>>,
    materials: ResMut<
        Assets<
            bevy::pbr::ExtendedMaterial<
                StandardMaterial,
                crate::common::game::bricks::studs::ShadowOpacityExtension,
            >,
        >,
    >,
    studs_materials: ResMut<
        Assets<
            bevy::pbr::ExtendedMaterial<
                StandardMaterial,
                crate::common::game::bricks::studs::StudsExtension,
            >,
        >,
    >,
    entities_query: Query<
        (
            Entity,
            Option<&crate::common::game::bricks::components::Brick>,
            Option<&crate::common::game::assets::components::Image>,
            Option<&crate::common::game::assets::components::Mesh>,
            Option<&crate::common::game::assets::components::Texture>,
        ),
        Without<Camera3d>,
    >,
    explorer_query: Query<
        (
            Entity,
            Option<&crate::scripting::ecs::ServerScript>,
            Option<&crate::scripting::ecs::LocalScript>,
            Option<&crate::scripting::ecs::ModuleScript>,
            Option<&crate::common::game::assets::components::Image>,
        ),
        Without<Camera3d>,
    >,
    save_query: Query<
        (
            Entity,
            &Transform,
            &Name,
            Option<&ChildOf>,
            Option<&Children>,
            Option<&crate::common::game::bricks::components::Brick>,
            Option<&crate::common::game::bricks::components::BrickShapeComponent>,
            &GlobalTransform,
            Option<&Mesh3d>,
            Option<
                &MeshMaterial3d<
                    bevy::pbr::ExtendedMaterial<
                        StandardMaterial,
                        crate::common::game::bricks::studs::ShadowOpacityExtension,
                    >,
                >,
            >,
            Option<
                &MeshMaterial3d<
                    bevy::pbr::ExtendedMaterial<
                        StandardMaterial,
                        crate::common::game::bricks::studs::StudsExtension,
                    >,
                >,
            >,
            Option<&crate::common::game::bricks::components::BrickPhysics>,
        ),
        Without<Camera3d>,
    >,
    save_explorer_query: Query<
        (
            Entity,
            &Name,
            Option<&ChildOf>,
            Option<&Children>,
            Option<&crate::common::game::bricks::components::Brick>,
            Option<&crate::scripting::ecs::ServerScript>,
            Option<&crate::scripting::ecs::LocalScript>,
            Option<&crate::scripting::ecs::ModuleScript>,
            Option<&crate::common::game::assets::components::Image>,
            Option<&crate::common::game::assets::components::Texture>,
            Option<&crate::common::game::assets::components::Mesh>,
        ),
        Without<Camera3d>,
    >,
    studs_query: Query<&crate::common::game::bricks::components::BrickStuds>,
) {
    let rx = file_dialog_state.rx.lock().unwrap();
    while let Ok(result) = rx.try_recv() {
        match result {
            FileDialogResult::BrowseSavePath(path) => {
                onboarding_data.save_path = path.display().to_string();
                file_dialog_state
                    .is_open
                    .store(false, std::sync::atomic::Ordering::Relaxed);
            }
            FileDialogResult::OpenFile(path) => {
                let open_path_str = path.display().to_string();
                if let Ok(state) =
                    crate::common::core::vrtx::VrtxFileState::load_from_file(&open_path_str)
                {
                    onboarding_data.save_path = open_path_str;
                    if onboarding_data.quick_open {
                        onboarding_data.quick_open = false;
                        next_onboarding_state.set(crate::studio::tools::OnboardingState::Inactive);
                    }
                    for (entity, brick_opt, image_opt, mesh_opt, texture_opt) in &entities_query {
                        if brick_opt.is_some()
                            || image_opt.is_some()
                            || mesh_opt.is_some()
                            || texture_opt.is_some()
                        {
                            commands.entity(entity).try_despawn();
                        }
                    }
                    for (entity, s_opt, l_opt, m_opt, _) in &explorer_query {
                        if s_opt.is_some() || l_opt.is_some() || m_opt.is_some() {
                            commands.entity(entity).try_despawn();
                        }
                    }
                    graphics_settings.ssao = state.settings.ssao;
                    graphics_settings.contact_shadows = state.settings.contact_shadows;
                    graphics_settings.bloom = state.settings.bloom;
                    if let Some(lighting) = service_resources.lighting_config.as_mut() {
                        **lighting = state.lighting.clone().into();
                    }
                    if let Some(ref mut g) = service_resources.gravity {
                        g.0 = state.gravity;
                    }
                    if let Some(mut ps) = service_resources.players_service.as_mut() {
                        **ps = crate::studio::tools::PlayersService {
                            speed: state.players.speed,
                            jump_power: state.players.jump_power,
                            gravity: state.players.gravity,
                            speed_response: state.players.speed_response,
                            friction: state.players.friction,
                            bounciness: state.players.bounciness,
                        };
                        if let Ok(mut shared) = crate::studio::tools::SHARED_PLAYERS_SERVICE.write() {
                            *shared = (**ps).clone();
                        }
                    } else if let Ok(mut shared) =
                        crate::studio::tools::SHARED_PLAYERS_SERVICE.write()
                    {
                        *shared = crate::studio::tools::PlayersService {
                            speed: state.players.speed,
                            jump_power: state.players.jump_power,
                            gravity: state.players.gravity,
                            speed_response: state.players.speed_response,
                            friction: state.players.friction,
                            bounciness: state.players.bounciness,
                        };
                    }
                    if let Some(mut cam_t) = camera_transform_query.iter_mut().next() {
                        *cam_t = state.camera_transform;
                    }
                    let mut named_entities = std::collections::HashMap::new();
                    for brick in state.bricks {
                        let layers = if brick.player_can_collide {
                            avian3d::prelude::CollisionLayers::from_bits(0b0001, 0xFFFF_FFFF)
                        } else {
                            avian3d::prelude::CollisionLayers::from_bits(0b0100, 0xFFFF_FFFD)
                        };
                        let new_id = commands
                            .spawn((
                                brick.transform,
                                crate::common::game::bricks::components::Brick,
                                crate::common::game::bricks::components::BrickShapeComponent {
                                    shape: brick.shape,
                                },
                                crate::common::game::bricks::components::BrickPhysics {
                                    enabled: brick.physics_enabled,
                                    bounciness: brick.bounciness,
                                    player_can_collide: brick.player_can_collide,
                                    friction: brick.friction,
                                    gravity_scale: brick.gravity_scale,
                                    mass: brick.mass,
                                },
                                crate::common::game::bricks::components::BrickColor {
                                    color: brick.color,
                                },
                                crate::common::game::bricks::components::BrickStuds {
                                    enabled: brick.show_studs,
                                },
                                layers,
                                Pickable::default(),
                                Name::new(brick.name.clone()),
                            ))
                            .id();
                        named_entities.insert(brick.name, new_id);
                    }
                    for script in state.scripts {
                        let mut cmd = commands.spawn(Name::new(script.name));
                        match script.script_type {
                            0 => {
                                cmd.insert(crate::scripting::ecs::ServerScript {
                                    code: script.code,
                                    enabled: script.enabled,
                                    started: false,
                                    running_code: String::new(),
                                });
                            }
                            1 => {
                                cmd.insert((
                                    crate::scripting::ecs::LocalScript {
                                        code: script.code,
                                        enabled: script.enabled,
                                        started: false,
                                        running_code: String::new(),
                                    },
                                    lightyear::prelude::Replicate::default(),
                                ));
                            }
                            _ => {
                                cmd.insert((
                                    crate::scripting::ecs::ModuleScript { code: script.code },
                                    lightyear::prelude::Replicate::default(),
                                ));
                            }
                        }
                        let new_script_entity = cmd.id();
                        if let Some(ref p_name) = script.parent_name {
                            if let Some(&parent_entity) = named_entities.get(p_name) {
                                commands.entity(parent_entity).add_child(new_script_entity);
                            }
                        }
                    }
                    for image in state.images {
                        let face = image
                            .face
                            .as_deref()
                            .and_then(crate::common::game::assets::components::ImageFace::from_str);
                        let cmd = commands.spawn((
                            image.transform,
                            Name::new(image.name),
                            crate::common::game::assets::components::Image {
                                asset_id: image.asset_id,
                                face,
                            },
                            Pickable::default(),
                        ));
                        let new_image_entity = cmd.id();
                        if let Some(ref p_name) = image.parent_name {
                            if let Some(&parent_entity) = named_entities.get(p_name) {
                                commands.entity(parent_entity).add_child(new_image_entity);
                            }
                        }
                    }
                    let mut mesh_entities: Vec<(Entity, u32)> = Vec::new();
                    for mesh in state.meshes {
                        let mesh_asset_id = mesh.asset_id;
                        let cmd = commands.spawn((
                            mesh.transform,
                            Name::new(mesh.name.clone()),
                            crate::common::game::assets::components::Mesh {
                                asset_id: mesh.asset_id,
                                normalize: mesh.normalize,
                            },
                            crate::common::game::bricks::components::BrickPhysics {
                                enabled: mesh.physics_enabled,
                                bounciness: mesh.bounciness,
                                player_can_collide: mesh.player_can_collide,
                                friction: mesh.friction,
                                gravity_scale: mesh.gravity_scale,
                                mass: mesh.mass,
                            },
                            avian3d::prelude::CollisionLayers::from_bits(0b0001, 0xFFFF_FFFF),
                            Pickable::default(),
                        ));
                        let new_mesh_entity = cmd.id();
                        named_entities.insert(mesh.name, new_mesh_entity);
                        mesh_entities.push((new_mesh_entity, mesh_asset_id));
                        if let Some(ref p_name) = mesh.parent_name {
                            if let Some(&parent_entity) = named_entities.get(p_name) {
                                commands.entity(parent_entity).add_child(new_mesh_entity);
                            }
                        }
                    }
                    // Texture children of meshes (mesh names are registered in
                    // named_entities above so they resolve as parents too).
                    let mut textured_meshes = std::collections::HashSet::new();
                    for texture in state.textures {
                        let parsed = crate::common::game::assets::components::Texture::parse_content_id(
                            &texture.id_string,
                        );
                        let cmd = commands.spawn((
                            Name::new(texture.name),
                            crate::common::game::assets::components::Texture {
                                asset_id: parsed.map(|(id, _)| id).unwrap_or(0),
                                is_decal: parsed.map(|(_, decal)| decal).unwrap_or(false),
                            },
                        ));
                        let new_texture_entity = cmd.id();
                        if let Some(ref p_name) = texture.parent_name {
                            if let Some(&parent_entity) = named_entities.get(p_name) {
                                commands.entity(parent_entity).add_child(new_texture_entity);
                                textured_meshes.insert(parent_entity);
                            }
                        }
                    }
                    // Meshes without a persisted texture child get a fresh
                    // default one.
                    for (mesh_entity, mesh_asset_id) in mesh_entities {
                        if !textured_meshes.contains(&mesh_entity) {
                            let texture_entity = commands
                                .spawn((
                                    Name::new("Texture"),
                                    crate::common::game::assets::components::Texture {
                                        asset_id: mesh_asset_id,
                                        is_decal: false,
                                    },
                                ))
                                .id();
                            commands.entity(mesh_entity).add_child(texture_entity);
                        }
                    }
                }
                onboarding_data.quick_open = false;
                file_dialog_state
                    .is_open
                    .store(false, std::sync::atomic::Ordering::Relaxed);
            }
            FileDialogResult::SaveAs(path) => {
                let save_path_str = path.display().to_string();
                onboarding_data.save_path = save_path_str.clone();

                let mut bricks_data = Vec::new();
                for (
                    entity,
                    transform,
                    name,
                    _,
                    _,
                    brick_opt,
                    shape_opt,
                    _,
                    _,
                    mat_opt,
                    studs_mat_opt,
                    phys_opt,
                ) in &save_query
                {
                    if brick_opt.is_some() {
                        let shape = shape_opt
                            .as_ref()
                            .map(|s| s.shape)
                            .unwrap_or(crate::common::game::bricks::components::BrickShape::Block);
                        let mut current_color = Color::Srgba(Srgba::new(0.84, 0.24, 0.16, 1.0));
                        if let Some(studs_mat_handle) = studs_mat_opt {
                            if let Some(mat) = studs_materials.get(&studs_mat_handle.0) {
                                current_color = mat.base.base_color;
                            }
                        } else if let Some(mat_handle) = mat_opt {
                            if let Some(mat) = materials.get(&mat_handle.0) {
                                current_color = mat.base.base_color;
                            }
                        }
                        let (
                            physics_enabled,
                            bounciness,
                            player_can_collide,
                            friction,
                            gravity_scale,
                            mass,
                        ) = if let Some(phys) = phys_opt {
                            (
                                phys.enabled,
                                phys.bounciness,
                                phys.player_can_collide,
                                phys.friction,
                                phys.gravity_scale,
                                phys.mass,
                            )
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
                            show_studs: studs_query.get(entity).map(|s| s.enabled).unwrap_or(true),
                        });
                    }
                }

                let mut scripts_data = Vec::new();
                for (_entity, name, child_of_opt, _, _, s_opt, l_opt, m_opt, _, _, _) in
                    &save_explorer_query
                {
                    let mut script_type_opt = None;
                    let mut code = String::new();
                    let mut enabled = true;
                    if let Some(s) = s_opt {
                        script_type_opt = Some(0);
                        code = s.code.clone();
                        enabled = s.enabled;
                    } else if let Some(l) = l_opt {
                        script_type_opt = Some(1);
                        code = l.code.clone();
                        enabled = l.enabled;
                    } else if let Some(m) = m_opt {
                        script_type_opt = Some(2);
                        code = m.code.clone();
                    }
                    if let Some(script_type) = script_type_opt {
                        let mut parent_name = None;
                        if let Some(child_of) = child_of_opt {
                            if let Ok((_, p_name, _, _, _, _, _, _, _, _, _)) =
                                save_explorer_query.get(child_of.parent())
                            {
                                parent_name = Some(p_name.to_string());
                            }
                        }
                        scripts_data.push(crate::common::core::vrtx::VrtxScript {
                            name: name.to_string(),
                            script_type,
                            code,
                            parent_name,
                            enabled,
                        });
                    }
                }

                let mut images_data = Vec::new();
                for (_entity, name, child_of_opt, _, _, _, _, _, image_opt, _, _) in
                    &save_explorer_query
                {
                    if let Some(image) = image_opt {
                        let mut parent_name = None;
                        if let Some(child_of) = child_of_opt {
                            if let Ok((_, p_name, _, _, _, _, _, _, _, _, _)) =
                                save_explorer_query.get(child_of.parent())
                            {
                                parent_name = Some(p_name.to_string());
                            }
                        }
                        let transform = save_query
                            .get(_entity)
                            .map(|(_, t, _, _, _, _, _, _, _, _, _, _)| *t)
                            .unwrap_or_default();
                        images_data.push(crate::common::core::vrtx::VrtxImage {
                            name: name.to_string(),
                            asset_id: image.asset_id,
                            face: image.face.as_ref().map(|f| f.as_str().to_string()),
                            parent_name,
                            transform,
                        });
                    }
                }

                let mut meshes_data = Vec::new();
                for (_entity, name, child_of_opt, _, _, _, _, _, _, _, mesh_opt) in
                    &save_explorer_query
                {
                    if let Some(mesh) = mesh_opt {
                        let mut parent_name = None;
                        if let Some(child_of) = child_of_opt {
                            if let Ok((_, p_name, _, _, _, _, _, _, _, _, _)) =
                                save_explorer_query.get(child_of.parent())
                            {
                                parent_name = Some(p_name.to_string());
                            }
                        }
                        let transform = save_query
                            .get(_entity)
                            .map(|(_, t, _, _, _, _, _, _, _, _, _, _)| *t)
                            .unwrap_or_default();
                        let (
                            physics_enabled,
                            bounciness,
                            player_can_collide,
                            friction,
                            gravity_scale,
                            mass,
                        ) = save_query
                            .get(_entity)
                            .map(|(_, _, _, _, _, _, _, _, _, _, _, phys_opt)| {
                                if let Some(phys) = phys_opt {
                                    (
                                        phys.enabled,
                                        phys.bounciness,
                                        phys.player_can_collide,
                                        phys.friction,
                                        phys.gravity_scale,
                                        phys.mass,
                                    )
                                } else {
                                    (true, 0.3, true, 0.3, 1.0, 1.0)
                                }
                            })
                            .unwrap_or((true, 0.3, true, 0.3, 1.0, 1.0));
                        meshes_data.push(crate::common::core::vrtx::VrtxMesh {
                            name: name.to_string(),
                            asset_id: mesh.asset_id,
                            normalize: mesh.normalize,
                            parent_name,
                            transform,
                            physics_enabled,
                            bounciness,
                            player_can_collide,
                            friction,
                            gravity_scale,
                            mass,
                        });
                    }
                }

                let mut textures_data = Vec::new();
                for (_entity, name, child_of_opt, _, _, _, _, _, _, texture_opt, _) in
                    &save_explorer_query
                {
                    if let Some(texture) = texture_opt {
                        let mut parent_name = None;
                        if let Some(child_of) = child_of_opt {
                            if let Ok((_, p_name, _, _, _, _, _, _, _, _, _)) =
                                save_explorer_query.get(child_of.parent())
                            {
                                parent_name = Some(p_name.to_string());
                            }
                        }
                        textures_data.push(crate::common::core::vrtx::VrtxTexture {
                            name: name.to_string(),
                            id_string: texture.as_content_id(),
                            parent_name,
                        });
                    }
                }

                let gravity_val = if let Some(g) = service_resources.gravity.as_ref() {
                    g.0
                } else {
                    Vec3::new(0.0, -186.9 * 0.28, 0.0)
                };
                let cam_transform = if vrtx_save_settings.include_camera_position {
                    camera_transform_query
                        .iter()
                        .next()
                        .map(|cam_t| *cam_t)
                        .unwrap_or_default()
                } else {
                    Transform::IDENTITY
                };
                let saved_settings = if vrtx_save_settings.include_graphics_settings {
                    crate::common::core::vrtx::VrtxSettings {
                        ssao: graphics_settings.ssao,
                        contact_shadows: graphics_settings.contact_shadows,
                        bloom: graphics_settings.bloom,
                    }
                } else {
                    crate::common::core::vrtx::VrtxSettings {
                        ssao: false,
                        contact_shadows: false,
                        bloom: true,
                    }
                };
                let lighting = service_resources
                    .lighting_config
                    .as_ref()
                    .map(|lighting| crate::common::core::vrtx::VrtxLighting::from(&**lighting))
                    .unwrap_or_default();
                let players = if let Some(ps) = service_resources.players_service.as_ref() {
                    crate::common::core::vrtx::VrtxPlayers {
                        speed: ps.speed,
                        jump_power: ps.jump_power,
                        gravity: ps.gravity,
                        speed_response: ps.speed_response,
                        friction: ps.friction,
                        bounciness: ps.bounciness,
                    }
                } else {
                    crate::common::core::vrtx::VrtxPlayers::default()
                };
                let state = crate::common::core::vrtx::VrtxFileState {
                    version: crate::common::core::vrtx::FORMAT_VERSION,
                    gravity: gravity_val,
                    settings: saved_settings,
                    lighting,
                    players,
                    camera_transform: cam_transform,
                    bricks: bricks_data,
                    scripts: scripts_data,
                    images: images_data,
                    meshes: meshes_data,
                    textures: textures_data,
                };
                let _ = state.save_to_file(&save_path_str);
                file_dialog_state
                    .is_open
                    .store(false, std::sync::atomic::Ordering::Relaxed);
            }
            FileDialogResult::Cancel => {
                onboarding_data.quick_open = false;
                file_dialog_state
                    .is_open
                    .store(false, std::sync::atomic::Ordering::Relaxed);
            }
        }
    }
}
