use bevy::{
    color::{
        Color,
        palettes::css::{BLACK, RED, WHITE},
    },
    math::Vec2,
    picking::{
        events::{Click, Drag, Pointer},
        pointer::PointerButton,
    },
    prelude::*,
    ui::{
        BackgroundColor, BorderColor, Node, Overflow, PositionType, ScrollPosition, UiRect, Val,
        percent, widget::Text,
    },
};
use civ_map_generator::ruleset::Ruleset;

use crate::RulesetResource;
use crate::assets::MaterialResource;

pub fn setup_tech_button(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(10.0),
                top: Val::Px(10.0),
                width: Val::Auto,
                height: Val::Auto,
                border: UiRect::all(Val::Px(2.0)),
                ..Default::default()
            },
            BackgroundColor(Color::BLACK),
            BorderColor::all(Color::WHITE),
            Text("Open Tech Tree".to_string()),
        ))
        .observe(open_tech_tree);
}

#[derive(Component)]
struct ScrollableNode;

fn open_tech_tree(
    drag: On<Pointer<Click>>,
    mut commands: Commands,
    ruleset: Res<RulesetResource>,
    materials: Res<MaterialResource>,
) {
    let ruleset = &ruleset.0;
    let column_count = ruleset
        .technologies
        .values()
        .map(|technology| technology.column)
        .max()
        .unwrap() as i16
        + 1;

    let row_count = ruleset
        .technologies
        .values()
        .map(|technology| technology.row)
        .max()
        .unwrap() as i16
        + 1;

    if matches!(drag.button, PointerButton::Primary) {
        commands
            .spawn((
                Node {
                    width: percent(100),
                    height: percent(100),
                    overflow: Overflow::scroll_x(),
                    ..Default::default()
                },
                ScrollPosition(Vec2::ZERO),
                ScrollableNode,
                BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
            ))
            .observe(
                |drag: On<Pointer<Drag>>,
                 mut scroll_position_query: Query<
                    (&mut ScrollPosition, &Node, &ComputedNode),
                    With<ScrollableNode>,
                >| {
                    // We will edit the query in the future
                    // `node` is unnessarily because we have known `node.overflow` before
                    if let Ok((mut scroll_position, node, computed)) =
                        scroll_position_query.single_mut()
                    {
                        let max_offset = (computed.content_size() - computed.size())
                            * computed.inverse_scale_factor();
                        let delta = drag.delta;
                        if node.overflow.x == OverflowAxis::Scroll && delta.x != 0. {
                            // Is this node already scrolled all the way in the direction of the scroll?
                            let max = if delta.x > 0. {
                                scroll_position.x >= max_offset.x
                            } else {
                                scroll_position.x <= 0.
                            };

                            if !max {
                                scroll_position.x += delta.x;
                            }
                        }

                        // It's unnecessary to check because `node.overflow.y == OverflowAxis::Scroll` is always false in this example.
                        /* if node.overflow.y == OverflowAxis::Scroll && delta.y != 0. {
                            // Is this node already scrolled all the way in the direction of the scroll?
                            let max = if delta.y > 0. {
                                scroll_position.y >= max_offset.y
                            } else {
                                scroll_position.y <= 0.
                            };

                            if !max {
                                scroll_position.y += delta.y;
                            }
                        } */
                    }
                },
            )
            .with_children(|builder| {
                builder
                    .spawn(Node {
                        display: Display::Grid,
                        grid_template_rows: RepeatedGridTrack::fr(row_count as u16, 1.),
                        grid_template_columns: RepeatedGridTrack::px(column_count as i32, 400.),
                        ..default()
                    })
                    .with_children(|builder| {
                        ruleset.technologies.values().for_each(|technology| {
                            builder.spawn((
                                Node {
                                    grid_row: GridPlacement::start(
                                        technology.row as i16, // Notice: In json file, row starts from 1, maybe 0 in the future
                                    ),
                                    grid_column: GridPlacement::start(technology.column as i16 + 1), // Notice: In json file, column starts from 0
                                    border: UiRect::all(Val::Px(2.0)),
                                    ..default()
                                },
                                Pickable {
                                    should_block_lower: false,
                                    is_hoverable: true,
                                },
                                children![technology_button(
                                    technology.name.clone(),
                                    &materials,
                                    ruleset
                                )],
                            ));
                        });
                    });
            });
    }
}

fn technology_button(
    technology_name: String,
    materials: &MaterialResource,
    ruleset: &Ruleset,
) -> impl Bundle {
    (
        Node {
            width: px(300),
            height: px(60),
            border: UiRect::all(Val::Px(2.0)),
            ..default()
        },
        BackgroundColor(Color::NONE),
        BorderColor::all(Color::WHITE),
        BorderRadius::all(Val::Px(10.0)),
        children![(
            Node {
                display: Display::Grid,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                grid_template_columns: vec![
                    GridTrack::percent(20.),
                    GridTrack::fr(1.0),
                    GridTrack::px(80.0)
                ],
                grid_template_rows: vec![GridTrack::percent(25.), GridTrack::percent(75.0)],
                ..default()
            },
            children![
                (
                    Node {
                        grid_column: GridPlacement::start(1),
                        grid_row: GridPlacement::start(1).set_span(2),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    children![(
                        Node {
                            width: px(40),
                            height: px(40),
                            border: UiRect::all(Val::Px(10.0)),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                        ImageNode::new(materials.texture_handle(&technology_name))
                            .with_color(RED.into()),
                        BorderRadius::all(px(f32::MAX)),
                        Outline {
                            width: px(2),
                            offset: px(3),
                            color: Color::WHITE,
                        },
                    ),],
                ),
                (
                    Node {
                        grid_column: GridPlacement::start(2),
                        grid_row: GridPlacement::start(1),
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    Text(technology_name.clone()),
                    TextFont {
                        font_size: 12.,
                        ..default()
                    },
                ),
                (
                    Node {
                        grid_column: GridPlacement::start(3),
                        grid_row: GridPlacement::start(1),
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    Text("5000 turns".to_string()),
                    TextFont {
                        font_size: 12.,
                        ..default()
                    },
                ),
                (
                    Node {
                        grid_column: GridPlacement::start(2).set_span(2),
                        grid_row: GridPlacement::start(2),
                        border: UiRect::all(Val::Px(1.0)),
                        margin: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BackgroundColor(Color::NONE),
                    BorderColor::all(Color::WHITE),
                    BorderRadius::all(Val::Px(10.0)),
                    children![tech_unlock_item_list(technology_name, ruleset, materials)],
                )
            ]
        )],
    )
}

/// This function creates a list of tech unlock items for a given technology.
///
/// TODO: In original game, every civ has some unique buildings, units, improvements, etc.
/// And they will replace the default ones when unlocked. This is not implemented yet.
fn tech_unlock_item_list(
    technology_name: String,
    ruleset: &Ruleset,
    materials: &MaterialResource,
) -> impl Bundle {
    let units = &ruleset.units;
    let unlock_units = units
        .values()
        .filter(|unit| unit.required_tech == technology_name && unit.unique_to.is_empty());

    let buildings = &ruleset.buildings;
    let unlock_buildings: Vec<_> = buildings
        .values()
        .filter(|building| {
            building.required_tech == technology_name && building.unique_to.is_empty()
        })
        .map(|building| building.name.clone())
        .collect();

    let tile_improvements = &ruleset.tile_improvements;
    let unlock_tile_improvements = tile_improvements.values().filter(|tile_improvement| {
        tile_improvement.required_tech == technology_name && tile_improvement.unique_to.is_empty()
    });

    let unlock_uniques = ruleset.technologies[&technology_name].uniques.clone();

    let unit_materials: Vec<_> = unlock_units
        .map(|unit| materials.texture_handle(&unit.name))
        .collect();

    let building_materials: Vec<_> = unlock_buildings
        .iter()
        .map(|building_name| materials.texture_handle(building_name))
        .collect();

    let tile_improvement_materials: Vec<_> = unlock_tile_improvements
        .map(|tile_improvement| materials.texture_handle(&tile_improvement.name))
        .collect();

    let unique_material = materials.texture_handle("Fallback");

    (
        Node {
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            display: Display::Grid,
            grid_template_columns: RepeatedGridTrack::fr(5, 1.),
            ..default()
        },
        Children::spawn((
            SpawnIter(
                unit_materials
                    .into_iter()
                    .chain(building_materials)
                    .chain(tile_improvement_materials)
                    .map(|building_name| {
                        (
                            Node {
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                ..default()
                            },
                            children![unit_or_building_or_tile_improvement_item(building_name)],
                        )
                    }),
            ),
            SpawnIter(unlock_uniques.into_iter().map(move |_| {
                (
                    Node {
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    children![unique_item(unique_material.clone())],
                )
            })),
        )),
    )
}

fn unit_or_building_or_tile_improvement_item(building_texture: Handle<Image>) -> impl Bundle {
    (
        Node {
            width: px(25),
            height: px(25),
            border: UiRect::all(Val::Px(10.0)),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        ImageNode::new(building_texture).with_color(BLACK.into()),
        BackgroundColor(WHITE.into()),
        BorderRadius::all(px(f32::MAX)),
        Outline {
            width: px(1),
            offset: px(3),
            color: Color::WHITE,
        },
    )
}

fn unique_item(texture: Handle<Image>) -> impl Bundle {
    (
        Node {
            width: px(25),
            height: px(25),
            border: UiRect::all(Val::Px(10.0)),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        ImageNode::new(texture).with_color(BLACK.into()),
        BackgroundColor(WHITE.into()),
        BorderRadius::all(px(f32::MAX)),
        Outline {
            width: px(1),
            offset: px(3),
            color: Color::WHITE,
        },
    )
}
