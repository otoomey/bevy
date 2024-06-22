use std::borrow::Cow;

use bevy_app::Plugin;
use bevy_asset::{load_internal_asset, Handle};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    schedule::IntoSystemConfigs,
    system::{Commands, Query, Res, Resource},
    world::{FromWorld, World},
};
use bevy_render::{
    render_resource::{
        binding_types::texture_storage_2d, BindGroup, BindGroupEntries, BindGroupLayout,
        BindGroupLayoutEntries, CachedComputePipelineId, ComputePipelineDescriptor, PipelineCache, Shader, ShaderStages, StorageTextureAccess,
    },
    renderer::RenderDevice,
    view::ViewDepthTexture,
    Render, RenderApp, RenderSet,
};

use crate::{core_3d::CORE_3D_DEPTH_FORMAT, prepass::ViewPrepassTextures};

pub struct HiZPlugin;

const HIZ_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(3904150601);

impl Plugin for HiZPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        load_internal_asset!(app, HIZ_SHADER_HANDLE, "hiz.wgsl", Shader::from_wgsl);

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .init_resource::<HiZPrepassPipeline>()
            .add_systems(
                Render,
                (prepare_hiz_bind_groups.in_set(RenderSet::PrepareBindGroups),),
            );
    }
}

#[derive(Default, Component, Clone)]
pub enum HiZ {
    Enabled(u32),
    #[default]
    Disabled,
}

#[derive(Resource)]
pub struct HiZPrepassPipeline {
    pub bind_group_layout: BindGroupLayout,
    pub pipeline_id: CachedComputePipelineId,
    pub work_group_size: u32,
}

#[derive(Component)]
pub struct HiZPrepassBindGroups {
    pub groups: Vec<BindGroup>,
}

impl FromWorld for HiZPrepassPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let bind_group_layout = render_device.create_bind_group_layout(
            "hiz_prepass_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (
                    texture_storage_2d(CORE_3D_DEPTH_FORMAT, StorageTextureAccess::ReadOnly),
                    texture_storage_2d(CORE_3D_DEPTH_FORMAT, StorageTextureAccess::WriteOnly),
                ),
            ),
        );
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline_id = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: None,
            layout: vec![bind_group_layout.clone()],
            push_constant_ranges: Vec::new(),
            shader: HIZ_SHADER_HANDLE,
            shader_defs: vec![],
            entry_point: Cow::from("computeHiZ"),
        });
        Self {
            bind_group_layout,
            pipeline_id,
            work_group_size: 8,
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
            let bind_groups = (0..depth.texture.views.len())
                .into_iter()
                .map(|i| {
                    render_device.create_bind_group(
                        Some("hiz_mipmap_bindgroups"),
                        &pipeline.bind_group_layout,
                        &BindGroupEntries::sequential((
                            if i == 0 {
                                &view_depth_texture.view()
                            } else {
                                &depth.texture.views[i - 1]
                            },
                            &depth.texture.views[i],
                        )),
                    )
                })
                .collect();
            commands.entity(entity).insert(HiZPrepassBindGroups {
                groups: bind_groups,
            });
        }
    }
}
