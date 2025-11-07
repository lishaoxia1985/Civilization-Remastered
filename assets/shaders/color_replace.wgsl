#import bevy_sprite::mesh2d_vertex_output::VertexOutput

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> inner_color: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var<uniform> outer_color: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var base_color_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var base_color_sampler: sampler;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(base_color_texture, base_color_sampler, in.uv);
    
    let r = color.r;
    let g = color.g;
    let b = color.b;
    
    return vec4<f32>(
        r * inner_color.r + g * outer_color.r,
        r * inner_color.g + g * outer_color.g,
        r * inner_color.b + g * outer_color.b,
        color.a
     );
}
