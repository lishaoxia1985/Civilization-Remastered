//! 城市地块标签叠加插件（文明5风格横幅）
//!
//! 在屏幕空间绘制一条圆角矩形的长条横幅，其中心对齐到城市中心地块的中心，整体布局仿照文明5的城市信息横幅：
//!
//! ```text
//! [ 人口 ] [ ↗增长条┃回合数 ]   城市名称   [ ↖进度条┃回合数 ] [ 建造图标 ]
//! ```
//!
//! - 最左侧：当前城市人口数字
//! - 每个人口/建造进度区：一条竖向进度条 + 一个数字（显示还需多少回合增长人口 / 完成当前建造）
//! - 中间：城市名称
//! - 最右侧：当前正在建造项目（建筑/单位）的图标
//!
//! 标签位置每帧跟随相机（世界坐标 → 屏幕坐标），并与地块保持相对固定。
//! 当城市实体被移除时，对应的标签也会自动销毁。

use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use civ_map_generator::{grid::Grid, ruleset::Ruleset};

use crate::{
    ScreenState,
    assets::GameAssets,
    components::{City, CityProduction, CityYields, MainCamera},
    resources::{MapParametersRes, TileMapRes},
};

/// 城市标签根节点标记
#[derive(Component)]
pub struct CityOverlay;

/// 城市标签横幅的固定尺寸：用于将横幅中心对齐到城市中心地块的中心。
/// 尺寸缩小、两端做成完整半圆（圆角半径 = 高度的一半）。
/// （横幅为绝对定位节点，位置取 left/top 为左上角，故需偏移半个宽高才能居中对齐）
const OVERLAY_WIDTH: f32 = 252.0;
const OVERLAY_HEIGHT: f32 = 52.0;

// ===== 标签内各个子节点标记（携带所属城市实体，便于逐帧定位更新）=====
/// 城市人口数字文本
#[derive(Component, Clone, Copy)]
struct OverlayPopText(Entity);
/// 人口增长回合数字
#[derive(Component, Clone, Copy)]
struct OverlayGrowthTurn(Entity);
/// 人口增长竖向进度条的填充节点
#[derive(Component, Clone, Copy)]
struct OverlayGrowthFill(Entity);
/// 城市名称文本
#[derive(Component, Clone, Copy)]
struct OverlayNameText(Entity);
/// 建造完成回合数字
#[derive(Component, Clone, Copy)]
struct OverlayProdTurn(Entity);
/// 建造进度竖向进度条的填充节点
#[derive(Component, Clone, Copy)]
struct OverlayProdFill(Entity);
/// 当前建造项目的图标节点
#[derive(Component, Clone, Copy)]
struct OverlayProdIcon(Entity);

/// 单个城市的标签数据（由城市组件与产出计算得出，用于驱动 UI 更新）
struct OverlayData {
    name: String,
    population: u32,
    /// 人口增长回合数（"2"、"∞" 等）
    growth_turns: String,
    /// 人口增长进度条填充百分比 0..100
    growth_fill_pct: f32,
    /// 建造完成回合数
    prod_turns: String,
    /// 建造进度条填充百分比 0..100
    prod_fill_pct: f32,
    /// 当前建造项目图标（无项目时为 None）
    icon: Option<Handle<Image>>,
}
/// 城市标签叠加插件
pub struct CityOverlayPlugin;

impl Plugin for CityOverlayPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            update_city_overlays.run_if(in_state(ScreenState::WorldMap)),
        );
    }
}

/// 每帧更新所有城市的标签：为新城市创建标签，为已有城市更新位置与文本/进度，
/// 并销毁已不存在城市所对应的标签。
fn update_city_overlays(
    mut commands: Commands,
    city_query: Query<(Entity, &City, &CityYields), With<City>>,
    tile_map: Option<Res<TileMapRes>>,
    map_params: Res<MapParametersRes>,
    materials: Res<GameAssets>,
    camera: Single<(&Camera, &GlobalTransform), With<MainCamera>>,
    // 缓存 city_entity -> overlay_ui_entity 的对应关系，避免每帧重复创建/销毁
    mut cached: Local<HashMap<Entity, Entity>>,

    // 各子节点的更新查询
    mut overlay_params: ParamSet<(
        Query<&mut Node, With<CityOverlay>>,                  // p0
        Query<(&OverlayPopText, &mut Text)>,                  // p1
        Query<(&OverlayGrowthTurn, &mut Text)>,               // p2
        Query<(&OverlayGrowthFill, &mut Node)>,               // p3
        Query<(&OverlayNameText, &mut Text)>,                 // p4
        Query<(&OverlayProdTurn, &mut Text)>,                 // p5
        Query<(&OverlayProdFill, &mut Node)>,                 // p6
        Query<(&OverlayProdIcon, &mut Node, &mut ImageNode)>, // p7
    )>,
) {
    let Some(tile_map) = tile_map else {
        return;
    };
    let tile_map = &tile_map.0;
    let grid = tile_map.world_grid.grid;
    let ruleset = &map_params.0.ruleset;

    // 第一遍：为每个城市计算标签数据，并为新城市创建标签、为所有城市更新位置
    let mut data_map: HashMap<Entity, OverlayData> = HashMap::new();
    let mut alive_cities: HashSet<Entity> = HashSet::new();

    for (city_entity, city, yields) in city_query.iter() {
        alive_cities.insert(city_entity);

        // 城市中心地块（拥有地块列表的第一个即城市中心）
        let Some(&center_tile) = city.owned_tiles.first() else {
            continue;
        };

        // 地块中心的世界坐标
        let world = grid.offset_to_pixel(center_tile.to_offset(grid));
        let viewport = camera
            .0
            .world_to_viewport(&camera.1, Vec3::new(world[0], world[1], 0.0))
            .ok()
            .unwrap_or_default();

        // 横幅中心对齐到城市中心地块的中心（屏幕坐标：左上角为原点，y 向下）。
        // 节点为固定尺寸的绝对定位，left/top 指向左上角，故偏移半个宽高使中心落在地块中心。
        let left = viewport.x - OVERLAY_WIDTH * 0.5;
        let top = viewport.y - OVERLAY_HEIGHT * 0.5;

        let data = compute_overlay_data(city, yields, ruleset, &materials);
        data_map.insert(city_entity, data);

        // 若该城市还没有标签则创建，并记录根节点 entity
        let ui_entity = if let Some(&existing) = cached.get(&city_entity) {
            existing
        } else {
            let entity = spawn_city_overlay(
                &mut commands,
                left,
                top,
                &data_map[&city_entity],
                city_entity,
            );
            cached.insert(city_entity, entity);
            entity
        };

        // 更新已存在标签的位置（刚创建的在本帧命令尚未生效，下帧才会查到）
        if let Ok(mut node) = overlay_params.p0().get_mut(ui_entity) {
            node.left = Val::Px(left);
            node.top = Val::Px(top);
        }
    }

    // 第二遍：根据缓存数据逐项更新各子节点
    for (marker, mut text) in &mut overlay_params.p1() {
        if let Some(data) = data_map.get(&marker.0) {
            text.0 = data.population.to_string();
        }
    }
    for (marker, mut text) in &mut overlay_params.p2() {
        if let Some(data) = data_map.get(&marker.0) {
            text.0 = data.growth_turns.clone();
        }
    }
    for (marker, mut node) in &mut overlay_params.p3() {
        if let Some(data) = data_map.get(&marker.0) {
            node.height = Val::Percent(data.growth_fill_pct);
        }
    }
    for (marker, mut text) in &mut overlay_params.p4() {
        if let Some(data) = data_map.get(&marker.0) {
            text.0 = data.name.clone();
        }
    }
    for (marker, mut text) in &mut overlay_params.p5() {
        if let Some(data) = data_map.get(&marker.0) {
            text.0 = data.prod_turns.clone();
        }
    }
    for (marker, mut node) in &mut overlay_params.p6() {
        if let Some(data) = data_map.get(&marker.0) {
            node.height = Val::Percent(data.prod_fill_pct);
        }
    }
    for (marker, mut node, mut image) in &mut overlay_params.p7() {
        if let Some(data) = data_map.get(&marker.0) {
            match &data.icon {
                Some(handle) => {
                    node.display = Display::Flex;
                    image.image = handle.clone();
                }
                None => {
                    node.display = Display::None;
                }
            }
        }
    }

    // 第三遍：清理已消失城市对应的标签
    cached.retain(|city, overlay| {
        if alive_cities.contains(city) {
            true
        } else {
            commands.entity(*overlay).despawn();
            false
        }
    });
}

/// 根据城市组件与产出计算标签数据
fn compute_overlay_data(
    city: &City,
    yields: &CityYields,
    ruleset: &Ruleset,
    materials: &GameAssets,
) -> OverlayData {
    // ---- 人口增长 ----
    let growth_remaining = city.food_needed.saturating_sub(city.food);
    let growth_per = yields.food;
    let growth_turns = turns_string(growth_remaining, growth_per);
    let growth_fill_pct = ratio_percent(city.food, city.food_needed);

    // ---- 当前建造项目 ----
    let (_cost, name, prod_remaining, prod_per, fill_pct) = match &city.current_production {
        Some(CityProduction::Building(building)) => {
            let info = &ruleset.buildings[*building];
            let cost = info.cost.max(0) as u32;
            (
                cost,
                info.name.clone(),
                cost.saturating_sub(city.production_progress),
                yields.production,
                ratio_percent(city.production_progress, cost),
            )
        }
        Some(CityProduction::Unit(unit)) => {
            let info = &ruleset.units[*unit];
            let cost = info.cost.max(0) as u32;
            (
                cost,
                info.name.clone(),
                cost.saturating_sub(city.production_progress),
                yields.production,
                ratio_percent(city.production_progress, cost),
            )
        }
        None => (0, String::new(), 0, yields.production, 0.0),
    };

    let prod_turns = if city.current_production.is_none() {
        "-".to_string()
    } else {
        turns_string(prod_remaining, prod_per)
    };

    let icon = if city.current_production.is_some() && !name.is_empty() {
        Some(materials.texture_handle(&name))
    } else {
        None
    };

    OverlayData {
        name: city.name.clone(),
        population: city.population,
        growth_turns,
        growth_fill_pct,
        prod_turns,
        prod_fill_pct: fill_pct,
        icon,
    }
}

/// 将「剩余量 / 每回合产出」转换为要显示的回合数字符串；每回合产出为 0 时显示 ∞
fn turns_string(remaining: u32, per_turn: u32) -> String {
    if per_turn == 0 {
        "∞".to_string()
    } else {
        remaining.div_ceil(per_turn).to_string()
    }
}

/// 把「当前 / 上限」计算为 0..100 的百分比（避免除零与越界）
fn ratio_percent(current: u32, max: u32) -> f32 {
    if max == 0 {
        0.0
    } else {
        (current as f32 / max as f32 * 100.0).clamp(0.0, 100.0)
    }
}
/// 创建单个城市标签横幅 UI 节点
fn spawn_city_overlay(
    commands: &mut Commands,
    left: f32,
    top: f32,
    data: &OverlayData,
    city_entity: Entity,
) -> Entity {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(left),
                top: Val::Px(top),
                width: Val::Px(OVERLAY_WIDTH),
                height: Val::Px(OVERLAY_HEIGHT),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(10.0),
                padding: UiRect::axes(Val::Px(6.0), Val::Px(4.0)),
                border: UiRect::all(Val::Px(2.0)),
                // 两端做成完整半圆：圆角半径 = 高度的一半（左右端各为半圆）
                border_radius: BorderRadius {
                    top_left: Val::Px(OVERLAY_HEIGHT * 0.5),
                    top_right: Val::Px(OVERLAY_HEIGHT * 0.5),
                    bottom_left: Val::Px(OVERLAY_HEIGHT * 0.5),
                    bottom_right: Val::Px(OVERLAY_HEIGHT * 0.5),
                },
                ..default()
            },
            BackgroundColor(Color::srgba(0.03, 0.04, 0.08, 0.82)),
            BorderColor::all(Color::srgba(1.0, 0.84, 0.2, 0.9)),
            Pickable::IGNORE,
            CityOverlay,
        ))
        .with_children(|parent| {
            // 1) 左侧：城市人口数字
            parent
                .spawn((
                    Node {
                        width: Val::Px(34.0),
                        height: Val::Px(34.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        border: UiRect::all(Val::Px(2.0)),
                        border_radius: BorderRadius::all(Val::Px(4.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.1, 0.12, 0.2, 0.8)),
                    BorderColor::all(Color::srgba(0.9, 0.9, 0.9, 0.5)),
                ))
                .with_children(|box_builder| {
                    box_builder.spawn((
                        Text::new(data.population.to_string()),
                        TextFont {
                            font_size: FontSize::Px(17.0),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                        OverlayPopText(city_entity),
                    ));
                });

            // 2) 人口增长进度区：回合数字 + 竖向进度条
            spawn_vertical_measure(
                parent,
                &data.growth_turns,
                data.growth_fill_pct,
                Color::srgb(0.35, 0.95, 0.45),
                OverlayGrowthTurn(city_entity),
                OverlayGrowthFill(city_entity),
            );

            // 3) 中间：城市名称（占据剩余空间并居中）
            parent.spawn((
                Node {
                    flex_grow: 1.0,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                children![(
                    Text::new(data.name.clone()),
                    TextFont {
                        font_size: FontSize::Px(15.0),
                        ..default()
                    },
                    TextColor(Color::WHITE),
                    OverlayNameText(city_entity),
                )],
            ));

            // 4) 建造进度区：回合数字 + 竖向进度条
            spawn_vertical_measure(
                parent,
                &data.prod_turns,
                data.prod_fill_pct,
                Color::srgb(1.0, 0.85, 0.4),
                OverlayProdTurn(city_entity),
                OverlayProdFill(city_entity),
            );

            // 5) 最右侧：当前建造项目图标
            parent
                .spawn((
                    Node {
                        display: Display::Flex,
                        width: Val::Px(36.0),
                        height: Val::Px(36.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        border: UiRect::all(Val::Px(2.0)),
                        border_radius: BorderRadius::all(Val::Px(4.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.1, 0.12, 0.2, 0.9)),
                    BorderColor::all(Color::srgba(0.9, 0.9, 0.9, 0.5)),
                ))
                .with_children(|icon_builder| {
                    icon_builder.spawn((
                        Node {
                            width: Val::Px(30.0),
                            height: Val::Px(30.0),
                            ..default()
                        },
                        ImageNode::new(data.icon.clone().unwrap_or_else(|| Handle::default()))
                            .with_mode(bevy::ui::widget::NodeImageMode::Stretch),
                        OverlayProdIcon(city_entity),
                    ));
                });
        })
        .id()
}
/// 生成「回合数字 + 竖向进度条」这一列子节点
///
/// 数字显示在进度条上方，进度条为固定尺寸容器，填充节点从其底部（`JustifyContent::End`）向上增长。
#[allow(clippy::too_many_arguments)]
fn spawn_vertical_measure(
    parent: &mut ChildSpawnerCommands,
    turns: &str,
    fill_pct: f32,
    fill_color: Color,
    turn_marker: impl Component,
    fill_marker: impl Component,
) {
    parent.spawn((
        Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: Val::Px(2.0),
            ..default()
        },
        children![
            (
                Text::new(turns.to_string()),
                TextFont {
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
                TextColor(Color::WHITE),
                turn_marker,
            ),
            (
                Node {
                    width: Val::Px(13.0),
                    height: Val::Px(20.0),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::End,
                    border: UiRect::all(Val::Px(1.0)),
                    border_radius: BorderRadius::all(Val::Px(2.0)),
                    overflow: Overflow::clip(),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
                BorderColor::all(Color::srgba(1.0, 1.0, 1.0, 0.35)),
                children![(
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Percent(fill_pct),
                        ..default()
                    },
                    BackgroundColor(fill_color),
                    fill_marker,
                )],
            ),
        ],
    ));
}
