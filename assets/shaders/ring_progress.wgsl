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
    // 以中心为原点的坐标，y 向上
    let uv = in.uv * 2.0 - 1.0;
    let x = uv.x;
    let y = -uv.y;

    let dist = sqrt(x * x + y * y);
    let angle = atan2(y, x); // (-PI, PI]

    let outer_radius = 1.0;
    let inner_radius = 1.0 - ring_thickness;

    // ---- 抗锯齿：环带内外边缘的软 alpha ----
    // 像素到内边缘的距离（正=环内，负=环外）
    let dist_to_inner = dist - inner_radius;
    // 像素到外边缘的距离（正=环外，负=环内）
    let dist_to_outer = outer_radius - dist;

    // 根据屏幕像素变化率计算平滑过渡的半宽（通常 1~1.5 像素）
    let edge_smooth = fwidth(dist) * 1.0;

    let inner_alpha = smoothstep(0.0, edge_smooth, dist_to_inner);
    let outer_alpha = smoothstep(0.0, edge_smooth, dist_to_outer);
    let ring_alpha = inner_alpha * outer_alpha;

    // 如果完全不在环带内，直接丢弃可以省一些计算（可选）
    if (ring_alpha <= 0.0) {
        return vec4<f32>(0.0);
    }

    // ---- 默认背景色（环带内灰色） ----
    var color = background_color;

    // ---- 左半环（HP）：角度 >= PI/2 或 <= -PI/2，从顶部逆时针填充 ----
    if (angle >= PI / 2.0 || angle <= -PI / 2.0) {
        var normalized_angle: f32;
        if (angle >= PI / 2.0) {
            normalized_angle = (angle - PI / 2.0) / PI;      // [0, 0.5]
        } else {
            normalized_angle = (angle + PI) / PI + 0.5;      // (0.5, 1]
        }

        // 抗锯齿：进度边缘的软切换
        let progress_edge = smoothstep(0.0, edge_smooth, left_progress - normalized_angle);
        color = mix(color, left_color, progress_edge);
    }

    // ---- 右半环（XP）：角度 -PI/2 ~ PI/2，从顶部顺时针填充 ----
    if (angle > -PI / 2.0 && angle < PI / 2.0) {
        let normalized_angle = (PI / 2.0 - angle) / PI;      // [0, 1]

        let progress_edge = smoothstep(0.0, edge_smooth, right_progress - normalized_angle);
        color = mix(color, right_color, progress_edge);
    }

    return vec4<f32>(color.rgb, color.a * ring_alpha);
}