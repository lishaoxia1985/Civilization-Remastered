use bevy::{
    asset::{Assets, Handle, RenderAssetUsages},
    camera::{
        Camera, Camera2d, OrthographicProjection, Projection, RenderTarget,
        visibility::RenderLayers,
    },
    color::Color,
    ecs::{
        component::Component,
        entity::Entity,
        observer::On,
        query::{Changed, With, Without},
        resource::Resource,
        system::{Commands, Local, Query, Res, ResMut, Single},
    },
    image::Image,
    math::{Rect, Vec2, Vec3},
    mesh::{Mesh, Mesh2d},
    picking::{
        Pickable,
        events::{Click, Pointer},
        pointer::PointerButton,
    },
    render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
    sprite_render::{ColorMaterial, MeshMaterial2d},
    transform::components::Transform,
    ui::{
        BorderColor, Node, Overflow, OverflowAxis, PositionType, UiRect, Val,
        widget::{ImageNode, NodeImageMode},
    },
    utils::default,
};
use civ_map_generator::{grid::Grid, tile::Tile, tile_component::BaseTerrain};
use enum_map::{EnumMap, enum_map};

use crate::{MainCamera, TileMapResource, assets::MaterialResource, custom_mesh::hex_mesh};

#[derive(Component)]
pub struct FieldOfViewIndicator;

#[derive(Component)]
pub struct AuxiliaryFOVIndicator;

const MINIMAP_WIDTH: f32 = 300.;
const MINIMAP_HEIGHT: f32 = 200.;

#[derive(Resource)]
pub struct DefaultFovIndicatorSize {
    pub width: f32,
    pub height: f32,
}

pub fn setup_minimap(
    mut commands: Commands,
    map: Option<Res<TileMapResource>>,
    materials: Res<MaterialResource>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut color_materials: ResMut<Assets<ColorMaterial>>,
    mut enable_minimap: Local<bool>,
    query_main_camera: Single<&Camera, With<MainCamera>>,
) {
    if map.is_none() {
        return;
    };

    if *enable_minimap {
        return;
    }

    let tile_map = &map.unwrap().0;
    let grid = tile_map.world_grid.grid;

    let minimap_grid = grid.with_resized_layout([10., 10.]);

    let base_terrain_and_material: EnumMap<BaseTerrain, Handle<ColorMaterial>> = enum_map! {
        base_terrain => color_materials.add(materials.texture_handle(base_terrain.as_str())),
    };

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
            target: RenderTarget::Image(image_handle.clone().into()),
            order: -1,
            ..default()
        },
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

    let logical_viewport_size = query_main_camera
        .into_inner()
        .logical_viewport_size()
        .unwrap();

    let fov_indicator_width = logical_viewport_size.x / world_grid_width * MINIMAP_WIDTH;
    let fov_indicator_height = logical_viewport_size.y / world_grid_height * MINIMAP_HEIGHT;

    commands.insert_resource(DefaultFovIndicatorSize {
        width: fov_indicator_width,
        height: fov_indicator_height,
    });

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
                    AuxiliaryFOVIndicator,
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
                    AuxiliaryFOVIndicator,
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
                    AuxiliaryFOVIndicator,
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
                    AuxiliaryFOVIndicator,
                ));
            }
        });

    *enable_minimap = true;
}

fn minimap_click_handler(
    drag: On<Pointer<Click>>,
    query_main_camera: Single<(&mut Transform, &Projection), With<MainCamera>>,
    query_minimap_indicator: Single<&mut Node, With<FieldOfViewIndicator>>,
    mut query_auxiliary_fov_indicators: Query<
        &mut Node,
        (With<AuxiliaryFOVIndicator>, Without<FieldOfViewIndicator>),
    >,
    map: Option<Res<TileMapResource>>,
    fov_size: Res<DefaultFovIndicatorSize>,
) {
    if map.is_none() {
        return;
    };

    let fov_width = fov_size.width;
    let fov_height = fov_size.height;

    let tile_map = &map.unwrap().0;
    let grid = tile_map.world_grid.grid;
    let width = grid.center()[0] * 2.0;
    let height = grid.center()[1] * 2.0;

    let (mut camera_transform, projection) = query_main_camera.into_inner();

    if matches!(drag.button, PointerButton::Primary)
        && let Projection::Orthographic(orthographic) = projection
    {
        let scale = orthographic.scale;

        let drag_position = drag.hit.position.unwrap().truncate();
        // Invert the y-axis to match the world coordinate system
        let normalized_drag_position = Vec2::new(drag_position[0] + 0.5, -drag_position[1] + 0.5);

        camera_transform.translation.x = normalized_drag_position[0] * width;
        camera_transform.translation.y = normalized_drag_position[1] * height;

        let mut minimap_indicator_node = query_minimap_indicator.into_inner();
        minimap_indicator_node.left =
            Val::Px(normalized_drag_position[0] * MINIMAP_WIDTH - fov_width / 2.0 * scale);
        minimap_indicator_node.bottom =
            Val::Px(normalized_drag_position[1] * MINIMAP_HEIGHT - fov_height / 2.0 * scale);
        minimap_indicator_node.width = Val::Px(fov_width * scale);
        minimap_indicator_node.height = Val::Px(fov_height * scale);

        query_auxiliary_fov_indicators
            .iter_mut()
            .for_each(|mut node| {
                node.width = Val::Px(fov_width * scale);
                node.height = Val::Px(fov_height * scale);
            });
    }
}

pub fn minimap_fov_update(
    query: Single<(&Transform, &Projection), (Changed<Camera>, With<MainCamera>)>,
    map: Option<Res<TileMapResource>>,
    query_minimap_indicator: Single<&mut Node, With<FieldOfViewIndicator>>,
    mut query_auxiliary_fov_indicators: Query<
        &mut Node,
        (With<AuxiliaryFOVIndicator>, Without<FieldOfViewIndicator>),
    >,
    fov_size: Res<DefaultFovIndicatorSize>,
) {
    if map.is_none() {
        return;
    }

    let tile_map = &map.unwrap().0;
    let grid = tile_map.world_grid.grid;
    let width = grid.center()[0] * 2.0;
    let height = grid.center()[1] * 2.0;

    let (camera_transform, projection) = query.into_inner();

    let scale = if let Projection::Orthographic(orthographic) = projection {
        orthographic.scale
    } else {
        1.0
    };

    let fov_width = fov_size.width;
    let fov_height = fov_size.height;

    let camera_position = camera_transform.translation.truncate().to_array();
    let camera_offset_coordinate = grid.pixel_to_offset(camera_position);
    let tile = Tile::from_offset(camera_offset_coordinate, grid);
    let offset_coordinate = tile.to_offset(grid);
    let pixel_position = grid.offset_to_pixel(offset_coordinate);
    let normalized_drag_position = Vec2::new(pixel_position[0] / width, pixel_position[1] / height);

    let mut minimap_indicator_node = query_minimap_indicator.into_inner();
    minimap_indicator_node.left =
        Val::Px(normalized_drag_position[0] * MINIMAP_WIDTH - fov_width / 2.0 * scale);
    minimap_indicator_node.bottom =
        Val::Px(normalized_drag_position[1] * MINIMAP_HEIGHT - fov_height / 2.0 * scale);
    minimap_indicator_node.width = Val::Px(fov_width * scale);
    minimap_indicator_node.height = Val::Px(fov_height * scale);

    query_auxiliary_fov_indicators
        .iter_mut()
        .for_each(|mut node| {
            node.width = Val::Px(fov_width * scale);
            node.height = Val::Px(fov_height * scale);
        });
}
