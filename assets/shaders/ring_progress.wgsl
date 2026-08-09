#import bevy_ui::ui_vertex_output::UiVertexOutput

@group(1) @binding(0) var<uniform> left_progress: f32;
@group(1) @binding(1) var<uniform> right_progress: f32;
@group(1) @binding(2) var<uniform> left_color: vec4<f32>;
@group(1) @binding(3) var<uniform> right_color: vec4<f32>;
@group(1) @binding(4) var<uniform> background_color: vec4<f32>;
@group(1) @binding(5) var<uniform> ring_thickness: f32;

const PI: f32 = 3.14159265358979323846;

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    // UV (0,0) 左上角 → 以中心为原点坐标
    let uv = in.uv * 2.0 - 1.0;
    let x = uv.x;
    let y = -uv.y;

    let dist = sqrt(x * x + y * y);
    let angle = atan2(y, x); // -PI ~ PI，0 在右

    let outer_radius = 1.0;
    let inner_radius = 1.0 - ring_thickness;

    // 不在环带内 → 全透明（环内留给叠加的图标显示）
    if (dist < inner_radius || dist > outer_radius) {
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }

    // 环带内默认灰色背景
    var color = background_color;

    // 左半环（HP）：角度 ≥ PI/2 或 ≤ -PI/2，从顶部逆时针填充
    if (angle >= PI / 2.0 || angle <= -PI / 2.0) {
        var normalized_angle: f32;
        if (angle >= PI / 2.0) {
            normalized_angle = (angle - PI / 2.0) / PI; // 0.0 ~ 0.5
        } else {
            normalized_angle = (angle + PI) / PI + 0.5; // 0.5 ~ 1.0
        }
        if (normalized_angle <= left_progress) {
            color = left_color;
        }
    }

    // 右半环（XP）：角度 -PI/2 ~ PI/2，从顶部顺时针填充
    if (angle > -PI / 2.0 && angle < PI / 2.0) {
        let normalized_angle = (PI / 2.0 - angle) / PI; // 0.0 ~ 1.0
        if (normalized_angle <= right_progress) {
            color = right_color;
        }
    }

    return color;
}