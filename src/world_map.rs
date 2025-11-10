use std::{
    collections::{HashMap, HashSet},
    f32::consts::FRAC_PI_2,
};

use bevy::prelude::*;
use civ_map_generator::{
    grid::{
        Grid,
        hex_grid::{Hex, HexOrientation},
        offset_coordinate::OffsetCoordinate,
    },
    tile::Tile,
    tile_component::{BaseTerrain, Feature, TerrainType},
    tile_map::RiverEdge,
};

use crate::{
    ColorReplaceMaterial, MainCamera, RulesetResource, TileMapResource,
    assets::MaterialResource,
    custom_mesh::{hex_mesh, line_mesh},
};

use enum_map::{EnumMap, enum_map};

#[allow(dead_code)]
#[derive(Component)]
struct MapTile(Tile);

pub fn setup_tile_map(
    mut commands: Commands,
    query: Single<&mut Transform, With<MainCamera>>,
    map: Option<Res<TileMapResource>>,
    ruleset: Res<RulesetResource>,
    materials: Res<MaterialResource>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut color_materials: ResMut<Assets<ColorMaterial>>,
    mut custom_materials: ResMut<Assets<ColorReplaceMaterial>>,
    mut exist_entity_and_offset_coordinates: Local<Vec<(Entity, OffsetCoordinate)>>,
) {
    if map.is_none() {
        return;
    };

    let tile_map = &map.unwrap().0;

    let grid = tile_map.world_grid.grid;

    let base_terrain_and_material: EnumMap<BaseTerrain, Handle<ColorMaterial>> = enum_map! {
        base_terrain => color_materials.add(materials.texture_handle(base_terrain.as_str())),
    };

    let mut tile_and_river_flow_direction = HashMap::new();

    tile_map.river_list.iter().flatten().for_each(|river_edge| {
        tile_and_river_flow_direction
            .entry(river_edge.tile)
            .or_insert_with(Vec::new)
            .push(river_edge.flow_direction);
    });

    let all_possible_river_edge_mesh: Vec<_> = grid
        .corner_direction_array()
        .iter()
        .map(|&flow_direction| {
            let river_edge = RiverEdge {
                tile: Tile::new(0),
                flow_direction,
            };

            let [start_corner_direction, end_corner_direction] =
                river_edge.start_and_end_corner_directions(grid);
            let start_corner_position = grid.layout.corner(Hex::new(0, 0), start_corner_direction);
            let end_corner_position = grid.layout.corner(Hex::new(0, 0), end_corner_direction);

            let start = [start_corner_position[0], start_corner_position[1], 0.0];
            let end = [end_corner_position[0], end_corner_position[1], 0.0];
            let line_mesh = line_mesh(start.into(), end.into(), 1.5);
            (flow_direction, line_mesh)
        })
        .collect();

    let tile_pixel_size = Vec2::from(grid.layout.size) * Vec2::new(2.0, 2.0);

    // We only need to rotate the sprite for `Feature::Ice` because it was originally designed exclusively for Pointy-oriented hexagons.
    // Other terrain sprites were created to work seamlessly with both Pointy and Flat hexagon orientations.
    let sprite_rotation = match grid.layout.orientation {
        HexOrientation::Pointy => Quat::default(),
        HexOrientation::Flat => Quat::from_rotation_z(FRAC_PI_2 * 3.),
    };

    // (1 + offset_x * 2) should < grid's width
    // Because if it's not, the same tile will be drawn twice due to the grid's wrapping behavior.
    const OFFSET_X: i32 = 18;
    assert!(1 + OFFSET_X * 2 < grid.width() as i32);
    // (1 + offset_y * 2) should < grid's height
    // Because if it's not, the same tile will be drawn twice due to the grid's wrapping behavior.
    const OFFSET_Y: i32 = 10;
    assert!(1 + OFFSET_Y * 2 < grid.height() as i32);

    let camera_position = query.into_inner().translation.truncate().to_array();
    let camera_offset_coordinate = grid.pixel_to_offset(camera_position).to_array();
    let mut left_x = camera_offset_coordinate[0] - OFFSET_X;
    let mut right_x = camera_offset_coordinate[0] + OFFSET_X;
    if !grid.wrap_x() {
        left_x = left_x.max(0);
        right_x = right_x.min(grid.width() as i32 - 1);
    }
    let mut bottom_y = camera_offset_coordinate[1] - OFFSET_Y;
    let mut top_y = camera_offset_coordinate[1] + OFFSET_Y;
    if !grid.wrap_y() {
        bottom_y = bottom_y.max(0);
        top_y = top_y.min(grid.height() as i32 - 1);
    }

    let mut offset_list = (left_x..=right_x)
        .flat_map(move |x| (bottom_y..=top_y).map(move |y| OffsetCoordinate::new(x, y)))
        .collect::<HashSet<_>>();

    // Despawn the tiles that are out of the current viewport
    // And remove the tiles that are still in the current viewport from offset_list,
    // we only need to spawn the tiles that can't be found in the current viewport later.
    exist_entity_and_offset_coordinates.retain(|(entity, map_offset)| {
        if !offset_list.contains(map_offset) {
            commands.entity(*entity).despawn();
            false
        } else {
            offset_list.remove(map_offset);
            true
        }
    });

    let hex_mesh = meshes.add(hex_mesh(&grid));

    for &offset_coordinate in offset_list.iter() {
        let pixel_position = grid.offset_to_pixel(offset_coordinate);
        let tile = Tile::from_offset(offset_coordinate, grid);
        // Spawn the tile with base terrain
        let parent = commands
            .spawn((
                Mesh2d(hex_mesh.clone()),
                Transform {
                    translation: Vec3::from((pixel_position[0], pixel_position[1], 0.)),
                    ..Default::default()
                },
                MeshMaterial2d(base_terrain_and_material[tile.base_terrain(tile_map)].clone()),
            ))
            .insert(MapTile(tile))
            .id();

        exist_entity_and_offset_coordinates.push((parent, offset_coordinate));

        commands.entity(parent).with_children(|parent| {
            // Draw river edges
            if let Some(flow_direction_list) = tile_and_river_flow_direction.get(&tile) {
                flow_direction_list.iter().for_each(|direction| {
                    let (_, line_mesh) = all_possible_river_edge_mesh
                        .iter()
                        .find(|(d, _)| d == direction)
                        .unwrap();
                    parent.spawn((
                        Mesh2d(meshes.add(line_mesh.clone())),
                        MeshMaterial2d(
                            color_materials
                                .add(ColorMaterial::from_color(Color::srgb_u8(140, 215, 215))),
                        ),
                        Transform {
                            translation: Vec3::new(0., 0., 5.),
                            ..Default::default()
                        },
                    ));
                })
            };

            // Draw terrain type Mountain with no natural wonder and Hill
            // Notice terrain type Flatland and Water are not drawn in this moment because they only need to be drawn with base terrain
            let terrain_type = tile.terrain_type(tile_map);
            let is_mountain_without_wonder =
                terrain_type == TerrainType::Mountain && tile.natural_wonder(tile_map).is_none();

            if is_mountain_without_wonder || terrain_type == TerrainType::Hill {
                parent.spawn((
                    Sprite {
                        custom_size: Some(tile_pixel_size),
                        image: materials.texture_handle(terrain_type.as_str()),
                        ..Default::default()
                    },
                    Transform {
                        translation: Vec3::new(0., 0., 3.),
                        ..Default::default()
                    },
                ));
            }

            // Draw the feature
            if let Some(feature) = tile.feature(tile_map) {
                parent.spawn((
                    Sprite {
                        custom_size: Some(tile_pixel_size),
                        image: materials.texture_handle(feature.as_str()),
                        ..Default::default()
                    },
                    Transform {
                        translation: Vec3::new(0., 0., 2.),
                        rotation: if feature == Feature::Ice {
                            sprite_rotation
                        } else {
                            Quat::default()
                        },
                        ..Default::default()
                    },
                ));
            }

            // Draw the natural wonder
            if let Some(natural_wonder) = tile.natural_wonder(tile_map) {
                parent.spawn((
                    Sprite {
                        custom_size: Some(tile_pixel_size),
                        image: materials.texture_handle(natural_wonder.as_str()),
                        ..Default::default()
                    },
                    Transform {
                        translation: Vec3::new(0., 0., 2.),
                        ..Default::default()
                    },
                ));
            }
        });

        // Place settler and warriors at the starting tile of the civilization
        let ruleset = &ruleset.0;
        let radius = tile_pixel_size.min_element() / 3.0;

        let inner_rectangle = meshes.add(Rectangle::new(radius / 2., radius / 2.));
        let outer_rectangle = meshes.add(Rectangle::new(radius, radius));

        tile_map.starting_tile_and_civilization.iter().for_each(
            |(&starting_tile, civilization)| {
                let outer_color = ruleset.nations[civilization.as_str()].outer_color;
                let inner_color = ruleset.nations[civilization.as_str()].inner_color;
                if starting_tile == tile {
                    commands.entity(parent).with_children(|parent| {
                        // Place settler
                        parent
                            .spawn((
                                Mesh2d(inner_rectangle.clone()),
                                MeshMaterial2d(custom_materials.add(ColorReplaceMaterial {
                                    inner_color: LinearRgba::from_u8_array_no_alpha(inner_color),
                                    outer_color: LinearRgba::from_u8_array_no_alpha(outer_color),
                                    texture: materials.texture_handle("Settler"),
                                })),
                                Transform {
                                    translation: Vec3::new(0., -tile_pixel_size.y / 4., 6.),
                                    ..Default::default()
                                },
                            ))
                            .with_children(|parent| {
                                parent.spawn((
                                    Mesh2d(outer_rectangle.clone()),
                                    MeshMaterial2d(custom_materials.add(ColorReplaceMaterial {
                                        inner_color: LinearRgba::from_u8_array_no_alpha(
                                            inner_color,
                                        ),
                                        outer_color: LinearRgba::from_u8_array_no_alpha(
                                            outer_color,
                                        ),
                                        texture: materials.texture_handle("sv_unitcitizen"),
                                    })),
                                    Transform::from_xyz(0., 0., -1.),
                                ));
                            });

                        // Place warrior
                        parent
                            .spawn((
                                Mesh2d(inner_rectangle.clone()),
                                MeshMaterial2d(custom_materials.add(ColorReplaceMaterial {
                                    inner_color: LinearRgba::from_u8_array_no_alpha(inner_color),
                                    outer_color: LinearRgba::from_u8_array_no_alpha(outer_color),
                                    texture: materials.texture_handle("Warrior"),
                                })),
                                Transform {
                                    translation: Vec3::new(0., tile_pixel_size.y / 4., 6.),
                                    ..Default::default()
                                },
                            ))
                            .with_children(|parent| {
                                parent.spawn((
                                    Mesh2d(outer_rectangle.clone()),
                                    MeshMaterial2d(custom_materials.add(ColorReplaceMaterial {
                                        inner_color: LinearRgba::from_u8_array_no_alpha(
                                            inner_color,
                                        ),
                                        outer_color: LinearRgba::from_u8_array_no_alpha(
                                            outer_color,
                                        ),
                                        texture: materials.texture_handle("sv_unitmilitary"),
                                    })),
                                    Transform::from_xyz(0., 0., -1.),
                                ));
                            });
                    });
                }
            },
        );

        // Place settlers on starting tiles of city states
        tile_map
            .starting_tile_and_city_state
            .iter()
            .for_each(|(&starting_tile, city_state)| {
                let outer_color = ruleset.nations[city_state.as_str()].outer_color;
                let inner_color = ruleset.nations[city_state.as_str()].inner_color;
                if starting_tile == tile {
                    commands.entity(parent).with_children(|parent| {
                        parent
                            .spawn((
                                Mesh2d(inner_rectangle.clone()),
                                MeshMaterial2d(custom_materials.add(ColorReplaceMaterial {
                                    inner_color: LinearRgba::from_u8_array_no_alpha(inner_color),
                                    outer_color: LinearRgba::from_u8_array_no_alpha(outer_color),
                                    texture: materials.texture_handle("Settler"),
                                })),
                                Transform {
                                    translation: Vec3::new(0., -tile_pixel_size.y / 4., 6.),
                                    ..Default::default()
                                },
                            ))
                            .with_children(|parent| {
                                parent.spawn((
                                    Mesh2d(outer_rectangle.clone()),
                                    MeshMaterial2d(custom_materials.add(ColorReplaceMaterial {
                                        inner_color: LinearRgba::from_u8_array_no_alpha(
                                            inner_color,
                                        ),
                                        outer_color: LinearRgba::from_u8_array_no_alpha(
                                            outer_color,
                                        ),
                                        texture: materials.texture_handle("sv_unitcitizen"),
                                    })),
                                    Transform::from_xyz(0., 0., -1.),
                                ));
                            });
                    });
                }
            });
    }
}
