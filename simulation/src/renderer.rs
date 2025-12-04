// =============================================================================
// WEBGPU RENDERER - 描画担当
// =============================================================================

use bytemuck::{Pod, Zeroable};
use std::cell::RefCell;
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;
use wgpu::util::DeviceExt;
use wgpu::*;

// シェーダーに時間を渡すためのユニフォームバッファ構造体。アライメント調整用のパディングを含む
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct TimeUniform {
    pub time: f32,
    pub _padding: [f32; 7],
}

// WebGPUのデバイス、キュー、パイプラインなど、描画に必要なリソースをまとめて管理する構造体
pub struct GpuRenderer {
    pub device: Device,
    pub queue: Queue,
    pub render_pipeline: RenderPipeline,
    pub packet_buffer: Buffer,
    pub packet_count: u32,
    pub surface: Surface<'static>,
    #[allow(dead_code)]
    pub surface_config: SurfaceConfiguration,
    #[allow(dead_code)]
    pub canvas_width: u32,
    #[allow(dead_code)]
    pub canvas_height: u32,
    pub time_buffer: Buffer,
    pub time_bind_group: BindGroup,
}

// 初期化したGpuRendererインスタンスをプログラムのどこからでもアクセスできるように保持しておく場所。
thread_local! {
    pub static GPU_RENDERER: RefCell<Option<GpuRenderer>> = RefCell::new(None);
}

// WGSL言語で記述された頂点シェーダーとフラグメントシェーダーのソースコード（外部ファイルから読み込み）
const SHADER_SOURCE: &str = include_str!("shader.wgsl");

// 一度に描画できるパケットの最大数
pub const MAX_PACKETS: usize = 100_000;

// 背景色（#0d1117）
const BG_COLOR: Color = Color {
    r: 0.050980392156862744,
    g: 0.050980392156862744,
    b: 0.09019607843137255,
    a: 1.0,
};

// JS側の関数（performance.now）をRustで使うための宣言
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);

    #[wasm_bindgen(js_namespace = performance)]
    fn now() -> f64;
}

// 実際のWebGPU初期化処理を行う非同期関数。デバイスやパイプラインの作成を行う
pub async fn init_gpu_internal(canvas_id: &str) -> Result<(), JsValue> {
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

    // 新しいバッファレイアウト: [x, y, r, g, b, size] = 6 floats per entity
    let render_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some("Entity Render Pipeline"),
        layout: Some(&render_pipeline_layout),
        vertex: VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[VertexBufferLayout {
                array_stride: std::mem::size_of::<f32>() as u64 * 6, // x, y, r, g, b, size
                step_mode: VertexStepMode::Instance,
                attributes: &[
                    // position (x, y)
                    VertexAttribute {
                        offset: 0,
                        shader_location: 0,
                        format: VertexFormat::Float32x2,
                    },
                    // color (r, g, b)
                    VertexAttribute {
                        offset: std::mem::size_of::<f32>() as u64 * 2,
                        shader_location: 1,
                        format: VertexFormat::Float32x3,
                    },
                    // size
                    VertexAttribute {
                        offset: std::mem::size_of::<f32>() as u64 * 5,
                        shader_location: 2,
                        format: VertexFormat::Float32,
                    },
                ],
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

    // バッファサイズ: エンティティ数 * 6 floats (x, y, r, g, b, size)
    let max_entities = 100_000;
    let packet_buffer = device.create_buffer(&BufferDescriptor {
        label: Some("Entity Buffer"),
        size: (max_entities * 6 * std::mem::size_of::<f32>()) as u64,
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

// 与えられた座標データを使ってGPUでパケットを描画する関数
pub fn render_packets_gpu(coords: &[f32]) {
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
                                load: LoadOp::Clear(BG_COLOR),
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
pub fn render_frame_internal() {
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
                                load: LoadOp::Clear(BG_COLOR),
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

/// エンティティデータ形式: [x, y, r, g, b, size] の配列
/// ノードとパケットを一緒に描画
pub fn render_simulation_frame_internal(entity_data: &[f32]) {
    GPU_RENDERER.with(|renderer_ref| {
        let mut renderer_opt = renderer_ref.borrow_mut();
        if let Some(renderer) = renderer_opt.as_mut() {
            // エンティティ数を計算（6 floats per entity）
            let entity_count = entity_data.len() / 6;
            let entity_count = entity_count.min(MAX_PACKETS);

            // タイムユニフォームを更新
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

            // サーフェステクスチャを取得
            let surface_texture = match renderer.surface.get_current_texture() {
                Ok(texture) => texture,
                Err(_) => return,
            };

            let view = surface_texture
                .texture
                .create_view(&TextureViewDescriptor::default());

            // エンティティがある場合はバッファに書き込み
            if entity_count > 0 {
                let data_to_render = &entity_data[0..(entity_count * 6)];
                renderer.queue.write_buffer(
                    &renderer.packet_buffer,
                    0,
                    bytemuck::cast_slice(data_to_render),
                );
            }

            {
                let mut encoder =
                    renderer
                        .device
                        .create_command_encoder(&CommandEncoderDescriptor {
                            label: Some("Simulation Render Encoder"),
                        });

                {
                    let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                        label: Some("Simulation Render Pass"),
                        color_attachments: &[Some(RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: Operations {
                                load: LoadOp::Clear(BG_COLOR),
                                store: StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        occlusion_query_set: None,
                        timestamp_writes: None,
                    });

                    if entity_count > 0 {
                        render_pass.set_pipeline(&renderer.render_pipeline);
                        render_pass.set_bind_group(0, &renderer.time_bind_group, &[]);
                        let buffer_size = (entity_count * 6 * std::mem::size_of::<f32>()) as u64;
                        render_pass.set_vertex_buffer(0, renderer.packet_buffer.slice(0..buffer_size));
                        render_pass.draw(0..4, 0..entity_count as u32);
                    }
                }

                renderer.queue.submit(Some(encoder.finish()));
            }

            surface_texture.present();
            renderer.packet_count = entity_count as u32;
        }
    });
}
