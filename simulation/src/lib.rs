use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;
use wgpu::util::DeviceExt;
use wgpu::*;

// =============================================================================
// WEBGPU RENDERER
// =============================================================================
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
// シェーダーに時間を渡すためのユニフォームバッファ構造体。アライメント調整用のパディングを含む
struct TimeUniform {
    time: f32,
    _padding: [f32; 7],
}

// WebGPUのデバイス、キュー、パイプラインなど、描画に必要なリソースをまとめて管理する構造体
struct GpuRenderer {
    device: Device,
    queue: Queue,
    render_pipeline: RenderPipeline,
    packet_buffer: Buffer, // 描画したいパケットの座標データをGPUメモリ上に保持するための領域
    packet_count: u32,     // 現在バッファに含まれている、あるいは描画するべきパケットの数を管理する
    surface: Surface<'static>, // 画面(この場合はHTMLの<canvas>要素)への描画領域を表す。レンダリング結果を最終的にユーザーに見せるための窓口
    surface_config: SurfaceConfiguration, // サーフェスの設定情報を保持する。ウィンドウサイズが変わった際など、surfaceを再設定するために必要
    canvas_width: u32,

    canvas_height: u32,
    time_buffer: Buffer,
    time_bind_group: BindGroup,
}

// 初期化したGpuRendererインスタンスをプログラムのどこからでもアクセスできるように保持しておく場所。
// [LEARN]thread_local!マクロを使用することで、「スレッドごとに1つ(=Wasm環境全体で実質1つ)の書き換え可能な永続データ」を安全に管理できる
// GPUレンダラー置き場を作っておき、最初は空にする。
// init_gpu関数で初期化が成功したら、ここにレンダラーを格納し、描画関数でここからレンダーを取り出して使う
thread_local! {
    // GpuRendererのインスタンスをスレッドローカル（Wasmでは実質グローバル）に保持するための変数
    static GPU_RENDERER: RefCell<Option<GpuRenderer>> = RefCell::new(None);
}

// JavaScriptからRustへ大量のデータを渡す際や、計算結果を一時的に保持するための「使いまわし可能なメモリ領域」
// 毎回新しいメモリを確保して開放するのは遅いため、このPACKET_BUFFERという「常駐する領域」を1つ用意しておく
thread_local! {
    // JSとRust間でデータをやり取りするための一時的な共有メモリバッファ
    static PACKET_BUFFER: RefCell<Vec<f32>> = RefCell::new(Vec::new());
}

const WIDTH: f32 = 800.0;
const HEIGHT: f32 = 600.0;

// WGSL言語で記述された頂点シェーダーとフラグメントシェーダーのソースコード（外部ファイルから読み込み）
const SHADER_SOURCE: &str = include_str!("shader.wgsl");

// JSから呼び出されるWebGPU初期化のエントリーポイント。非同期処理のPromiseを返す
#[wasm_bindgen]
pub fn init_gpu(canvas_id: &str) -> JsValue {
    let canvas_id = canvas_id.to_string();
    wasm_bindgen_futures::future_to_promise(async move {
        init_gpu_internal(&canvas_id)
            .await
            .map(|_| JsValue::TRUE)
            .map_err(|e| e)
    })
    .into()
}

// 実際のWebGPU初期化処理を行う非同期関数。デバイスやパイプラインの作成を行う
async fn init_gpu_internal(canvas_id: &str) -> Result<(), JsValue> {
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("no global Window exists"))?;

    let document = window
        .document()
        .ok_or_else(|| JsValue::from_str("no Document exists"))?;

    let canvas = document
        .get_element_by_id(canvas_id)
        .and_then(|e| e.dyn_into::<HtmlCanvasElement>().ok())
        .ok_or_else(|| JsValue::from_str("canvas element not found"))?;

    let canvas_width = canvas.width();
    let canvas_height = canvas.height();

    log(&format!(
        "[Rust/Wasm] Initializing WebGPU for canvas {}x{}",
        canvas_width, canvas_height
    ));

    let instance = Instance::new(&InstanceDescriptor {
        backends: Backends::BROWSER_WEBGPU | Backends::GL,
        ..Default::default()
    });

    let surface = instance
        .create_surface(SurfaceTarget::Canvas(canvas))
        .expect("Failed to create surface");

    let adapter = match instance
        .request_adapter(&RequestAdapterOptions {
            power_preference: PowerPreference::default(),
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .await
    {
        Some(adapter) => adapter,
        None => {
            log("[Rust/Wasm] Failed to get WebGPU adapter");
            return Err(JsValue::from_str("Failed to get WebGPU adapter"));
        }
    };

    let (device, queue) = match adapter
        .request_device(
            &DeviceDescriptor {
                label: None,
                required_features: Features::empty(),
                required_limits: Limits::downlevel_webgl2_defaults()
                    .using_resolution(adapter.limits()),
                memory_hints: MemoryHints::default(),
            },
            None,
        )
        .await
    {
        Ok(result) => result,
        Err(e) => {
            let err_msg = format!("Failed to get WebGPU device: {:?}", e);
            log(&err_msg);
            return Err(JsValue::from_str(&err_msg));
        }
    };

    let surface_caps = surface.get_capabilities(&adapter);
    let surface_format = surface_caps
        .formats
        .iter()
        .find(|f| matches!(f, TextureFormat::Bgra8UnormSrgb | TextureFormat::Bgra8Unorm))
        .copied()
        .unwrap_or(surface_caps.formats[0]);

    let surface_config = SurfaceConfiguration {
        usage: TextureUsages::RENDER_ATTACHMENT,
        format: surface_format,
        width: canvas_width,
        height: canvas_height,
        present_mode: surface_caps.present_modes[0],
        alpha_mode: surface_caps.alpha_modes[0],
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    };

    surface.configure(&device, &surface_config);

    let time_uniform = TimeUniform {
        time: 0.0,
        _padding: [0.0; 7],
    };

    let time_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Time Buffer"),
        contents: bytemuck::cast_slice(&[time_uniform]),
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
    });

    let time_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        entries: &[BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::VERTEX,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
        label: Some("time_bind_group_layout"),
    });

    let time_bind_group = device.create_bind_group(&BindGroupDescriptor {
        layout: &time_bind_group_layout,
        entries: &[BindGroupEntry {
            binding: 0,
            resource: time_buffer.as_entire_binding(),
        }],
        label: Some("time_bind_group"),
    });

    let render_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: Some("Render Pipeline Layout"),
        bind_group_layouts: &[&time_bind_group_layout],
        push_constant_ranges: &[],
    });

    let shader = device.create_shader_module(ShaderModuleDescriptor {
        label: Some("Packet Shader"),
        source: ShaderSource::Wgsl(SHADER_SOURCE.into()),
    });

    let render_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some("Packet Render Pipeline"),
        layout: Some(&render_pipeline_layout),
        vertex: VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[VertexBufferLayout {
                array_stride: std::mem::size_of::<f32>() as u64 * 2,
                step_mode: VertexStepMode::Instance,
                attributes: &[VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: VertexFormat::Float32x2,
                }],
            }],
            compilation_options: PipelineCompilationOptions::default(),
        },
        fragment: Some(FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(ColorTargetState {
                format: surface_config.format,
                blend: Some(BlendState::REPLACE),
                write_mask: ColorWrites::ALL,
            })],
            compilation_options: PipelineCompilationOptions::default(),
        }),
        primitive: PrimitiveState {
            topology: PrimitiveTopology::TriangleStrip,
            strip_index_format: None,
            front_face: FrontFace::Ccw,
            cull_mode: None,
            unclipped_depth: false,
            polygon_mode: PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: None,
        multisample: MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview: None,
        cache: None,
    });

    let max_packets = 100_000;
    let packet_buffer = device.create_buffer(&BufferDescriptor {
        label: Some("Packet Buffer"),
        size: (max_packets * 2 * std::mem::size_of::<f32>()) as u64,
        usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let renderer = GpuRenderer {
        device,
        queue,
        render_pipeline,
        packet_buffer,
        packet_count: 0,
        surface,
        surface_config,
        canvas_width,
        canvas_height,
        time_buffer,
        time_bind_group,
    };

    GPU_RENDERER.with(|r| {
        *r.borrow_mut() = Some(renderer);
    });

    log("[Rust/Wasm] WebGPU initialized successfully!");
    Ok(())
}

// 一度に描画できるパケットの最大数
const MAX_PACKETS: usize = 100_000;

// 与えられた座標データを使ってGPUでパケットを描画する関数
fn render_packets_gpu(coords: &[f32]) {
    GPU_RENDERER.with(|renderer_ref| {
        let mut renderer_opt = renderer_ref.borrow_mut();
        if let Some(renderer) = renderer_opt.as_mut() {
            let total_packets = coords.len() / 2;
            if total_packets == 0 {
                log("[Rust/Wasm] No packets to render");
                return;
            }

            let packet_count = total_packets.min(MAX_PACKETS);
            let coords_to_render = &coords[0..(packet_count * 2)];

            if total_packets > MAX_PACKETS {
                log(&format!(
                    "[Rust/Wasm] Warning: {} packets received, rendering only {} (buffer limit)",
                    total_packets, packet_count
                ));
            } else {
                log(&format!("[Rust/Wasm] Rendering {} packets", packet_count));
            }

            renderer.queue.write_buffer(
                &renderer.packet_buffer,
                0,
                bytemuck::cast_slice(coords_to_render),
            );

            let current_time = (now() / 1000.0) as f32;
            let time_data = TimeUniform {
                time: current_time,
                _padding: [0.0; 7],
            };
            renderer.queue.write_buffer(
                &renderer.time_buffer,
                0,
                bytemuck::cast_slice(&[time_data]),
            );

            let surface_texture = match renderer.surface.get_current_texture() {
                Ok(texture) => texture,
                Err(e) => {
                    log(&format!(
                        "[Rust/Wasm] Failed to get surface texture: {:?}",
                        e
                    ));
                    return;
                }
            };

            let view = surface_texture
                .texture
                .create_view(&TextureViewDescriptor::default());

            {
                let mut encoder =
                    renderer
                        .device
                        .create_command_encoder(&CommandEncoderDescriptor {
                            label: Some("Render Encoder"),
                        });

                {
                    let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                        label: Some("Render Pass"),
                        color_attachments: &[Some(RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: Operations {
                                load: LoadOp::Clear(Color {
                                    r: 0.050980392156862744, // #0d1117
                                    g: 0.050980392156862744,
                                    b: 0.09019607843137255,
                                    a: 1.0,
                                }),
                                store: StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        occlusion_query_set: None,
                        timestamp_writes: None,
                    });

                    render_pass.set_pipeline(&renderer.render_pipeline);
                    render_pass.set_bind_group(0, &renderer.time_bind_group, &[]);
                    let buffer_size = (packet_count * 2 * std::mem::size_of::<f32>()) as u64;
                    render_pass.set_vertex_buffer(0, renderer.packet_buffer.slice(0..buffer_size));
                    render_pass.draw(0..4, 0..packet_count as u32);
                }

                renderer.queue.submit(Some(encoder.finish()));
            }

            surface_texture.present();
            renderer.packet_count = packet_count as u32;
            log(&format!(
                "[Rust/Wasm] Rendered {} packets successfully",
                packet_count
            ));
        } else {
            log("[Rust/Wasm] GPU renderer not initialized");
        }
    });
}

// アニメーションフレームごとに呼び出され、画面を再描画する関数
#[wasm_bindgen]
pub fn render_frame() {
    GPU_RENDERER.with(|renderer_ref| {
        let mut renderer_opt = renderer_ref.borrow_mut();
        if let Some(renderer) = renderer_opt.as_mut() {
            let packet_count = renderer.packet_count as usize;
            if packet_count == 0 {
                return;
            }

            let current_time = (now() / 1000.0) as f32;
            let time_data = TimeUniform {
                time: current_time,
                _padding: [0.0; 7],
            };
            renderer.queue.write_buffer(
                &renderer.time_buffer,
                0,
                bytemuck::cast_slice(&[time_data]),
            );

            let surface_texture = match renderer.surface.get_current_texture() {
                Ok(texture) => texture,
                Err(_) => return,
            };

            let view = surface_texture
                .texture
                .create_view(&TextureViewDescriptor::default());

            {
                let mut encoder =
                    renderer
                        .device
                        .create_command_encoder(&CommandEncoderDescriptor {
                            label: Some("Render Encoder"),
                        });

                {
                    let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                        label: Some("Render Pass"),
                        color_attachments: &[Some(RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: Operations {
                                load: LoadOp::Clear(Color {
                                    r: 0.050980392156862744,
                                    g: 0.050980392156862744,
                                    b: 0.09019607843137255,
                                    a: 1.0,
                                }),
                                store: StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        occlusion_query_set: None,
                        timestamp_writes: None,
                    });

                    render_pass.set_pipeline(&renderer.render_pipeline);
                    render_pass.set_bind_group(0, &renderer.time_bind_group, &[]);
                    let buffer_size = (packet_count * 2 * std::mem::size_of::<f32>()) as u64;
                    render_pass.set_vertex_buffer(0, renderer.packet_buffer.slice(0..buffer_size));
                    render_pass.draw(0..4, 0..packet_count as u32);
                }

                renderer.queue.submit(Some(encoder.finish()));
            }

            surface_texture.present();
        }
    });
}

// 共有バッファのメモリアドレス（ポインタ）をJSに返す関数
#[wasm_bindgen]
pub fn get_packet_buffer_ptr() -> *const f32 {
    PACKET_BUFFER.with(|buffer| buffer.borrow().as_ptr())
}

// 共有バッファの現在の長さをJSに返す関数
#[wasm_bindgen]
pub fn get_packet_buffer_len() -> usize {
    PACKET_BUFFER.with(|buffer| buffer.borrow().len())
}

// 共有バッファのメモリ領域を指定サイズ分確保する関数
#[wasm_bindgen]
pub fn allocate_packet_buffer(capacity: usize) {
    PACKET_BUFFER.with(|buffer| {
        let mut buf = buffer.borrow_mut();
        buf.clear();
        buf.reserve(capacity * 2);
        log(&format!(
            "[Rust/Wasm] Allocated packet buffer with capacity for {} packets ({} bytes)",
            capacity,
            capacity * 2 * std::mem::size_of::<f32>()
        ));
    });
}

// 共有バッファの内容をクリアする関数
#[wasm_bindgen]
pub fn clear_packet_buffer() {
    PACKET_BUFFER.with(|buffer| {
        buffer.borrow_mut().clear();
    });
}

// バイナリデータからパケット情報を読み取り、共有バッファを更新する関数
#[wasm_bindgen]
pub fn update_packet_buffer_from_binary(data: &[u8]) -> usize {
    let packet_count = data.len() / 8;

    PACKET_BUFFER.with(|buffer| {
        let mut buf = buffer.borrow_mut();
        buf.clear();

        let required = packet_count * 2;
        let current_capacity = buf.capacity();
        if current_capacity < required {
            buf.reserve(required - current_capacity);
        }

        for i in 0..packet_count {
            let offset = i * 8;

            let x16 = u16::from_le_bytes([data[offset + 4], data[offset + 5]]);
            let x = (x16 as f32) * width / 65535.0;

            let y16 = u16::from_le_bytes([data[offset + 6], data[offset + 7]]);
            let y = (y16 as f32) * height / 65535.0;

            buf.push(x);
            buf.push(y);
        }
    });

    packet_count
}

// JSON文字列からパケット情報を読み取り、共有バッファを更新する関数
#[wasm_bindgen]
pub fn update_packet_buffer_from_json(json_data: &str) -> usize {
    let packets: Vec<Packet> = match serde_json::from_str(json_data) {
        Ok(p) => p,
        Err(_) => return 0,
    };

    PACKET_BUFFER.with(|buffer| {
        let mut buf = buffer.borrow_mut();
        buf.clear();

        let required = packets.len() * 2;
        let current_capacity = buf.capacity();
        if current_capacity < required {
            buf.reserve(required - current_capacity);
        }

        for packet in &packets {
            buf.push(packet.x as f32);
            buf.push(packet.y as f32);
        }

        packets.len()
    })
}

// WasmのメモリインスタンスをJSに返す関数
#[wasm_bindgen]
pub fn get_memory() -> JsValue {
    wasm_bindgen::memory()
}

// パケットのデータを表す構造体。JSONのシリアライズ/デシリアライズに対応
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Packet {
    pub id: u32,
    pub x: f64,
    pub y: f64,
}

#[wasm_bindgen]
// JS側の関数（console.log, performance.now）をRustで使うための宣言
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);

    #[wasm_bindgen(js_namespace = performance)]
    fn now() -> f64;
}

// JSのconsole.logをRustから使いやすくラップした関数
#[wasm_bindgen]
pub fn console_log(message: &str) {
    log(message);
}

// WebSocketなどで受信したメッセージ（JSONまたは文字列）を処理し、描画を行う関数
#[wasm_bindgen]
pub fn handle_message(message: &str) {
    let msg_size = message.len();
    log(&format!(
        "[Rust/Wasm] Received: {} bytes ({:.2} KB)",
        msg_size,
        msg_size as f64 / 1024.0
    ));

    let start_parse = now();
    if let Ok(packets) = serde_json::from_str::<Vec<Packet>>(message) {
        let parse_time = now() - start_parse;

        log(&format!(
            "[Rust/Wasm] Parsed {} packets in {:.2}ms",
            packets.len(),
            parse_time
        ));

        let start_convert = now();
        let coords: Vec<f32> = packets
            .iter()
            .flat_map(|p| [p.x as f32, p.y as f32])
            .collect();
        let convert_time = now() - start_convert;

        let start_draw = now();
        render_packets_gpu(&coords);
        let draw_time = now() - start_draw;

        log(&format!(
            "[Rust/Wasm] Performance: parse={:.2}ms, convert={:.2}ms, draw={:.2}ms, total={:.2}ms",
            parse_time,
            convert_time,
            draw_time,
            parse_time + convert_time + draw_time
        ));
        log(&format!(
            "[Rust/Wasm] JSON overhead: {:.2} bytes/packet",
            msg_size as f64 / packets.len() as f64
        ));

        return;
    }

    match serde_json::from_str::<Packet>(message) {
        Ok(packet) => {
            log(&format!(
                "[Rust/Wasm] Parsed single Packet: id={}, x={}, y={}",
                packet.id, packet.x, packet.y
            ));
            let coords = vec![packet.x as f32, packet.y as f32];
            render_packets_gpu(&coords);
        }
        Err(_) => {
            log(&format!("[Rust/Wasm] Plain text: {}", message));
        }
    }
}

// Wasmモジュール読み込み時に自動実行されるエントリーポイント
#[wasm_bindgen(start)]
pub fn main() {
    log("[Rust/Wasm] Module initialized!");
}

// バイナリ形式のパケットデータを受け取り、解析して描画する関数
#[wasm_bindgen]
pub fn handle_binary(data: &[u8]) {
    let packet_count = data.len() / 8;

    let mut coords: Vec<f32> = Vec::with_capacity(packet_count * 2);

    for i in 0..packet_count {
        let offset = i * 8;

        let x16 = u16::from_le_bytes([data[offset + 4], data[offset + 5]]);
        let x = (x16 as f32) * WIDTH / 65535.0;

        let y16 = u16::from_le_bytes([data[offset + 6], data[offset + 7]]);
        let y = (y16 as f32) * HEIGHT / 65535.0;

        coords.push(x);
        coords.push(y);
    }

    render_packets_gpu(&coords);
}
