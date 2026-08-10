//! 环形进度条材质（UiMaterial）
//!
//! 在单位图标周围绘制左右半圆环进度条（无需 shader 采样图标，图标由独立 ImageNode 叠加）：
//! - 左半圆环：HP 进度（绿，受伤时缩短）
//! - 右半圆环：XP 进度（蓝，积满升级）
//! - 未填充环：灰色背景；环外与环内：全透明

use bevy::{
    asset::Asset,
    color::LinearRgba,
    reflect::TypePath,
    render::render_resource::{AsBindGroup, RenderPipelineDescriptor},
    shader::ShaderRef,
    ui_render::ui_material::{UiMaterial, UiMaterialKey},
};

/// 环形进度条材质
#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
pub struct RingProgressMaterial {
    /// 左半圆环进度 (0.0 ~ 1.0) - HP
    #[uniform(0)]
    pub left_progress: f32,
    /// 右半圆环进度 (0.0 ~ 1.0) - XP
    #[uniform(1)]
    pub right_progress: f32,
    /// 左半圆环颜色 - HP
    #[uniform(2)]
    pub left_color: LinearRgba,
    /// 右半圆环颜色 - XP
    #[uniform(3)]
    pub right_color: LinearRgba,
    /// 未填充环颜色
    #[uniform(4)]
    pub background_color: LinearRgba,
    /// 环厚度（UV 百分比，0.0 ~ 0.5）
    #[uniform(5)]
    pub ring_thickness: f32,
}

impl UiMaterial for RingProgressMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/ring_progress.wgsl".into()
    }

    fn specialize(_descriptor: &mut RenderPipelineDescriptor, _key: UiMaterialKey<Self>) {}
}
