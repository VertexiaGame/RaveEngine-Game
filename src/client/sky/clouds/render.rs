use bevy::{
    asset::{embedded_asset, embedded_path, AssetPath},
    prelude::*,
    reflect::TypePath,
    render::render_resource::{AsBindGroup, Face},
    shader::{load_shader_library, ShaderRef},
};

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub(crate) struct CloudsMaterial {
    #[texture(100, visibility(vertex, fragment))]
    #[sampler(101, visibility(vertex, fragment))]
    pub cloud_render_image: Handle<Image>,

    #[texture(102, visibility(vertex, fragment))]
    #[sampler(103, visibility(vertex, fragment))]
    pub cloud_atlas_image: Handle<Image>,

    #[texture(104, visibility(vertex, fragment), dimension = "3d")]
    #[sampler(105, visibility(vertex, fragment))]
    pub cloud_worley_image: Handle<Image>,

    #[texture(106, visibility(vertex, fragment))]
    #[sampler(107, visibility(vertex, fragment))]
    pub sky_image: Handle<Image>,
}

impl Material for CloudsMaterial {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Path(
            AssetPath::from_path_buf(embedded_path!("shaders/clouds.wgsl")).with_source("embedded"),
        )
    }

    fn specialize(
        _pipeline: &bevy::pbr::MaterialPipeline,
        descriptor: &mut bevy::render::render_resource::RenderPipelineDescriptor,
        _layout: &bevy::render::mesh::MeshVertexBufferLayoutRef,
        _key: bevy::pbr::MaterialPipelineKey<Self>,
    ) -> Result<(), bevy::render::render_resource::SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = Some(Face::Front);
        if let Some(depth_stencil) = &mut descriptor.depth_stencil {
            depth_stencil.depth_write_enabled = Some(false);
        }
        Ok(())
    }
}

pub(crate) struct CloudsShaderPlugin;

impl Plugin for CloudsShaderPlugin {
    fn build(&self, app: &mut App) {
        load_shader_library!(app, "shaders/common.wgsl");

        embedded_asset!(app, "shaders/clouds.wgsl");
        embedded_asset!(app, "shaders/clouds_compute.wgsl");

        app.add_plugins(MaterialPlugin::<CloudsMaterial>::default());
    }
}