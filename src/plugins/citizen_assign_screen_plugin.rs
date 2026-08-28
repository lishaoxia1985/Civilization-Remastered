//! 市民分配屏幕插件
//!
//! 当玩家在城市操作菜单点击"分配市民"时，进入 [`ScreenState::CitizenAssign`] 状态。
//! 进入该状态时，在城市工作范围内的地块位置上创建市民工作图标（UI 节点）；
//! 点击图标可分配/取消分配市民；退出该状态时，所有创建的图标节点自动销毁。

use bevy::prelude::*;
use civ_map_generator::grid::Grid;

use crate::{
    NationComponent, Player, ScreenState, TurnManager,
    assets::GameAssets,
    components::{CitizenTile, City, MainCamera, Owner, SelectedCity},
    resources::TileMapRes,
};

/// 市民图标在屏幕上的尺寸（UI 节点逻辑像素）
const CITIZEN_ICON_SIZE: f32 = 40.0;

/// 市民分配屏幕插件
pub struct CitizenAssignScreenPlugin;

impl Plugin for CitizenAssignScreenPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(ScreenState::CitizenAssign), show_citizen_assign_icons)
            .add_systems(
                Update,
                handle_citizen_assign_click.run_if(in_state(ScreenState::CitizenAssign)),
            )
            .add_systems(
                Update,
                exit_citizen_assign_screen.run_if(in_state(ScreenState::CitizenAssign)),
            );
    }
}

/// 进入分配市民屏幕时，为城市工作范围内的地块显示市民图标（UI 节点）
///
/// 通过查询 owner 地块（`owned_tiles`）和 work 地块（`worked_tiles`）的位置，
/// 用相机把地块的世界坐标转换为屏幕坐标，再在该屏幕位置添加一个
/// [`CitizenWorkerIcon`] UI 节点（`ImageNode` + `TextureAtlas` 图集帧）。
/// 已工作与未工作的地块使用不同的图集帧区分。
/// 所有节点都带有 `DespawnOnExit(ScreenState::CitizenAssign)`，退出状态时自动销毁。
fn show_citizen_assign_icons(
    mut commands: Commands,
    city_query: Query<&City, With<SelectedCity>>,
    tile_map: Option<Res<TileMapRes>>,
    materials: Res<GameAssets>,
    camera: Single<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    mut layout_handle: Local<Option<Handle<TextureAtlasLayout>>>,
) {
    let Ok(city) = city_query.single() else {
        return;
    };
    let Some(tile_map) = tile_map else {
        return;
    };
    let grid = tile_map.0.world_grid.grid;

    // 图集布局只创建一次并复用
    let layout_handle = layout_handle
        .get_or_insert_with(|| {
            let layout = TextureAtlasLayout::from_grid(UVec2::splat(256), 4, 4, None, None);
            texture_atlas_layouts.add(layout)
        })
        .clone();
    let texture = materials.texture_handle("citizenicons1024");

    // 城市中心地块
    let center_tile = *city
        .owned_tiles
        .first()
        .expect("City must have a center tile");
    let work_radius = city.work_radius as i32;

    // 遍历城市拥有的非中心地块，只显示工作半径范围内
    for (idx, &tile) in city.owned_tiles.iter().enumerate() {
        if idx == 0 {
            continue; // 跳过城市中心
        }
        if grid.distance_to(center_tile.to_cell(), tile.to_cell()) > work_radius {
            continue;
        }

        let is_worked = city.worked_tiles.contains(&tile);
        let index = if is_worked { 0 } else { 9 };

        // world -> screen 坐标（以地块中心为基准，向上/左偏移半尺寸使图标居中）
        let world = grid.offset_to_pixel(tile.to_offset(grid));
        let viewport = camera
            .0
            .world_to_viewport(&camera.1, Vec3::from((world[0], world[1], 0.0)))
            .ok()
            .unwrap_or_default();
        let pos = viewport - Vec2::splat(CITIZEN_ICON_SIZE * 0.5);

        // 在该屏幕位置创建 UI 节点，退出状态时自动销毁
        commands.spawn((
            DespawnOnExit(ScreenState::CitizenAssign),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(pos.x),
                top: Val::Px(pos.y),
                width: Val::Px(CITIZEN_ICON_SIZE),
                height: Val::Px(CITIZEN_ICON_SIZE),
                ..default()
            },
            ImageNode {
                image: texture.clone(),
                texture_atlas: Some(TextureAtlas {
                    layout: layout_handle.clone(),
                    index,
                }),
                ..default()
            },
            CitizenTile(tile),
        ));
    }
}
/// 市民分配屏幕中，点击城市地块分配/取消市民工作
fn handle_citizen_assign_click(
    mut click_events: MessageReader<Pointer<Click>>,
    mut icon_query: Query<(&CitizenTile, &mut ImageNode)>,
    mut city_query: Query<(&mut City, &Owner), With<SelectedCity>>,
    player_query: Query<&NationComponent, With<Player>>,
    turn_manager: Res<TurnManager>,
    tile_map: Option<Res<TileMapRes>>,
) {
    // 需要地图数据计算距离
    let Some(tile_map_res) = tile_map else {
        return;
    };
    let tile_map = &tile_map_res.0;

    // 仅在玩家回合处理
    let current_entity = turn_manager.current_nation_entity();
    let Ok(nation_component) = player_query.get(current_entity) else {
        return;
    };
    let player_nation = nation_component.0;

    // 获取当前分配市民的城市（选中的城市）
    let Ok((mut city, owner)) = city_query.single_mut() else {
        return;
    };
    if owner.0 != player_nation {
        return;
    }

    for click in click_events.read() {
        // 点击的目标即为市民图标 UI 节点，读取其关联的地块
        let click_target = click.event_target();
        let Ok((citizen_tile, mut sprite)) = icon_query.get_mut(click_target) else {
            continue;
        };
        let clicked_tile = citizen_tile.0;

        // 检查该地块是否属于城市（非中心地块）
        let is_owned = city
            .owned_tiles
            .iter()
            .enumerate()
            .any(|(idx, &t)| idx > 0 && t == clicked_tile);
        if !is_owned {
            continue;
        }

        // 检查该地块是否在工作半径（3格）范围内
        let center_tile = *city
            .owned_tiles
            .first()
            .expect("City must have a center tile");
        let grid = tile_map.world_grid.grid;
        let work_radius = city.work_radius as i32;
        if grid.distance_to(center_tile.to_cell(), clicked_tile.to_cell()) > work_radius {
            continue;
        }

        // 分配/取消分配
        if let Some(worked_idx) = city.worked_tiles.iter().position(|&t| t == clicked_tile) {
            // 已工作 → 取消工作
            city.worked_tiles.remove(worked_idx);
            if let Some(atlas) = &mut sprite.texture_atlas {
                atlas.index = 9;
            }
            info!(
                "City {}: Citizen unassigned from tile {} ({} worked)",
                city.name,
                clicked_tile.index(),
                city.worked_tiles.len()
            );
        } else {
            // 检查是否还有空余市民（城市中心自动工作，不占用市民名额）
            let max_workers = city.population as usize;
            if city.worked_tiles.len() < max_workers {
                city.worked_tiles.push(clicked_tile);
                if let Some(atlas) = &mut sprite.texture_atlas {
                    atlas.index = 0;
                }
                info!(
                    "City {}: Citizen assigned to tile {} ({} worked)",
                    city.name,
                    clicked_tile.index(),
                    city.worked_tiles.len()
                );
            } else {
                info!(
                    "City {}: No available citizens to assign (max {})",
                    city.name, max_workers
                );
            }
        }
    }
}

/// 按 Esc 退出市民分配屏幕返回世界地图
fn exit_citizen_assign_screen(
    input: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<ScreenState>>,
) {
    if input.just_pressed(KeyCode::Escape) {
        next_state.set(ScreenState::WorldMap);
    }
}