use bevy::{
    asset::RenderAssetUsages,
    core_pipeline::core_2d::Transparent2d,
    ecs::{
        query::QueryItem,
        system::{SystemParamItem, lifetimeless::*},
    },
    image::ImageSampler,
    math::FloatOrd,
    mesh::{MeshVertexBufferLayoutRef, VertexBufferLayout},
    prelude::*,
    render::{
        Render, RenderApp, RenderStartup, RenderSystems,
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        render_asset::RenderAssets,
        render_phase::*,
        render_resource::*,
        renderer::RenderDevice,
        sync_world::MainEntity,
        view::ExtractedView,
    },
    sprite_render::{
        Mesh2dPipeline, Mesh2dPipelineKey, RenderMesh2dInstances, SetMesh2dBindGroup,
        SetMesh2dViewBindGroup, init_mesh_2d_pipeline,
    },
};
use bytemuck::{Pod, Zeroable};

use crate::grid::Grid;
use crate::grid_coloration::ColorGradient;
use crate::ui::UiMetrics;

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct CellInstance {
    pub position: Vec2,
    pub cell_size: f32,
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub smooth: f32,
}

#[derive(Component, Deref)]
pub struct CellInstanceData(pub Vec<CellInstance>);

impl ExtractComponent for CellInstanceData {
    type QueryData = &'static CellInstanceData;
    type QueryFilter = ();
    type Out = Self;
    fn extract_component(item: QueryItem<'_, '_, Self::QueryData>) -> Option<Self> {
        Some(CellInstanceData(item.0.clone()))
    }
}

#[derive(Component)]
pub struct CellGrid;

#[derive(Component, Default, Deref, DerefMut)]
pub struct GradientIndex(pub usize);

impl ExtractComponent for GradientIndex {
    type QueryData = &'static GradientIndex;
    type QueryFilter = ();
    type Out = Self;
    fn extract_component(item: QueryItem<'_, '_, Self::QueryData>) -> Option<Self> {
        Some(GradientIndex(item.0))
    }
}

#[derive(Component, Default, Deref, DerefMut)]
pub struct SmoothMode(pub bool);

impl ExtractComponent for SmoothMode {
    type QueryData = &'static SmoothMode;
    type QueryFilter = ();
    type Out = Self;
    fn extract_component(item: QueryItem<'_, '_, Self::QueryData>) -> Option<Self> {
        Some(SmoothMode(item.0))
    }
}

#[derive(Resource)]
pub struct HeatmapTexture(pub Handle<Image>);

#[derive(Component)]
pub struct HeatmapSprite;

pub struct CellMaterialPlugin;
impl Plugin for CellMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractComponentPlugin::<CellInstanceData>::default());
        app.add_plugins(ExtractComponentPlugin::<GradientIndex>::default());
        app.add_plugins(ExtractComponentPlugin::<SmoothMode>::default());
        app.add_systems(Startup, setup);
        app.add_systems(
            Update,
            (
                rebuild_grid_instances,
                sync_camera.after(rebuild_grid_instances),
                update_cell_textures,
            ),
        );
        app.add_systems(FixedUpdate, update_instance_data);

        app.sub_app_mut(RenderApp)
            .add_render_command::<Transparent2d, DrawCells>()
            .init_resource::<SpecializedMeshPipelines<CellPipeline>>()
            .add_systems(
                RenderStartup,
                init_cell_pipeline.after(init_mesh_2d_pipeline),
            )
            .add_systems(
                Render,
                (
                    queue_cells.in_set(RenderSystems::QueueMeshes),
                    prepare_cell_buffers.in_set(RenderSystems::PrepareResources),
                ),
            );
    }
}

/// Align the top-left corner of the cell grid with the top-left corner of the
/// viewport. The grid is rounded up to whole cells, so it may extend slightly
/// past the right/bottom edges — anchoring the top-left keeps that overflow
/// off-screen instead of leaving a gap next to the panel.
fn sync_camera(
    metrics: Res<UiMetrics>,
    grid: Res<Grid>,
    windows: Query<&Window>,
    mut cameras: Query<&mut Transform, With<Camera2d>>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let window_size = window.resolution.size();
    let grid_size = Vec2::new(
        grid.width as f32 * grid.cell_size,
        grid.height as f32 * grid.cell_size,
    );
    let position = Vec2::new(
        window_size.x / 2.0 - grid_size.x / 2.0 - metrics.panel_width,
        metrics.topbar_height - window_size.y / 2.0 + grid_size.y / 2.0,
    );

    for mut transform in &mut cameras {
        transform.translation.x = position.x;
        transform.translation.y = position.y;
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    windows: Query<&Window>,
    metrics: Res<UiMetrics>,
) {
    let window = windows.single().unwrap();
    let base_cell_size = 12.0;
    let visible_width = window.resolution.width() - metrics.panel_width;
    let visible_height = window.resolution.height() - metrics.topbar_height - metrics.status_height;
    let grid_width = (visible_width / base_cell_size) as usize;
    let grid_height = (visible_height / base_cell_size) as usize;
    let grid = Grid::new(grid_width.max(1), grid_height.max(1), base_cell_size);

    commands.spawn((
        Camera2d,
        Transform::from_xyz(
            -metrics.panel_width / 2.0,
            (metrics.topbar_height - metrics.status_height) / 2.0,
            0.0,
        ),
    ));

    let num_channels = grid.rule.num_channels;

    let instances: Vec<CellInstance> = (0..grid.width * grid.height)
        .map(|i| {
            let x = (i % grid.width) as f32 * base_cell_size
                - (grid.width as f32 * base_cell_size) / 2.0
                + base_cell_size / 2.0;
            let y = (i / grid.width) as f32 * base_cell_size
                - (grid.height as f32 * base_cell_size) / 2.0
                + base_cell_size / 2.0;
            let (r, g, b) = channels_to_rgb_flat(
                &grid.cell_data,
                i,
                num_channels,
                &grid.grid_coloration.gradient,
            );
            CellInstance {
                position: Vec2::new(x, y),
                cell_size: base_cell_size,
                r,
                g,
                b,
                smooth: if grid.grid_coloration.smooth {
                    1.0
                } else {
                    0.0
                },
            }
        })
        .collect();

    let mesh = meshes.add(Rectangle::new(1.0, 1.0));

    commands.spawn((
        Mesh2d(mesh),
        CellInstanceData(instances),
        CellGrid,
        GradientIndex(0),
        SmoothMode(false),
        Transform::default(),
        GlobalTransform::default(),
        Visibility::default(),
    ));

    let heat_image = Image::new(
        Extent3d {
            width: grid_width.max(1) as u32,
            height: grid_height.max(1) as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        vec![0u8; grid_width.max(1) * grid_height.max(1) * 4],
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::default(),
    );
    let image_handle = images.add(heat_image);
    commands.insert_resource(HeatmapTexture(image_handle.clone()));
    commands.spawn((
        Sprite {
            image: image_handle,
            custom_size: Some(Vec2::new(
                grid_width.max(1) as f32 * base_cell_size,
                grid_height.max(1) as f32 * base_cell_size,
            )),
            ..default()
        },
        Transform::default(),
        GlobalTransform::default(),
        Visibility::Hidden,
        HeatmapSprite,
    ));

    commands.insert_resource(grid);
}

fn channels_to_rgb_flat(
    cell_data: &[f32],
    idx: usize,
    num_channels: usize,
    gradient: &ColorGradient,
) -> (f32, f32, f32) {
    if num_channels == 0 {
        return (0.0, 0.0, 0.0);
    }
    let base = idx * num_channels;
    if num_channels >= 3 {
        return (cell_data[base], cell_data[base + 1], cell_data[base + 2]);
    }
    if num_channels == 2 {
        let color = gradient.lerp(cell_data[base]);
        return (
            color.red * cell_data[base],
            color.green * cell_data[base + 1],
            color.blue * 0.5,
        );
    }
    let color = gradient.lerp(cell_data[base]);
    (color.red, color.green, color.blue)
}

pub fn update_instance_data(
    grid: Option<Res<Grid>>,
    mut query: Query<&mut CellInstanceData, With<CellGrid>>,
    mut gradient_query: Query<&mut GradientIndex, With<CellGrid>>,
    mut smooth_query: Query<&mut SmoothMode, With<CellGrid>>,
) {
    let Some(grid) = grid else {
        return;
    };
    let Ok(mut instance_data) = query.single_mut() else {
        return;
    };
    let smooth = if grid.grid_coloration.smooth {
        1.0
    } else {
        0.0
    };
    let num_channels = grid.rule.num_channels;
    for (i, inst) in instance_data.0.iter_mut().enumerate() {
        let (r, g, b) = channels_to_rgb_flat(
            &grid.cell_data,
            i,
            num_channels,
            &grid.grid_coloration.gradient,
        );
        inst.r = r;
        inst.g = g;
        inst.b = b;
        inst.smooth = smooth;
    }
    if let Ok(mut gradient_idx) = gradient_query.single_mut() {
        gradient_idx.0 = ColorGradient::all()
            .iter()
            .position(|g| g.name == grid.grid_coloration.gradient.name)
            .unwrap_or(0);
    }
    if let Ok(mut smooth_mode) = smooth_query.single_mut() {
        smooth_mode.0 = grid.grid_coloration.smooth;
    }
}

fn catmull_rom(t: f32, p0: f32, p1: f32, p2: f32, p3: f32) -> f32 {
    0.5 * (2.0 * p1
        + (-p0 + p2) * t
        + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t * t
        + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t * t * t)
}

fn sample_bicubic(
    cell_data: &[f32],
    w: usize,
    h: usize,
    ch: usize,
    num_ch: usize,
    gx: f32,
    gy: f32,
) -> f32 {
    let ix = gx.floor() as i32;
    let iy = gy.floor() as i32;
    let fx = gx - ix as f32;
    let fy = gy - iy as f32;

    let mut col_vals = [0.0f32; 4];
    for ri in 0..4usize {
        let yy = (iy + ri as i32 - 1).clamp(0, h as i32 - 1) as usize;
        let row_base = yy * w;
        let mut row_vals = [0.0f32; 4];
        for ci in 0..4usize {
            let xx = (ix + ci as i32 - 1).clamp(0, w as i32 - 1) as usize;
            row_vals[ci] = cell_data[(row_base + xx) * num_ch + ch];
        }
        col_vals[ri] = catmull_rom(fx, row_vals[0], row_vals[1], row_vals[2], row_vals[3]);
    }
    catmull_rom(fy, col_vals[0], col_vals[1], col_vals[2], col_vals[3])
}

pub fn update_cell_textures(
    grid: Option<Res<Grid>>,
    mut images: ResMut<Assets<Image>>,
    heatmap_tex: Res<HeatmapTexture>,
    mut sprite_query: Query<(&mut Sprite, &mut Visibility), With<HeatmapSprite>>,
) {
    let Some(grid) = grid else { return };
    let Ok((mut sprite, mut visibility)) = sprite_query.single_mut() else {
        return;
    };

    // The bicubic resample is expensive; only run it when the smooth view is active.
    if !grid.grid_coloration.smooth {
        *visibility = Visibility::Hidden;
        return;
    }

    let max_tex = 800u32;
    let tex_w = ((grid.width as f32 * grid.cell_size).ceil() as u32).min(max_tex);
    let tex_h = ((grid.height as f32 * grid.cell_size).ceil() as u32).min(max_tex);
    let tex_size = (tex_w * tex_h * 4) as usize;

    let needs_resize = images.get(&heatmap_tex.0).map_or(true, |img| {
        img.data.as_ref().map_or(true, |d| d.len() != tex_size)
    });
    if needs_resize {
        let new_image = Image::new(
            Extent3d {
                width: tex_w,
                height: tex_h,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            vec![0u8; tex_size],
            TextureFormat::Rgba8Unorm,
            RenderAssetUsages::default(),
        );
        let _ = images.insert(&heatmap_tex.0, new_image);
    }

    let Some(image) = images.get_mut(&heatmap_tex.0) else {
        return;
    };
    let num_channels = grid.rule.num_channels;
    let gradient = &grid.grid_coloration.gradient;
    let w_f = (grid.width - 1).max(1) as f32;
    let h_f = (grid.height - 1).max(1) as f32;
    let tw_f = (tex_w - 1) as f32;
    let th_f = (tex_h - 1) as f32;

    if let Some(data) = &mut image.data {
        if num_channels >= 3 {
            for py in 0..tex_h {
                for px in 0..tex_w {
                    let gx = px as f32 / tw_f * w_f;
                    let gy = h_f - py as f32 / th_f * h_f;
                    let r = sample_bicubic(
                        &grid.cell_data,
                        grid.width,
                        grid.height,
                        0,
                        num_channels,
                        gx,
                        gy,
                    )
                    .clamp(0.0, 1.0);
                    let g = sample_bicubic(
                        &grid.cell_data,
                        grid.width,
                        grid.height,
                        1,
                        num_channels,
                        gx,
                        gy,
                    )
                    .clamp(0.0, 1.0);
                    let b = sample_bicubic(
                        &grid.cell_data,
                        grid.width,
                        grid.height,
                        2,
                        num_channels,
                        gx,
                        gy,
                    )
                    .clamp(0.0, 1.0);
                    let off = (py * tex_w + px) as usize * 4;
                    data[off] = (r * 255.0) as u8;
                    data[off + 1] = (g * 255.0) as u8;
                    data[off + 2] = (b * 255.0) as u8;
                    data[off + 3] = 255;
                }
            }
        } else if num_channels == 2 {
            for py in 0..tex_h {
                for px in 0..tex_w {
                    let gx = px as f32 / tw_f * w_f;
                    let gy = h_f - py as f32 / th_f * h_f;
                    let v0 = sample_bicubic(
                        &grid.cell_data,
                        grid.width,
                        grid.height,
                        0,
                        num_channels,
                        gx,
                        gy,
                    )
                    .clamp(0.0, 1.0);
                    let v1 = sample_bicubic(
                        &grid.cell_data,
                        grid.width,
                        grid.height,
                        1,
                        num_channels,
                        gx,
                        gy,
                    )
                    .clamp(0.0, 1.0);
                    let color = gradient.lerp(v0);
                    let off = (py * tex_w + px) as usize * 4;
                    data[off] = ((color.red * v0).clamp(0.0, 1.0) * 255.0) as u8;
                    data[off + 1] = ((color.green * v1).clamp(0.0, 1.0) * 255.0) as u8;
                    data[off + 2] = ((color.blue * 0.5).clamp(0.0, 1.0) * 255.0) as u8;
                    data[off + 3] = 255;
                }
            }
        } else {
            for py in 0..tex_h {
                for px in 0..tex_w {
                    let gx = px as f32 / tw_f * w_f;
                    let gy = h_f - py as f32 / th_f * h_f;
                    let v = sample_bicubic(
                        &grid.cell_data,
                        grid.width,
                        grid.height,
                        0,
                        num_channels,
                        gx,
                        gy,
                    )
                    .clamp(0.0, 1.0);
                    let color = gradient.lerp(v);
                    let off = (py * tex_w + px) as usize * 4;
                    data[off] = (color.red * 255.0) as u8;
                    data[off + 1] = (color.green * 255.0) as u8;
                    data[off + 2] = (color.blue * 255.0) as u8;
                    data[off + 3] = 255;
                }
            }
        }
    }

    image.sampler = ImageSampler::nearest();

    let sprite_size = Vec2::new(
        grid.width as f32 * grid.cell_size,
        grid.height as f32 * grid.cell_size,
    );
    sprite.custom_size = Some(sprite_size);

    *visibility = if grid.grid_coloration.smooth {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
}

pub fn rebuild_grid_instances(
    mut grid: ResMut<Grid>,
    mut query: Query<(&mut CellInstanceData, &mut GradientIndex), With<CellGrid>>,
    mut smooth_query: Query<&mut SmoothMode, With<CellGrid>>,
    windows: Query<&Window>,
    metrics: Res<UiMetrics>,
) {
    let window = windows.single().unwrap();
    let visible_width = window.resolution.width() - metrics.panel_width;
    let visible_height = window.resolution.height() - metrics.topbar_height - metrics.status_height;

    let width = ((visible_width / grid.cell_size).ceil() as usize).max(1);
    let height = ((visible_height / grid.cell_size).ceil() as usize).max(1);

    if grid.width == width && grid.height == height && !grid.needs_rebuild() {
        return;
    }

    let Ok((mut instance_data, mut gradient_idx)) = query.single_mut() else {
        return;
    };
    if let Ok(mut smooth_mode) = smooth_query.single_mut() {
        smooth_mode.0 = grid.grid_coloration.smooth;
    }

    let num_channels = grid.rule.num_channels;
    let total_cells = width * height;
    let cell_size = grid.cell_size;
    grid.resize_preserving(width, height, cell_size);

    let instances: Vec<CellInstance> = (0..total_cells)
        .map(|i| {
            let x = (i % width) as f32 * grid.cell_size - (width as f32 * grid.cell_size) / 2.0
                + grid.cell_size / 2.0;
            let y = (i / width) as f32 * grid.cell_size - (height as f32 * grid.cell_size) / 2.0
                + grid.cell_size / 2.0;
            let (r, g, b) = channels_to_rgb_flat(
                &grid.cell_data,
                i,
                num_channels,
                &grid.grid_coloration.gradient,
            );
            CellInstance {
                position: Vec2::new(x, y),
                cell_size: grid.cell_size,
                r,
                g,
                b,
                smooth: if grid.grid_coloration.smooth {
                    1.0
                } else {
                    0.0
                },
            }
        })
        .collect();

    instance_data.0 = instances;
    gradient_idx.0 = 0;
}

#[derive(Component)]
pub struct CellInstanceBuffer {
    buffer: Buffer,
    length: usize,
}

fn prepare_cell_buffers(
    mut commands: Commands,
    query: Query<(Entity, &CellInstanceData)>,
    render_device: Res<RenderDevice>,
) {
    let items: Vec<_> = query.iter().collect();
    for (entity, data) in items {
        let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("cell instance buffer"),
            contents: bytemuck::cast_slice(&data.0),
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        });
        commands.entity(entity).insert(CellInstanceBuffer {
            buffer,
            length: data.0.len(),
        });
    }
}

#[derive(Resource)]
struct CellPipeline {
    shaders: Vec<Handle<Shader>>,
    mesh2d_pipeline: Mesh2dPipeline,
}

fn init_cell_pipeline(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mesh2d_pipeline: Res<Mesh2dPipeline>,
) {
    let shaders = ColorGradient::all()
        .iter()
        .map(|g| asset_server.load(format!("shaders/{}.wgsl", g.name.to_lowercase())))
        .collect();
    commands.insert_resource(CellPipeline {
        shaders,
        mesh2d_pipeline: mesh2d_pipeline.clone(),
    });
}

impl SpecializedMeshPipeline for CellPipeline {
    type Key = (Mesh2dPipelineKey, usize);

    fn specialize(
        &self,
        (key, gradient_idx): Self::Key,
        layout: &MeshVertexBufferLayoutRef,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mut desc = self.mesh2d_pipeline.specialize(key, layout)?;
        let shader = self.shaders[gradient_idx].clone();
        desc.vertex.shader = shader.clone();
        desc.vertex.buffers.push(VertexBufferLayout {
            array_stride: std::mem::size_of::<CellInstance>() as u64,
            step_mode: VertexStepMode::Instance,
            attributes: vec![
                VertexAttribute {
                    format: VertexFormat::Float32x3,
                    offset: 0,
                    shader_location: 3,
                },
                VertexAttribute {
                    format: VertexFormat::Float32x3,
                    offset: 12,
                    shader_location: 4,
                },
                VertexAttribute {
                    format: VertexFormat::Float32,
                    offset: 24,
                    shader_location: 5,
                },
            ],
        });
        if let Some(fragment) = desc.fragment.as_mut() {
            fragment.shader = shader;
            for target in fragment.targets.iter_mut().flatten() {
                target.blend = Some(BlendState::REPLACE);
            }
        }
        Ok(desc)
    }
}

fn queue_cells(
    draw_functions: Res<DrawFunctions<Transparent2d>>,
    pipeline: Res<CellPipeline>,
    mut pipelines: ResMut<SpecializedMeshPipelines<CellPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    meshes: Res<RenderAssets<bevy::render::mesh::RenderMesh>>,
    render_mesh_instances: Res<RenderMesh2dInstances>,
    material_meshes: Query<(Entity, &MainEntity), With<CellInstanceData>>,
    mut transparent_phases: ResMut<ViewSortedRenderPhases<Transparent2d>>,
    views: Query<(&ExtractedView, &Msaa)>,
    gradient_query: Query<&GradientIndex, With<CellInstanceData>>,
    smooth_query: Query<&SmoothMode, With<CellInstanceData>>,
) {
    let draw_fn = draw_functions.read().id::<DrawCells>();
    let gradient_idx = gradient_query.iter().next().map(|g| **g).unwrap_or(0);
    let smooth = smooth_query.iter().next().map(|s| **s).unwrap_or(false);
    // Skip instanced rendering in smooth mode; sprite texture handles it
    if smooth {
        return;
    }
    let meshes_ref = &*meshes;
    for (view, msaa) in &views {
        let Some(phase) = transparent_phases.get_mut(&view.retained_view_entity) else {
            continue;
        };
        let msaa_key = Mesh2dPipelineKey::from_msaa_samples(msaa.samples());
        let view_key = msaa_key | Mesh2dPipelineKey::from_hdr(view.hdr);
        for (entity, main_entity) in &material_meshes {
            let Some(mesh_instance) = render_mesh_instances.get(main_entity) else {
                continue;
            };
            let Some(mesh) = meshes_ref.get(mesh_instance.mesh_asset_id) else {
                continue;
            };
            let key =
                view_key | Mesh2dPipelineKey::from_primitive_topology(mesh.primitive_topology());
            let pipeline_id = pipelines
                .specialize(
                    &pipeline_cache,
                    &pipeline,
                    (key, gradient_idx),
                    &mesh.layout,
                )
                .unwrap();
            phase.add(Transparent2d {
                entity: (entity, *main_entity),
                draw_function: draw_fn,
                pipeline: pipeline_id,
                sort_key: FloatOrd(0.0),
                batch_range: 0..1,
                extra_index: PhaseItemExtraIndex::None,
                extracted_index: 0,
                indexed: true,
            });
        }
    }
}

type DrawCells = (
    SetItemPipeline,
    SetMesh2dViewBindGroup<0>,
    SetMesh2dBindGroup<1>,
    DrawCellsInstanced,
);

struct DrawCellsInstanced;
impl<P: PhaseItem> RenderCommand<P> for DrawCellsInstanced {
    type Param = (
        SRes<RenderAssets<bevy::render::mesh::RenderMesh>>,
        SRes<RenderMesh2dInstances>,
        SRes<bevy::render::mesh::allocator::MeshAllocator>,
    );
    type ViewQuery = ();
    type ItemQuery = Read<CellInstanceBuffer>;

    fn render<'w>(
        item: &P,
        _view: (),
        instance_buffer: Option<&'w CellInstanceBuffer>,
        param: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let (meshes, mesh_instances, mesh_allocator) = param;
        let mesh_allocator = mesh_allocator.into_inner();
        let Some(mesh_instance) = mesh_instances.get(&item.main_entity()) else {
            return RenderCommandResult::Skip;
        };
        let meshes_ref = &*meshes;
        let Some(gpu_mesh) = meshes_ref.get(mesh_instance.mesh_asset_id) else {
            return RenderCommandResult::Skip;
        };
        let Some(inst_buf) = instance_buffer else {
            return RenderCommandResult::Skip;
        };
        let Some(vertex_slice) = mesh_allocator.mesh_vertex_slice(&mesh_instance.mesh_asset_id)
        else {
            return RenderCommandResult::Skip;
        };

        pass.set_vertex_buffer(0, vertex_slice.buffer.slice(..));
        pass.set_vertex_buffer(1, inst_buf.buffer.slice(..));

        match &gpu_mesh.buffer_info {
            bevy::render::mesh::RenderMeshBufferInfo::Indexed {
                index_format,
                count,
            } => {
                let Some(idx_slice) = mesh_allocator.mesh_index_slice(&mesh_instance.mesh_asset_id)
                else {
                    return RenderCommandResult::Skip;
                };
                pass.set_index_buffer(idx_slice.buffer.slice(..), 0, *index_format);
                pass.draw_indexed(
                    idx_slice.range.start..(idx_slice.range.start + count),
                    vertex_slice.range.start as i32,
                    0..inst_buf.length as u32,
                );
            }
            bevy::render::mesh::RenderMeshBufferInfo::NonIndexed => {
                pass.draw(vertex_slice.range, 0..inst_buf.length as u32);
            }
        }
        RenderCommandResult::Success
    }
}
