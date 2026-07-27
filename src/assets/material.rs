//! 自定义材质
//!
//! 定义游戏中使用的自定义着色器材质。

use bevy::{
    asset::{Asset, Handle},
    color::LinearRgba,
    image::Image,
    reflect::TypePath,
    render::render_resource::AsBindGroup,
    shader::ShaderRef,
    sprite_render::{AlphaMode2d, Material2d},
};

/// 颜色替换材质 - 用于单位图标等需要动态着色的对象
#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
pub struct ColorReplaceMaterial {
    #[uniform(0)]
    /// 内部颜色
    pub inner_color: LinearRgba,
    #[uniform(1)]
    /// 外部颜色
    pub outer_color: LinearRgba,
    #[texture(2)]
    #[sampler(3)]
    /// 纹理
    pub texture: Handle<Image>,
}

impl Material2d for ColorReplaceMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/color_replace.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}
