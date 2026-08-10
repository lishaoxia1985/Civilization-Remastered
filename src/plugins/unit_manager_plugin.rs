//! 单位管理插件
//!
//! TODO: 未来监听添加Unit的Message（通常在建造完成时触发），并生成对应的单位

use std::collections::BTreeSet;

use bevy::prelude::*;
use civ_map_generator::ruleset::{
    Ruleset,
    enums::{EnumStr, Nation, Unit},
};

use crate::{
    AppState, NationComponent, Player,
    assets::{ColorReplaceMaterial, GameAssets},
    components::{Experience, Health, Movement, Owner, Strength, UnitComponent},
    resources::{MapParametersRes, TileEntityMap, TileMapRes},
};

/// 单位所属文明组件
///
/// TODO： 此组件暂时无用，未来可能作为切换未行动unit的逻辑使用
#[derive(Component, Default)]
pub struct UnitManager(BTreeSet<Entity>);

/// 单位管理插件
pub struct UnitManagerPlugin;

impl Plugin for UnitManagerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(AppState::GameStart),
            spawn_starting_units_for_every_nations,
        );
    }
}

// ============ 单位生成系统 ============

/// 为所有文明添加起始单位
///
/// TODO: 暂时不支持CityState
fn spawn_starting_units_for_every_nations(
    mut commands: Commands,
    map_params: Res<MapParametersRes>,
    tile_map: Option<Res<TileMapRes>>,
    materials: Res<GameAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut custom_materials: ResMut<Assets<ColorReplaceMaterial>>,
    tile_entity_map: Res<TileEntityMap>,
    nation_query: Query<(Entity, Option<&Player>, &NationComponent), With<NationComponent>>,
) {
    let Some(tile_map) = tile_map else {
        return;
    };

    let tile_map = &tile_map.0;
    let grid = &tile_map.world_grid.grid;
    let ruleset = &map_params.0.ruleset;

    let tile_pixel_size = Vec2::from(grid.layout.size) * Vec2::new(2.0, 2.0);
    let radius = tile_pixel_size.min_element() / 3.0;

    let inner_rectangle = meshes.add(Rectangle::new(radius / 2., radius / 2.));
    let outer_rectangle = meshes.add(Rectangle::new(radius, radius));

    for (nation_entity, _, nation_component) in nation_query.iter() {
        let civilization = nation_component.0;
        let start_tile = tile_map
            .starting_tile_and_civilization
            .iter()
            .find(|&(_, &v)| v == civilization)
            .map(|(&k, _)| k)
            .expect("Can't find start tile of nation");

        let replace_warrior_unit = ruleset
            .units
            .values()
            .find(|&unit| unit.unique_to == civilization.as_str() && unit.replaces == "Warrior");
        let military_unit = if let Some(unit) = replace_warrior_unit {
            Unit::from_str(&unit.name)
        } else {
            Unit::Warrior
        };

        let tile_entity = tile_entity_map
            .get(start_tile)
            .expect("Tile entity not found");

        let warrior_entity = commands
            .spawn(unit_bundle(
                UnitComponent::Military(military_unit),
                civilization,
                ruleset,
                inner_rectangle.clone(),
                outer_rectangle.clone(),
                &mut custom_materials,
                &materials,
                tile_pixel_size,
            ))
            .id();

        let settler_entity = commands
            .spawn(unit_bundle(
                UnitComponent::Civilian(Unit::Settler),
                civilization,
                ruleset,
                inner_rectangle.clone(),
                outer_rectangle.clone(),
                &mut custom_materials,
                &materials,
                tile_pixel_size,
            ))
            .id();

        commands
            .entity(tile_entity)
            .add_children(&[warrior_entity, settler_entity]);

        commands
            .entity(nation_entity)
            .insert(UnitManager(BTreeSet::from([
                warrior_entity,
                settler_entity,
            ])));
    }

    // TODO: 当前未实现CityState相关逻辑
    //       未来或许和文明逻辑一致
    for (&tile, &city_state) in tile_map.starting_tile_and_city_state.iter() {
        let tile_entity = tile_entity_map.get(tile).expect("Tile entity not found");

        let settler_entity = commands
            .spawn(unit_bundle(
                UnitComponent::Civilian(Unit::Settler),
                city_state,
                ruleset,
                inner_rectangle.clone(),
                outer_rectangle.clone(),
                &mut custom_materials,
                &materials,
                tile_pixel_size,
            ))
            .id();

        commands.entity(tile_entity).add_children(&[settler_entity]);
    }
}

/// 创建单位组（包含战斗系统所需的所有组件）
fn unit_bundle(
    unit: UnitComponent,
    nation: Nation,
    ruleset: &Ruleset,
    inner_rectangle: Handle<Mesh>,
    outer_rectangle: Handle<Mesh>,
    custom_materials: &mut ResMut<Assets<ColorReplaceMaterial>>,
    materials: &GameAssets,
    tile_pixel_size: Vec2,
) -> impl Bundle {
    let (unit_name, transform_y, out_texture_name) = match &unit {
        UnitComponent::Civilian(unit) => (unit.as_str(), -tile_pixel_size.y / 4., "sv_unitcitizen"),
        UnitComponent::Military(unit) => (unit.as_str(), tile_pixel_size.y / 4., "sv_unitmilitary"),
    };

    let outer_color = ruleset.nations[nation].outer_color;
    let inner_color = ruleset.nations[nation].inner_color;

    // 从 ruleset 中获取单位属性
    let unit_key = *match &unit {
        UnitComponent::Military(u) => u,
        UnitComponent::Civilian(u) => u,
    };
    let unit_info = &ruleset.units[unit_key];

    let (strength, health, movement) = match &unit {
        UnitComponent::Military(_) => {
            let hp = 100u32;
            let mv = unit_info.movement.max(0) as u32;
            (
                Strength(unit_info.strength.max(0) as u32),
                Health {
                    current: hp,
                    max: hp,
                },
                Movement {
                    current: mv,
                    max: mv,
                },
            )
        }
        UnitComponent::Civilian(_) => {
            let hp = 50u32;
            let mv = unit_info.movement.max(0) as u32;
            (
                Strength(0),
                Health {
                    current: hp,
                    max: hp,
                },
                Movement {
                    current: mv,
                    max: mv,
                },
            )
        }
    };

    (
        unit,
        Owner(nation),
        strength,
        health,
        movement,
        Experience {
            current: 0,
            max: 100,
        },
        Mesh2d(inner_rectangle.clone()),
        MeshMaterial2d(custom_materials.add(ColorReplaceMaterial {
            inner_color: bevy::color::LinearRgba::from_u8_array_no_alpha(inner_color),
            outer_color: bevy::color::LinearRgba::from_u8_array_no_alpha(outer_color),
            texture: materials.texture_handle(&unit_name),
        })),
        Transform {
            translation: Vec3::new(0., transform_y, 6.),
            ..Default::default()
        },
        children![(
            Mesh2d(outer_rectangle.clone()),
            MeshMaterial2d(custom_materials.add(ColorReplaceMaterial {
                inner_color: bevy::color::LinearRgba::from_u8_array_no_alpha(inner_color,),
                outer_color: bevy::color::LinearRgba::from_u8_array_no_alpha(outer_color,),
                texture: materials.texture_handle(out_texture_name),
            },)),
            Transform::from_xyz(0., 0., -1.),
        )],
    )
}
