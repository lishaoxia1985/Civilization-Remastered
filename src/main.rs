mod assets;
mod ruleset;
mod tile_map;

use assets::{AssetsPlugin, MaterialResource};
use bevy_prototype_lyon::prelude::*;
use ruleset::Ruleset;
use tile_map::{
    hex::{Hex, HexOrientation, Offset, SQRT_3},
    HexLayout, MapParameters, MapSize, TileMap,
};

use bevy::{math::DVec2, prelude::*, window::close_on_esc};

use crate::ruleset::Unique;

fn main() {
    let ruleset = Ruleset::new();
    dbg!(&ruleset.terrains);
    let mut unique_list = Vec::new();
    for terrains in ruleset.terrains.values() {
        for unique in terrains.uniques.iter() {
            if !unique_list.contains(unique) {
                unique_list.push(unique.to_owned())
            }
        }
    }
    dbg!(unique_list);
    let unique_objects =
        Unique::new("[-33]% Strength <for [All] units> <when below [-10] Happiness>");
    dbg!(unique_objects);
    App::new()
        .insert_resource(Msaa::Sample4)
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Civilization-Remastered".to_owned(),
                resolution: (800., 600.).into(),
                window_level: bevy::window::WindowLevel::AlwaysOnTop,
                ..default()
            }),
            ..default()
        }))
        .insert_resource(Ruleset::new())
        .add_plugins(ShapePlugin)
        .add_plugins(AssetsPlugin)
        .add_systems(Startup, (camera_setup, start_up_system))
        .add_systems(Update, close_on_esc)
        .run();
}

fn camera_setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

fn start_up_system(
    mut commands: Commands,
    materials: Res<MaterialResource>,
    ruleset: Res<Ruleset>,
) {
    let mut tile_map = TileMap::new(
        MapParameters {
            map_size: MapSize {
                width: 80,
                height: 40,
            },
            hex_layout: HexLayout {
                orientation: HexOrientation::Pointy,
                size: DVec2::new(8., 8.),
                origin: DVec2::new(0., 0.),
            },
            offset: Offset::Odd,
            ..Default::default()
        },
        &ruleset,
    );
    let tile_pixel_size = tile_map.map_parameters.hex_layout.size * DVec2::new(2.0, 3_f64.sqrt());
    tile_map.spawn_tile_type_for_pangaea(&ruleset);
    //tile_map.spawn_tile_type_for_fractal(&ruleset);
    tile_map.generate_terrain(&ruleset);
    tile_map.generate_coasts(&ruleset);
    tile_map.generate_lakes(&ruleset);
    tile_map.recalculate_areas();
    tile_map.add_rivers(&ruleset);
    tile_map.add_lakes(&ruleset);
    tile_map.add_features(&ruleset);
    tile_map.natural_wonder_generator(&ruleset);
    tile_map.recalculate_areas();

    let position_offset = tile_map.tile_list.values().fold(
        (0.0_f64, 0.0_f64, 0.0_f64, 0.0_f64),
        |(min_offset_x, min_offset_y, max_offset_x, max_offset_y), tile| {
            let [offset_x, offset_y] = tile_map
                .map_parameters
                .hex_layout
                .hex_to_pixel(Hex::from(tile.hex_position))
                .to_array();
            (
                min_offset_x.min(offset_x),
                min_offset_y.min(offset_y),
                max_offset_x.max(offset_x),
                max_offset_y.max(offset_y),
            )
        },
    );

    tile_map.map_parameters.hex_layout.origin = -(DVec2::new(position_offset.0, position_offset.1)
        + DVec2::new(position_offset.2, position_offset.3))
        / 2.;

    /* let height = tile_map.map_parameters.map_size.height;
    let width = tile_map.map_parameters.map_size.width;

    let (height_pixel, width_pixel) = match tile_map.map_parameters.hex_layout.orientation {
        HexOrientation::Pointy => (
            (2. + (width as f64 - 1.) * 1.5) * tile_map.map_parameters.hex_layout.size.x,
            (height as f64 + 0.5) * SQRT_3 * tile_map.map_parameters.hex_layout.size.y,
        ),
        HexOrientation::Flat => (
            (height as f64 + 0.5) * SQRT_3 * tile_map.map_parameters.hex_layout.size.x,
            (2. + (width as f64 - 1.) * 1.5) * tile_map.map_parameters.hex_layout.size.y,
        ),
    }; */

    tile_map.river_list.values().for_each(|river| {
        let mut path_builder = PathBuilder::new();
        river.iter().for_each(|(hex_position, direction)| {
            let tile = &tile_map.tile_list[hex_position];
            let index = tile_map
                .tile_edge_direction()
                .iter()
                .position(|x| x == direction)
                .unwrap();
            let (first_point, second_point) = match tile_map.map_parameters.hex_layout.orientation {
                HexOrientation::Pointy => (
                    tile_map.tile_corner_direction()[index],
                    tile_map.tile_corner_direction()[(index + 1) % 6],
                ),
                HexOrientation::Flat => (
                    tile_map.tile_corner_direction()[(index + 5) % 6],
                    tile_map.tile_corner_direction()[index],
                ),
            };
            let first_point_position = tile.tile_corner_position(first_point, &tile_map);
            let second_point_position = tile.tile_corner_position(second_point, &tile_map);

            path_builder.move_to(first_point_position.as_vec2());
            path_builder.line_to(second_point_position.as_vec2());
        });

        let path = path_builder.build();

        commands.spawn((
            ShapeBundle {
                path: GeometryBuilder::build_as(&path),
                transform: Transform::from_translation(Vec3::new(0., 0., 10.)),
                ..default()
            },
            Stroke::new(Color::BLACK, 2.0),
            Fill::color(Color::RED),
        ));
    });

    let (sprite_rotation, text_rotation) = match tile_map.map_parameters.hex_layout.orientation {
        HexOrientation::Pointy => (
            Quat::from_rotation_z(std::f32::consts::FRAC_PI_2 * 3.),
            Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2 * 3.),
        ),
        HexOrientation::Flat => (Quat::default(), Quat::default()),
    };

    for tile in tile_map.tile_list.values() {
        let pixel_position = tile.pixel_position(tile_map.map_parameters.hex_layout);
        commands
            .spawn(SpriteBundle {
                sprite: Sprite {
                    custom_size: Some(tile_pixel_size.as_vec2()),
                    ..Default::default()
                },
                texture: materials.texture_handle(&tile.base_terrain.name),
                transform: Transform {
                    translation: Vec3::from((pixel_position.as_vec2(), 1.)),
                    rotation: sprite_rotation,
                    ..Default::default()
                },
                ..Default::default()
            })
            .with_children(|parent| {
                tile.terrain_features.iter().for_each(|terrain_feature| {
                    let terrain_feature = match terrain_feature.name.as_str() {
                        "Forest" => "ForestG",
                        "Jungle" => "JungleG",
                        _ => &terrain_feature.name,
                    };
                    parent.spawn(SpriteBundle {
                        sprite: Sprite {
                            custom_size: Some(tile_pixel_size.as_vec2()),
                            ..Default::default()
                        },
                        texture: materials.texture_handle(terrain_feature),
                        transform: Transform::from_translation(Vec3::new(0., 0., 1.)),
                        ..Default::default()
                    });
                })
            });
    }
}
