
use bevy_app::{App, Plugin};
use bevy_asset::{load_internal_asset, Handle};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    schedule::IntoSystemConfigs,
    system::{Commands, Query, Res, Resource},
    world::{FromWorld, World},
};
use bevy_reflect::Reflect;
use bevy_render::{
    render_resource::{
        binding_types::{sampler, texture_depth_2d, texture_storage_2d}, BindGroup, BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries, CachedComputePipelineId, ComputePipelineDescriptor, PipelineCache, PushConstantRange, SamplerBindingType, Shader, ShaderStages, StorageTextureAccess, TextureFormat
    },
    renderer::RenderDevice,
    view::ViewDepthTexture,
    Render, RenderApp, RenderSet,
};

use crate::prepass::ViewPrepassTextures;

pub struct HiZPlugin;

/// Maximum number of mipmaps supported by the wgpu version Bevy is using
/// The spec suggests that it should be possible to go higher
pub const HIZ_MIPMAP_COUNT: u32 = 11;

const HIZ_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(3904150601);

impl Plugin for HiZPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        load_internal_asset!(app, HIZ_SHADER_HANDLE, "hiz.wgsl", Shader::from_wgsl);

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app.add_systems(
            Render,
            (prepare_hiz_bind_groups.in_set(RenderSet::PrepareBindGroups),),
        );
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.init_resource::<HiZPrepassPipeline>();
    }
}

#[derive(Component, Default, Reflect, Clone)]
pub struct HiZ;

#[derive(Resource)]
pub struct HiZPrepassPipeline {
    pub bind_group_layout: BindGroupLayout,
    pub downsample_depth_first: CachedComputePipelineId,
    pub downsample_depth_second: CachedComputePipelineId,
    pub work_group_size: u32,
}

#[derive(Component)]
pub struct HiZPrepassBindGroup(pub BindGroup);

impl FromWorld for HiZPrepassPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let bind_group_layout = render_device.create_bind_group_layout(
            "hiz_downsample_depth_bind_group_layout",
            &BindGroupLayoutEntries::sequential(ShaderStages::COMPUTE, {
                let write_only_r32float = || {
                    texture_storage_2d(TextureFormat::R32Float, StorageTextureAccess::WriteOnly)
                };
                (
                    texture_depth_2d(),
                    write_only_r32float(),
                    write_only_r32float(),
                    write_only_r32float(),
                    write_only_r32float(),
                    write_only_r32float(),
                    texture_storage_2d(
                        TextureFormat::R32Float,
                        StorageTextureAccess::ReadWrite,
                    ),
                    write_only_r32float(),
                    write_only_r32float(),
                    write_only_r32float(),
                    write_only_r32float(),
                    write_only_r32float(),
                    write_only_r32float(),
                    sampler(SamplerBindingType::NonFiltering),
                )
            }),
        );
        let pipeline_cache = world.resource::<PipelineCache>();
        let downsample_depth_first = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some("hiz_downsample_depth_first_pipeline".into()),
            layout: vec![bind_group_layout.clone()],
            push_constant_ranges: vec![PushConstantRange {
                stages: ShaderStages::COMPUTE,
                range: 0..4,
            }],
            shader: HIZ_SHADER_HANDLE,
            shader_defs: vec![],
            entry_point: "downsample_depth_first".into(),
        });
        let downsample_depth_second = pipeline_cache.queue_compute_pipeline(
            ComputePipelineDescriptor {
                label: Some("hiz_downsample_depth_second_pipeline".into()),
                layout: vec![bind_group_layout.clone()],
                push_constant_ranges: vec![PushConstantRange {
                    stages: ShaderStages::COMPUTE,
                    range: 0..4,
                }],
                shader: HIZ_SHADER_HANDLE,
                shader_defs: vec![],
                entry_point: "downsample_depth_second".into(),
            },
        );
        Self {
            bind_group_layout,
            downsample_depth_first,
            downsample_depth_second,
            work_group_size: 64,
        }
    }
}

fn prepare_hiz_bind_groups(
    mut commands: Commands,
    pipeline: Res<HiZPrepassPipeline>,
    render_device: Res<RenderDevice>,
    views: Query<(Entity, &ViewDepthTexture, &ViewPrepassTextures)>,
) {
    for (entity, view_depth_texture, view_prepass_textures) in &views {
        if let Some(depth) = &view_prepass_textures.depth {
            let bind_group = render_device.create_bind_group(
                Some("hiz_mipmap_bindgroups"),
                &pipeline.bind_group_layout,
                &BindGroupEntries::sequential((
                    view_depth_texture,
                    &depth.texture.views[0],
                    &depth.texture.views[1],
                    &depth.texture.views[2],
                    &depth.texture.views[3],
                    &depth.texture.views[4],
                    &depth.texture.views[5],
                    &depth.texture.views[6],
                    &depth.texture.views[7],
                    &depth.texture.views[8],
                    &depth.texture.views[9],
                    &depth.texture.views[10],
                    &depth.texture.views[11],
                    &gpu_scene.depth_pyramid_sampler,
            )));
            println!("Added {} views.", depth.texture.views.len());
            commands.entity(entity).insert(HiZPrepassBindGroup(bind_group));
        }
    }
}
