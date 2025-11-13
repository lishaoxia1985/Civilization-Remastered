use std::{collections::HashMap, f32::consts::FRAC_PI_2};

use bevy::prelude::*;
use civ_map_generator::{
    grid::{
        Grid,
        hex_grid::{Hex, HexOrientation},
        offset_coordinate::OffsetCoordinate,
    },
    ruleset::Ruleset,
    tile::Tile,
    tile_component::{BaseTerrain, Feature, TerrainType},
    tile_map::RiverEdge,
};

use crate::{
    ColorReplaceMaterial, MainCamera, RulesetResource, TileMapResource,
    assets::MaterialResource,
    custom_mesh::{hex_mesh, line_mesh},
    unit_component::{Owner, Unit},
};

use enum_map::{EnumMap, enum_map};

#[derive(Component)]
pub struct WorldTile(pub Tile);

pub fn setup_tile_map(
    mut commands: Commands,
    map: Option<Res<TileMapResource>>,
    ruleset: Res<RulesetResource>,
    materials: Res<MaterialResource>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut color_materials: ResMut<Assets<ColorMaterial>>,
    mut custom_materials: ResMut<Assets<ColorReplaceMaterial>>,
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
    let feature_ice_sprite_rotation = match grid.layout.orientation {
        HexOrientation::Pointy => Quat::default(),
        HexOrientation::Flat => Quat::from_rotation_z(FRAC_PI_2 * 3.),
    };

    let hex_mesh = meshes.add(hex_mesh(&grid));

    for tile in tile_map.all_tiles() {
        // Spawn the tile with base terrain
        // this is the base tile entity that will be used to spawn the child entities
        let tile_entity = commands
            .spawn((
                Mesh2d(hex_mesh.clone()),
                MeshMaterial2d(base_terrain_and_material[tile.base_terrain(tile_map)].clone()),
                Visibility::Hidden,
                WorldTile(tile),
            ))
            .id();

        commands.entity(tile_entity).with_children(|parent| {
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
                            feature_ice_sprite_rotation
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

        let ruleset = &ruleset.0;
        let radius = tile_pixel_size.min_element() / 3.0;

        let inner_rectangle = meshes.add(Rectangle::new(radius / 2., radius / 2.));
        let outer_rectangle = meshes.add(Rectangle::new(radius, radius));

        // Place settler and warriors at the starting tile of the civilization
        if let Some(&civilization) = tile_map.starting_tile_and_civilization.get(&tile) {
            let replace_warrior_unit = ruleset.units.values().find(|&unit| {
                unit.unique_to == civilization.as_str() && unit.replaces == "Warrior"
            });
            let military_unit = if let Some(unit) = replace_warrior_unit {
                unit.name.clone()
            } else {
                "Warrior".to_string()
            };

            // Spawn the military unit
            commands.entity(tile_entity).with_children(|parent| {
                parent.spawn(unit_icon(
                    Unit::Military(military_unit),
                    Owner::Civilization(civilization),
                    ruleset,
                    inner_rectangle.clone(),
                    outer_rectangle.clone(),
                    &mut custom_materials,
                    &materials,
                    tile_pixel_size,
                ));

                parent.spawn(unit_icon(
                    Unit::Civilian("Settler".to_owned()),
                    Owner::Civilization(civilization),
                    ruleset,
                    inner_rectangle.clone(),
                    outer_rectangle.clone(),
                    &mut custom_materials,
                    &materials,
                    tile_pixel_size,
                ));
            });
        }

        // Place settler ast the starting tile of city state
        if let Some(&city_state) = tile_map.starting_tile_and_city_state.get(&tile) {
            commands.entity(tile_entity).with_children(|parent| {
                parent.spawn(unit_icon(
                    Unit::Civilian("Settler".to_owned()),
                    Owner::CityState(city_state),
                    ruleset,
                    inner_rectangle.clone(),
                    outer_rectangle.clone(),
                    &mut custom_materials,
                    &materials,
                    tile_pixel_size,
                ));
            });
        }
    }
}

/// Show the area of the main camera on the world map. The area without the main camera on the world map will be hidden to avoid visual confusion.
///
/// This function dynamically crops the world map display area to always match the main camera's viewport.
/// Non-visible areas are hidden to prevent visual confusion, with this mechanism supporting both wrap and non-wrap map projection modes.
pub fn show_main_camera_area(
    query: Single<&mut Transform, With<MainCamera>>,
    map: Option<Res<TileMapResource>>,
    mut query_world_tile: Query<
        (&mut Visibility, &mut Transform, &WorldTile),
        (With<WorldTile>, Without<MainCamera>),
    >,
) {
    if map.is_none() {
        return;
    };

    let tile_map = &map.unwrap().0;

    let grid = tile_map.world_grid.grid;

    // The width and height of the visible area in tiles.
    // Please make sure they are odd numbers. That will make sure the center of the camera is exactly on the center of the visible area.
    const WIDTH_OF_VISIBLE_AREA: i32 = 37;
    const HEIGHT_OF_VISIBLE_AREA: i32 = 21;

    // `width_of_camera` should < grid's width
    // Because if it's not, the same tile will be drawn twice due to the grid's wrapping behavior.
    assert!(WIDTH_OF_VISIBLE_AREA < grid.width() as i32);
    // `height_of_camera` should < grid's height
    // Because if it's not, the same tile will be drawn twice due to the grid's wrapping behavior.
    assert!(HEIGHT_OF_VISIBLE_AREA < grid.height() as i32);

    let camera_position = query.into_inner().translation.truncate().to_array();
    let camera_offset_coordinate = grid.pixel_to_offset(camera_position).to_array();
    let mut left_x = camera_offset_coordinate[0] - WIDTH_OF_VISIBLE_AREA / 2;
    let mut right_x = camera_offset_coordinate[0] + WIDTH_OF_VISIBLE_AREA / 2;
    // If the grid does not wrap on the x-axis, then we need to make sure that the left_x and right_x are within the bounds of the grid.
    if !grid.wrap_x() {
        left_x = left_x.max(0);
        right_x = right_x.min(grid.width() as i32 - 1);
    }
    let mut bottom_y = camera_offset_coordinate[1] - HEIGHT_OF_VISIBLE_AREA / 2;
    let mut top_y = camera_offset_coordinate[1] + HEIGHT_OF_VISIBLE_AREA / 2;
    // If the grid does not wrap on the y-axis, then we need to make sure that the bottom_y and top_y are within the bounds of the grid.
    if !grid.wrap_y() {
        bottom_y = bottom_y.max(0);
        top_y = top_y.min(grid.height() as i32 - 1);
    }

    let visible_tile_and_offset_list: HashMap<Tile, OffsetCoordinate> = (left_x..=right_x)
        .flat_map(|x| (bottom_y..=top_y).map(move |y| OffsetCoordinate::new(x, y)))
        .map(|offset_coordinate| {
            (
                Tile::from_offset(offset_coordinate, grid),
                offset_coordinate,
            )
        })
        .collect();

    for (mut visibility, mut transform, world_tile) in query_world_tile.iter_mut() {
        if let Some(&offset_coordinate) = visible_tile_and_offset_list.get(&world_tile.0) {
            let pixel_position = grid.offset_to_pixel(offset_coordinate);
            *visibility = Visibility::Visible;
            transform.translation = Vec3::from((pixel_position[0], pixel_position[1], 0.));
        } else {
            *visibility = Visibility::Hidden;
        }
    }
}

fn unit_icon(
    unit: Unit,
    owner: Owner,
    ruleset: &Ruleset,
    inner_rectangle: Handle<Mesh>,
    outer_rectangle: Handle<Mesh>,
    custom_materials: &mut ResMut<Assets<ColorReplaceMaterial>>,
    materials: &MaterialResource,
    tile_pixel_size: Vec2,
) -> impl Bundle {
    let (unit_name, transform_y, out_texture_name) = match &unit {
        Unit::Civilian(unit) => (unit.to_owned(), -tile_pixel_size.y / 4., "sv_unitcitizen"),
        Unit::Military(unit) => (unit.to_owned(), tile_pixel_size.y / 4., "sv_unitmilitary"),
    };

    let nation = match owner {
        Owner::Civilization(ref nation) | Owner::CityState(ref nation) => nation.as_str(),
    };

    let outer_color = ruleset.nations[nation].outer_color;
    let inner_color = ruleset.nations[nation].inner_color;

    (
        unit,
        owner,
        Mesh2d(inner_rectangle.clone()),
        MeshMaterial2d(custom_materials.add(ColorReplaceMaterial {
            inner_color: LinearRgba::from_u8_array_no_alpha(inner_color),
            outer_color: LinearRgba::from_u8_array_no_alpha(outer_color),
            texture: materials.texture_handle(&unit_name),
        })),
        Transform {
            translation: Vec3::new(0., transform_y, 6.),
            ..Default::default()
        },
        children![(
            Mesh2d(outer_rectangle.clone()),
            MeshMaterial2d(custom_materials.add(ColorReplaceMaterial {
                inner_color: LinearRgba::from_u8_array_no_alpha(inner_color,),
                outer_color: LinearRgba::from_u8_array_no_alpha(outer_color,),
                texture: materials.texture_handle(out_texture_name),
            },)),
            Transform::from_xyz(0., 0., -1.),
        )],
    )
}
