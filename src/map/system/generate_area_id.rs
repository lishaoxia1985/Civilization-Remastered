use std::collections::{BTreeMap, BTreeSet, VecDeque};

use bevy::{
    prelude::{Commands, Entity, Query, Res, ResMut},
    utils::hashbrown::{HashMap, HashSet},
};

use crate::{
    map::{
        component::AreaId,
        terrain_type::TerrainType,
        tile_query::{TileQuery, TileQueryItem},
        AreaIdAndSize, TileStorage,
    },
    tile_map::MapParameters,
};

pub fn bfs(
    commands: &mut Commands,
    map_parameters: &MapParameters,
    tile_storage: &TileStorage,
    area_id_and_size: &mut AreaIdAndSize,
    filter_condition: impl Fn(&TileQueryItem) -> bool,
    query_tile: &Query<TileQuery>,
) {
    let mut area_entities = query_tile
        .iter()
        .filter(|tile| filter_condition(tile))
        .map(|tile| tile.entity)
        .collect::<HashSet<_>>();

    let mut current_area_id = query_tile.iter().map(|tile| tile.area_id.0).max().unwrap() + 1;

    while let Some(&initial_area_entity) = area_entities.iter().next() {
        commands
            .entity(initial_area_entity)
            .insert(AreaId(current_area_id));
        area_entities.remove(&initial_area_entity);

        // Store all the entities in the current area.
        let mut entities_in_current_area = HashSet::new();
        entities_in_current_area.insert(initial_area_entity);

        // Store all the entities that need to check whether their neighbors are in the current area within the following 'while {..}' loop.
        let mut entities_to_check = VecDeque::new();
        entities_to_check.push_back(initial_area_entity);

        while let Some(entity_we_are_checking) = entities_to_check.pop_front() {
            let tile_we_are_checking = query_tile.get(entity_we_are_checking).unwrap();
            tile_we_are_checking
                .hex_position
                .entity_neighbors(&tile_storage, &map_parameters)
                .iter()
                .for_each(|&entity| {
                    if !entities_in_current_area.contains(&entity)
                        && area_entities.contains(&entity)
                    {
                        entities_in_current_area.insert(entity);
                        commands.entity(entity).insert(AreaId(current_area_id));
                        entities_to_check.push_back(entity);
                        area_entities.remove(&entity);
                    }
                });
        }
        area_id_and_size
            .0
            .insert(current_area_id, entities_in_current_area.len() as u32);
        current_area_id += 1;
    }
}

pub fn dfs(
    mut commands: Commands,
    map_parameters: Res<MapParameters>,
    tile_storage: Res<TileStorage>,
    area_id_and_size: &mut AreaIdAndSize,
    filter_condition: impl Fn(&TileQueryItem) -> bool,
    query_tile: Query<TileQuery>,
) {
    let mut area_entities = query_tile
        .iter()
        .filter(|tile| filter_condition(tile))
        .map(|tile| tile.entity)
        .collect::<HashSet<_>>();

    let mut current_area_id = query_tile.iter().map(|tile| tile.area_id.0).max().unwrap() + 1;

    while let Some(&initial_area_entity) = area_entities.iter().next() {
        commands
            .entity(initial_area_entity)
            .insert(AreaId(current_area_id));
        area_entities.remove(&initial_area_entity);

        // Store all the entities in the current area.
        let mut entities_in_current_area = HashSet::new();
        entities_in_current_area.insert(initial_area_entity);
        // Store all the entities that need to check whether their neighbors are in the current area within the following 'while {..}' loop.
        let mut entities_to_check = Vec::new();
        entities_to_check.push(initial_area_entity);

        while let Some(entity_we_are_checking) = entities_to_check.pop() {
            let tile_we_are_checking = query_tile.get(entity_we_are_checking).unwrap();
            tile_we_are_checking
                .hex_position
                .entity_neighbors(&tile_storage, &map_parameters)
                .iter()
                .for_each(|&entity| {
                    if !entities_in_current_area.contains(&entity)
                        && area_entities.contains(&entity)
                    {
                        entities_in_current_area.insert(entity);
                        commands.entity(entity).insert(AreaId(current_area_id));
                        entities_to_check.push(entity);
                        area_entities.remove(&entity);
                    }
                });
        }
        area_id_and_size
            .0
            .insert(current_area_id, entities_in_current_area.len() as u32);
        current_area_id += 1;
    }
}

pub fn recalculate_areas(
    mut commands: Commands,
    map_parameters: Res<MapParameters>,
    tile_storage: Res<TileStorage>,
    mut area_id_and_size: ResMut<AreaIdAndSize>,
    query_tile: Query<TileQuery>,
) {
    area_id_and_size.0.clear();

    query_tile.iter().into_iter().for_each(|tile| {
        commands.entity(tile.entity).insert(AreaId(-1));
    });

    let water_condition = |tile: &TileQueryItem| tile.terrain_type == &TerrainType::Water;

    let hill_and_flatland_condition = |tile: &TileQueryItem| {
        tile.terrain_type == &TerrainType::Flatland || tile.terrain_type == &TerrainType::Hill
    };

    let mountain_condition = |tile: &TileQueryItem| tile.terrain_type == &TerrainType::Mountain;

    let conditions = vec![
        water_condition,
        hill_and_flatland_condition,
        mountain_condition,
    ];

    conditions.iter().for_each(|condition| {
        bfs(
            &mut commands,
            &map_parameters,
            &tile_storage,
            &mut area_id_and_size,
            condition,
            &query_tile,
        );
    });
}

pub fn reassign_area_id(
    mut commands: Commands,
    map_parameters: Res<MapParameters>,
    tile_storage: Res<TileStorage>,
    mut area_id_and_size: ResMut<AreaIdAndSize>,
    query_tile: Query<TileQuery>,
) {
    const MIN_AREA_SIZE: u32 = 7;

    // Get id of the smaller area whose size < MIN_AREA_SIZE
    let small_area_id: Vec<_> = area_id_and_size
        .0
        .iter()
        .filter(|(_, size)| **size < MIN_AREA_SIZE)
        .map(|(&id, _)| id)
        .collect();

    let mut reassign_id_entities_and_area_id: HashMap<Entity, i32> = HashMap::new();

    small_area_id.into_iter().for_each(|area_id| {
        let current_area_entities = query_tile
            .iter()
            .filter(|tile| tile.area_id.0 == area_id)
            .map(|tile| tile.entity)
            .collect::<Vec<_>>();

        let area_is_water = query_tile
            .get(current_area_entities[0])
            .unwrap()
            .terrain_type
            == &TerrainType::Water;

        // Get the border entities of the current area, these entities don't belong to the area, but they surround the area.
        // Using BTreeSet to store the border entities will make sure the entities are processed in the same order every time.
        // That means that we can get the same 'surround_area_size_and_id' every time.
        let mut border_entities = BTreeSet::new();

        current_area_entities.iter().for_each(|&entity| {
            let tile = query_tile.get(entity).unwrap();
            // Get the neighbor entities of the current tile
            let neighbor_entities = tile
                .hex_position
                .entity_neighbors(&tile_storage, &map_parameters);
            // Get the neighbor entities that don't belong to the current area and add them to the border entities
            neighbor_entities.into_iter().for_each(|neighbor_entity| {
                let neighbor_tile = query_tile.get(neighbor_entity).unwrap();
                let neighbor_is_water = neighbor_tile.terrain_type == &TerrainType::Water;
                if area_is_water == neighbor_is_water
                    && !current_area_entities.contains(&neighbor_entity)
                {
                    border_entities.insert(neighbor_entity);
                }
            });
        });

        // Get the size and area id of the surround area
        // Notice: different surround area may have the same size, we use BTreeMap only to retain the last added pair (area_size, area_id)
        let surround_area_size_and_id: BTreeMap<u32, i32> = border_entities
            .iter()
            .map(|&entity| {
                if let Some(&reassign_id) = reassign_id_entities_and_area_id.get(&entity) {
                    // If the entity is reassign, use the reassign area id
                    let reassign_area_size = area_id_and_size.0[&reassign_id];
                    (reassign_area_size, reassign_id)
                } else {
                    // Otherwise, use the original area id
                    let tile = query_tile.get(entity).unwrap();
                    let area_size = area_id_and_size.0[&tile.area_id.0];
                    (area_size, tile.area_id.0)
                }
            })
            .collect();

        // Merge the current small area with the largest surround area whose size is >= MIN_AREA_SIZE and its terrain type is the same as the current area
        // Get the area id of the largest surround area and assign it to the current area
        if let Some((&area_size, &area_id)) = surround_area_size_and_id.last_key_value() {
            if area_size >= MIN_AREA_SIZE {
                let first_entity = current_area_entities[0];
                let old_area_id = query_tile.get(first_entity).unwrap().area_id;
                area_id_and_size.0.remove(&old_area_id.0);

                area_id_and_size
                    .0
                    .entry(area_id)
                    .and_modify(|e| *e += current_area_entities.len() as u32);

                current_area_entities.iter().for_each(|&entity| {
                    commands.entity(entity).insert(AreaId(area_id));
                    reassign_id_entities_and_area_id.insert(entity, area_id);
                })
            }
        }
    });
}
