//! 小地图插件
//!
//! 管理小地图的创建、渲染、视野指示器和点击导航。

use bevy::{
    asset::{Assets, Handle, RenderAssetUsages},
    camera::{
        Camera, Camera2d, OrthographicProjection, Projection, RenderTarget,
        visibility::RenderLayers,
    },
    color::Color,
    image::Image,
    math::{Rect, Vec2, Vec3},
    mesh::{Mesh, Mesh2d},
    picking::{
        Pickable,
        events::{Click, Pointer},
        pointer::PointerButton,
    },
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
    sprite_render::{ColorMaterial, MeshMaterial2d},
    ui::{
        BackgroundColor, BorderColor, Node, Overflow, OverflowAxis, PositionType, UiRect, Val,
        widget::{ImageNode, NodeImageMode, Text},
    },
    utils::default,
};
use civ_map_generator::{
    grid::Grid,
    ruleset::enums::{BaseTerrain, EnumStr},
    tile::Tile,
};
use enum_map::{EnumMap, enum_map};

use crate::{
    AppState, ScreenState,
    assets::{GameAssets, hex_mesh},
    components::{InfoPanel, MainCamera},
    resources::TileMapRes,
};

const MINIMAP_WIDTH: f32 = 300.;
const MINIMAP_HEIGHT: f32 = 200.;

/// 视野指示器
#[derive(Component)]
struct FieldOfViewIndicator;

/// 辅助视野指示器（用于地图环绕显示）
#[derive(Component)]
struct AuxiliaryFovIndicator;

/// 小地图插件
pub struct MinimapPlugin;

impl Plugin for MinimapPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(AppState::GameStart),
            (setup_minimap, setup_info_panel),
        )
        .add_systems(
            Update,
            (minimap_fov_update).run_if(in_state(ScreenState::WorldMap)),
        )
        .add_systems(OnExit(AppState::MapGenerating), spawn_tile_map_for_minimap);
    }
}

/// 设置小地图
fn setup_minimap(
    mut commands: Commands,
    tile_map: Option<Res<TileMapRes>>,
    mut images: ResMut<Assets<Image>>,
    main_camera_query: Single<&Camera, With<MainCamera>>,
) {
    let Some(tile_map) = tile_map else {
        return;
    };

    let tile_map = &tile_map.0;
    let grid = tile_map.world_grid.grid;

    let minimap_grid = grid.with_resized_layout([10., 10.]);

    let minimap_center = minimap_grid.center();
    let minimap_width = minimap_center[0] * 2.0;
    let minimap_height = minimap_center[1] * 2.0;

    let size = Extent3d {
        width: minimap_width as u32,
        height: minimap_height as u32,
        ..default()
    };

    let mut image = Image::new_uninit(
        size,
        TextureDimension::D2,
        TextureFormat::Bgra8UnormSrgb,
        RenderAssetUsages::all(),
    );

    image.texture_descriptor.usage =
        TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::RENDER_ATTACHMENT;

    let image_handle = images.add(image);

    commands.spawn((
        Camera2d,
        Camera {
            order: -1,
            ..default()
        },
        RenderTarget::Image(image_handle.clone().into()),
        Projection::Orthographic(OrthographicProjection {
            area: Rect {
                min: Vec2::new(0., 0.),
                max: Vec2::new(minimap_width, minimap_height),
            },
            ..OrthographicProjection::default_2d()
        }),
        Transform::from_xyz(minimap_center[0], minimap_center[1], 0.0),
        RenderLayers::layer(1),
    ));

    let world_grid_center = tile_map.world_grid.grid.center();

    let [world_grid_width, world_grid_height] =
        [world_grid_center[0] * 2.0, world_grid_center[1] * 2.0];

    let logical_viewport_size = main_camera_query
        .into_inner()
        .logical_viewport_size()
        .unwrap();

    let fov_indicator_width = logical_viewport_size.x / world_grid_width * MINIMAP_WIDTH;
    let fov_indicator_height = logical_viewport_size.y / world_grid_height * MINIMAP_HEIGHT;

    let default_fov_indicator_size = DefaultFovIndicatorSize {
        width: fov_indicator_width,
        height: fov_indicator_height,
    };

    commands.insert_resource(default_fov_indicator_size);

    let minimap = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(20.0),
                top: Val::Px(20.0),
                width: Val::Px(MINIMAP_WIDTH),
                height: Val::Px(MINIMAP_HEIGHT),
                border: UiRect::all(Val::Px(2.0)),
                overflow: Overflow {
                    x: OverflowAxis::Clip,
                    y: OverflowAxis::Clip,
                },
                ..Default::default()
            },
            BorderColor::all(Color::BLACK),
            ImageNode::new(image_handle).with_mode(NodeImageMode::Stretch),
        ))
        .observe(minimap_click_handler)
        .id();

    let mut field_of_view_indicator = Entity::PLACEHOLDER;

    commands.entity(minimap).with_children(|parent| {
        field_of_view_indicator = parent
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(MINIMAP_WIDTH / 2.0 - fov_indicator_width / 2.0),
                    bottom: Val::Px(MINIMAP_HEIGHT / 2.0 - fov_indicator_height / 2.0),
                    width: Val::Px(fov_indicator_width),
                    height: Val::Px(fov_indicator_height),
                    border: UiRect::all(Val::Px(2.0)),
                    ..Default::default()
                },
                BorderColor::all(Color::WHITE),
                Pickable::IGNORE,
                FieldOfViewIndicator,
            ))
            .id();
    });

    commands
        .entity(field_of_view_indicator)
        .with_children(|parent| {
            if grid.wrap_x() {
                parent.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        right: Val::Px(MINIMAP_WIDTH),
                        width: Val::Px(fov_indicator_width),
                        height: Val::Px(fov_indicator_height),
                        border: UiRect::all(Val::Px(2.0)),
                        ..Default::default()
                    },
                    BorderColor::all(Color::WHITE),
                    AuxiliaryFovIndicator,
                ));
                parent.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        right: Val::Px(-MINIMAP_WIDTH),
                        width: Val::Px(fov_indicator_width),
                        height: Val::Px(fov_indicator_height),
                        border: UiRect::all(Val::Px(2.0)),
                        ..Default::default()
                    },
                    BorderColor::all(Color::WHITE),
                    AuxiliaryFovIndicator,
                ));
            }

            if grid.wrap_y() {
                parent.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        bottom: Val::Px(MINIMAP_HEIGHT),
                        width: Val::Px(fov_indicator_width),
                        height: Val::Px(fov_indicator_height),
                        border: UiRect::all(Val::Px(2.0)),
                        ..Default::default()
                    },
                    BorderColor::all(Color::WHITE),
                    AuxiliaryFovIndicator,
                ));
                parent.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        bottom: Val::Px(-MINIMAP_HEIGHT),
                        width: Val::Px(fov_indicator_width),
                        height: Val::Px(fov_indicator_height),
                        border: UiRect::all(Val::Px(2.0)),
                        ..Default::default()
                    },
                    BorderColor::all(Color::WHITE),
                    AuxiliaryFovIndicator,
                ));
            }
        });
}

/// 为小地图创建地块渲染
fn spawn_tile_map_for_minimap(
    mut commands: Commands,
    tile_map: Option<Res<TileMapRes>>,
    materials: Res<GameAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut color_materials: ResMut<Assets<ColorMaterial>>,
) {
    let Some(tile_map) = tile_map else {
        return;
    };

    let tile_map = &tile_map.0;
    let grid = tile_map.world_grid.grid;

    let base_terrain_and_material: EnumMap<BaseTerrain, Handle<ColorMaterial>> = enum_map! {
        base_terrain => color_materials.add(materials.texture_handle(base_terrain.as_str())),
    };

    let minimap_grid = grid.with_resized_layout([10., 10.]);
    let hex_mesh = meshes.add(hex_mesh(&minimap_grid));

    for tile in tile_map.all_tiles() {
        let offset_coordinate = tile.to_offset(minimap_grid);
        let pixel_position = minimap_grid.offset_to_pixel(offset_coordinate);
        commands.spawn((
            Mesh2d(hex_mesh.clone()),
            MeshMaterial2d(base_terrain_and_material[tile.base_terrain(tile_map)].clone()),
            Transform {
                translation: Vec3::from((pixel_position[0], pixel_position[1], 9.)),
                ..Default::default()
            },
            RenderLayers::layer(1),
        ));
    }
}

/// 小地图点击处理
fn minimap_click_handler(
    click: On<Pointer<Click>>,
    mut set: ParamSet<(
        Single<&mut Node, With<FieldOfViewIndicator>>,
        Query<&mut Node, With<AuxiliaryFovIndicator>>,
    )>,
    main_camera_query: Single<(&mut Transform, &Projection), With<MainCamera>>,
    tile_map: Option<Res<TileMapRes>>,
    default_fov_indicator_size: Res<DefaultFovIndicatorSize>,
) {
    let Some(tile_map) = tile_map else {
        return;
    };

    let tile_map = &tile_map.0;
    let fov_width = default_fov_indicator_size.width;
    let fov_height = default_fov_indicator_size.height;

    let grid = tile_map.world_grid.grid;
    let width = grid.center()[0] * 2.0;
    let height = grid.center()[1] * 2.0;

    let (mut camera_transform, projection) = main_camera_query.into_inner();

    if matches!(click.button, PointerButton::Primary)
        && let Projection::Orthographic(orthographic) = projection
    {
        let scale = orthographic.scale;

        let drag_position = click.hit.position.unwrap().truncate();
        let normalized_drag_position = Vec2::new(drag_position[0] + 0.5, -drag_position[1] + 0.5);

        camera_transform.translation.x = normalized_drag_position[0] * width;
        camera_transform.translation.y = normalized_drag_position[1] * height;

        let mut minimap_indicator_node = set.p0().into_inner();
        minimap_indicator_node.left =
            Val::Px(normalized_drag_position[0] * MINIMAP_WIDTH - fov_width / 2.0 * scale);
        minimap_indicator_node.bottom =
            Val::Px(normalized_drag_position[1] * MINIMAP_HEIGHT - fov_height / 2.0 * scale);
        minimap_indicator_node.width = Val::Px(fov_width * scale);
        minimap_indicator_node.height = Val::Px(fov_height * scale);

        set.p1().iter_mut().for_each(|mut node| {
            node.width = Val::Px(fov_width * scale);
            node.height = Val::Px(fov_height * scale);
        });
    }
}

/// 主镜头移动或缩放时小地图视野更新
fn minimap_fov_update(
    main_camera_query: Single<(&Transform, &Projection), (Changed<Camera>, With<MainCamera>)>,
    tile_map: Option<Res<TileMapRes>>,
    mut set: ParamSet<(
        Single<&mut Node, With<FieldOfViewIndicator>>,
        Query<&mut Node, With<AuxiliaryFovIndicator>>,
    )>,
    default_fov_indicator_size: Res<DefaultFovIndicatorSize>,
) {
    let Some(tile_map) = tile_map else {
        return;
    };

    let tile_map = &tile_map.0;
    let grid = tile_map.world_grid.grid;
    let width = grid.center()[0] * 2.0;
    let height = grid.center()[1] * 2.0;

    let (camera_transform, projection) = main_camera_query.into_inner();

    let scale = if let Projection::Orthographic(orthographic) = projection {
        orthographic.scale
    } else {
        1.0
    };

    let fov_width = default_fov_indicator_size.width;
    let fov_height = default_fov_indicator_size.height;

    let camera_position = camera_transform.translation.truncate().to_array();
    let mut camera_offset_coordinate = grid.pixel_to_offset(camera_position);

    if !grid.wrap_x() {
        camera_offset_coordinate.0.x = camera_offset_coordinate
            .0
            .x
            .clamp(0, grid.width() as i32 - 1);
    }

    if !grid.wrap_y() {
        camera_offset_coordinate.0.y = camera_offset_coordinate
            .0
            .y
            .clamp(0, grid.height() as i32 - 1);
    }

    let tile = Tile::from_offset(camera_offset_coordinate, grid);
    let offset_coordinate = tile.to_offset(grid);
    let pixel_position = grid.offset_to_pixel(offset_coordinate);
    let normalized_drag_position = Vec2::new(pixel_position[0] / width, pixel_position[1] / height);

    let mut minimap_indicator_node = set.p0().into_inner();
    minimap_indicator_node.left =
        Val::Px(normalized_drag_position[0] * MINIMAP_WIDTH - fov_width / 2.0 * scale);
    minimap_indicator_node.bottom =
        Val::Px(normalized_drag_position[1] * MINIMAP_HEIGHT - fov_height / 2.0 * scale);
    minimap_indicator_node.width = Val::Px(fov_width * scale);
    minimap_indicator_node.height = Val::Px(fov_height * scale);

    set.p1().iter_mut().for_each(|mut node| {
        node.width = Val::Px(fov_width * scale);
        node.height = Val::Px(fov_height * scale);
    });
}

/// 信息面板组件
pub fn setup_info_panel(mut commands: Commands) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(10.0),
            bottom: Val::Px(10.0),
            width: Val::Auto,
            height: Val::Auto,
            border: UiRect::all(Val::Px(2.0)),
            ..Default::default()
        },
        BackgroundColor(Color::BLACK),
        BorderColor::all(Color::WHITE),
        Text::new("info panel"),
        TextFont {
            font_size: FontSize::Px(14.0),
            ..Default::default()
        },
        TextColor(Color::WHITE),
        InfoPanel,
    ));
}

/// 默认视野指示器尺寸
#[derive(Resource, Default)]
struct DefaultFovIndicatorSize {
    /// 宽度
    pub width: f32,
    /// 高度
    pub height: f32,
}
