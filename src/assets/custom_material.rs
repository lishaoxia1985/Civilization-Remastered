use bevy::{
    asset::{Asset, Handle},
    color::LinearRgba,
    image::Image,
    reflect::TypePath,
    render::render_resource::AsBindGroup,
    shader::ShaderRef,
    sprite_render::{AlphaMode2d, Material2d},
};

#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
pub struct ColorReplaceMaterial {
    #[uniform(0)]
    pub inner_color: LinearRgba,
    #[uniform(1)]
    pub outer_color: LinearRgba,
    #[texture(2)]
    #[sampler(3)]
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
