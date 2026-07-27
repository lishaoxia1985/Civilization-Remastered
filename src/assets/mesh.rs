//! 自定义网格
//!
//! 定义游戏中使用的各种网格生成函数。

use bevy::{
    asset::RenderAssetUsages,
    math::Vec3,
    mesh::{Indices, Mesh, PrimitiveTopology},
};
use civ_map_generator::grid::{Hex, HexGrid};

/// 生成线段网格
pub fn line_mesh(start: Vec3, end: Vec3, width: f32) -> Mesh {
    let direction = end - start;
    let _length = direction.length();
    let normalized_direction = direction.normalize();

    let perpendicular =
        Vec3::new(-normalized_direction.y, normalized_direction.x, 0.0).normalize() * width / 2.0;

    let vertices = vec![
        start + perpendicular,
        start - perpendicular,
        end + perpendicular,
        end - perpendicular,
    ];

    let uvs = vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]];

    let indices = Indices::U32(vec![0, 1, 2, 2, 1, 3]);

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.with_inserted_indices(indices)
}

/// 生成六边形网格
pub fn hex_mesh(grid: &HexGrid) -> Mesh {
    let hex_layout = &grid.layout;
    let vertices: Vec<[f32; 3]> = hex_layout
        .all_corners(Hex::new(0, 0))
        .map(|corner| [corner[0], corner[1], 0.0])
        .to_vec();

    let indices = Indices::U32(vec![0, 1, 2, 0, 2, 3, 0, 3, 4, 0, 4, 5]);

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
    mesh.with_inserted_indices(indices)
}
