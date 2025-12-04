// =============================================================================
// WGSL Shader - パケットとノードの描画
// =============================================================================

struct TimeUniform {
    time: f32,
    _padding: vec3<f32>,
}
@group(0) @binding(0) var<uniform> time_data: TimeUniform;

// 頂点シェーダー出力 / フラグメントシェーダー入力
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
    @location(0) entity_pos: vec2<f32>,   // x, y
    @location(1) entity_color: vec3<f32>, // r, g, b
    @location(2) entity_size: f32,        // size
) -> VertexOutput {
    var output: VertexOutput;
    
    // サイズに基づいて四角形の頂点を計算
    let size = entity_size;
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
    
    // パケットの場合のみアニメーション（サイズが小さい場合）
    var animated_pos = entity_pos;
    if (size < 10.0) {
        let wave = sin(time_data.time * 5.0 + entity_pos.y * 0.05) * 3.0;
        animated_pos = vec2<f32>(entity_pos.x + wave, entity_pos.y);
    }

    // クリップ座標に変換
    let canvas_width = 1920.0;
    let canvas_height = 1080.0;
    let world_pos = animated_pos + pos;
    let x = (world_pos.x / canvas_width) * 2.0 - 1.0;
    let y = 1.0 - (world_pos.y / canvas_height) * 2.0;
    
    output.position = vec4<f32>(x, y, 0.0, 1.0);
    output.color = vec4<f32>(entity_color, 1.0);
    
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return input.color;
}
