struct TimeUniform {
    time: f32,
    _padding: vec3<f32>,
}
@group(0) @binding(0) var<uniform> time_data: TimeUniform;

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
    @location(0) packet_pos: vec2<f32>,
) -> @builtin(position) vec4<f32> {
    let size = 2.0;

    var pos = vec2<f32>(0.0, 0.0);
    if (vertex_index == 0u) {
        pos = vec2<f32>(-size, -size);
    } else if (vertex_index == 1u) {
        pos = vec2<f32>( size, -size);
    } else if (vertex_index == 2u) {
        pos = vec2<f32>(-size,  size);
    } else {
        pos = vec2<f32>( size,  size); 
    }
    
    // アニメーション計算: 時間とY座標を使ってX座標を揺らす
    let wave = sin(time_data.time * 5.0 + packet_pos.y * 0.05) * 10.0;
    let animated_pos = vec2<f32>(packet_pos.x + wave, packet_pos.y);

    let canvas_width = 800.0;
    let canvas_height = 600.0;
    let world_pos = animated_pos + pos;
    let x = (world_pos.x / canvas_width) * 2.0 - 1.0;
    let y = 1.0 - (world_pos.y / canvas_height) * 2.0;
    
    return vec4<f32>(x, y, 0.0, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 1.0, 1.0, 1.0);
}

