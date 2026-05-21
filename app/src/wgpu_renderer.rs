use bytemuck::{Pod, Zeroable};
use leptos::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
use wgpu::util::DeviceExt;

use crate::canvas_grid::{Event, EventState, PolineManager, PolineManagerImpl};

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct SimParams {
    width: u32,
    height: u32,
    damping: f32,
    _pad: f32,
}

const MAX_DROPS: usize = 16;

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct DropsUniform {
    count: u32,
    _pad: [u32; 3],
    // Uniform-buffer array elements need 16-byte stride on WebGL/WGSL.
    // Store x/y in the first two lanes and leave z/w unused.
    coords: [[u32; 4]; MAX_DROPS],
}

impl Default for DropsUniform {
    fn default() -> Self {
        Self {
            count: 0,
            _pad: [0; 3],
            coords: [[0; 4]; MAX_DROPS],
        }
    }
}

pub struct WgpuLiquidRenderer<T> {
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,

    // Ping-pong simulation textures (Rg32Float)
    tex_a_view: wgpu::TextureView,
    tex_b_view: wgpu::TextureView,

    // Simulation pipeline + ping-pong bind groups
    sim_pipeline: wgpu::RenderPipeline,
    sim_bg_a: wgpu::BindGroup, // reads A, writes B
    sim_bg_b: wgpu::BindGroup, // reads B, writes A

    // Display pipeline + ping-pong bind groups
    display_pipeline: wgpu::RenderPipeline,
    display_bg_a: wgpu::BindGroup, // reads B (output of sim_bg_a)
    display_bg_b: wgpu::BindGroup, // reads A (output of sim_bg_b)

    // Uniform buffers
    drops_buf: wgpu::Buffer,

    // State
    frame_parity: bool,
    width: u32,
    height: u32,

    // Leptos integration
    events: ReadSignal<EventState>,
    clear_events: T,
    poline: Memo<PolineManagerImpl>,
    last_hue: f64,
    palette_texture: wgpu::Texture,
}

fn create_sim_texture(device: &wgpu::Device, width: u32, height: u32) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("sim_texture"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rg32Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    })
}

fn create_palette_texture(device: &wgpu::Device) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("palette_texture"),
        size: wgpu::Extent3d {
            width: 256,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba32Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    })
}

fn upload_palette(queue: &wgpu::Queue, texture: &wgpu::Texture, colors: &[[u8; 3]]) {
    let palette_data: Vec<[f32; 4]> = colors
        .iter()
        .map(|[r, g, b]| {
            [
                *r as f32 / 255.0,
                *g as f32 / 255.0,
                *b as f32 / 255.0,
                1.0,
            ]
        })
        .collect();

    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        bytemuck::cast_slice(&palette_data),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(256 * 16), // 256 * 4 floats * 4 bytes
            rows_per_image: Some(1),
        },
        wgpu::Extent3d {
            width: 256,
            height: 1,
            depth_or_array_layers: 1,
        },
    );
}

#[cfg(target_arch = "wasm32")]
fn ensure_gpu_canvas_context_constructor() {
    let global = js_sys::global();
    let name = wasm_bindgen::JsValue::from_str("GPUCanvasContext");

    if js_sys::Reflect::has(&global, &name).unwrap_or(false) {
        return;
    }

    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return;
    };

    let Ok(canvas) = document
        .create_element("canvas")
        .and_then(|el| el.dyn_into::<web_sys::HtmlCanvasElement>().map_err(Into::into))
    else {
        return;
    };

    let Ok(Some(context)) = canvas.get_context("webgpu") else {
        return;
    };

    let Ok(constructor) = js_sys::Reflect::get(&context, &wasm_bindgen::JsValue::from_str("constructor")) else {
        return;
    };

    _ = js_sys::Reflect::set(&global, &name, &constructor);
}

#[cfg(not(target_arch = "wasm32"))]
fn ensure_gpu_canvas_context_constructor() {}

impl<T> WgpuLiquidRenderer<T>
where
    T: Fn() + 'static,
{
    pub async fn new(
        canvas: web_sys::HtmlCanvasElement,
        width: u32,
        height: u32,
        events: ReadSignal<EventState>,
        clear_events: T,
        poline: Memo<PolineManagerImpl>,
    ) -> Option<Self> {
        ensure_gpu_canvas_context_constructor();

        let instance = wgpu::util::new_instance_with_webgpu_detection(&wgpu::InstanceDescriptor {
            // Chrome on Linux and some Firefox configurations expose `navigator.gpu` but still
            // return `null` from `requestAdapter()`. Let wgpu detect that case and fall back to
            // WebGL2 rather than constructing a WebGPU-only instance that can never get an adapter.
            backends: wgpu::Backends::BROWSER_WEBGPU | wgpu::Backends::GL,
            ..Default::default()
        })
        .await;

        let surface = match instance.create_surface(wgpu::SurfaceTarget::Canvas(canvas)) {
            Ok(surface) => surface,
            Err(err) => {
                log::error!("failed to create WebGPU surface: {err:?}");
                return None;
            }
        };

        let mut adapter = None;
        for power_preference in [
            wgpu::PowerPreference::HighPerformance,
            wgpu::PowerPreference::LowPower,
            wgpu::PowerPreference::None,
        ] {
            adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference,
                    compatible_surface: Some(&surface),
                    force_fallback_adapter: false,
                })
                .await;

            if adapter.is_some() {
                break;
            }
        }

        let Some(adapter) = adapter else {
            log::error!(
                "failed to request WebGPU adapter; navigator.gpu is present but no adapter was available"
            );
            return None;
        };

        let (device, queue) = match adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("liquid_device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                        .using_resolution(adapter.limits()),
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None,
            )
            .await
        {
            Ok(device_queue) => device_queue,
            Err(err) => {
                log::error!("failed to request WebGPU device: {err:?}");
                return None;
            }
        };

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width,
            height,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &surface_config);

        // Create simulation textures
        let tex_a = create_sim_texture(&device, width, height);
        let tex_b = create_sim_texture(&device, width, height);
        let tex_a_view = tex_a.create_view(&Default::default());
        let tex_b_view = tex_b.create_view(&Default::default());

        // Create palette texture
        let palette_texture = create_palette_texture(&device);
        let palette_view = palette_texture.create_view(&Default::default());

        // Upload initial palette
        let initial_hue = {
            let p = poline.read_untracked();
            upload_palette(&queue, &palette_texture, p.colors());
            *p.abs_hue()
        };

        // Uniform buffers
        let sim_params = SimParams {
            width,
            height,
            damping: 0.99,
            _pad: 0.0,
        };
        let sim_params_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("sim_params"),
            contents: bytemuck::bytes_of(&sim_params),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let drops_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("drops"),
            contents: bytemuck::bytes_of(&DropsUniform::default()),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Simulation pipeline
        let sim_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("simulate"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/simulate.wgsl").into()),
        });

        let sim_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("sim_bind_group_layout"),
                entries: &[
                    // input texture
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // sim params uniform
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // drops uniform
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let sim_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sim_pipeline_layout"),
            bind_group_layouts: &[&sim_bind_group_layout],
            push_constant_ranges: &[],
        });

        let sim_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("sim_pipeline"),
            layout: Some(&sim_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &sim_shader,
                entry_point: Some("vs"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &sim_shader,
                entry_point: Some("fs"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rg32Float,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // sim bind groups: A reads tex_a (writes to tex_b), B reads tex_b (writes to tex_a)
        let sim_bg_a = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sim_bg_a"),
            layout: &sim_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&tex_a_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: sim_params_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: drops_buf.as_entire_binding(),
                },
            ],
        });
        let sim_bg_b = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sim_bg_b"),
            layout: &sim_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&tex_b_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: sim_params_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: drops_buf.as_entire_binding(),
                },
            ],
        });

        // Display pipeline
        let display_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("display"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/display.wgsl").into()),
        });

        let display_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("display_bind_group_layout"),
                entries: &[
                    // sim output texture
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // palette texture
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                ],
            });

        let display_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("display_pipeline_layout"),
                bind_group_layouts: &[&display_bind_group_layout],
                push_constant_ranges: &[],
            });

        let display_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("display_pipeline"),
            layout: Some(&display_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &display_shader,
                entry_point: Some("vs"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &display_shader,
                entry_point: Some("fs"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // display bind groups: A reads from tex_b (output when sim uses bg_a), B reads from tex_a
        let display_bg_a = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("display_bg_a"),
            layout: &display_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&tex_b_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&palette_view),
                },
            ],
        });
        let display_bg_b = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("display_bg_b"),
            layout: &display_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&tex_a_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&palette_view),
                },
            ],
        });

        Some(Self {
            device,
            queue,
            surface,
            surface_config,
            tex_a_view,
            tex_b_view,
            sim_pipeline,
            sim_bg_a,
            sim_bg_b,
            display_pipeline,
            display_bg_a,
            display_bg_b,
            drops_buf,
            frame_parity: false,
            width,
            height,
            events,
            clear_events,
            poline,
            last_hue: initial_hue,
            palette_texture,
        })
    }

    fn collect_drops(&self) -> DropsUniform {
        let mut drops = DropsUniform::default();
        self.events.with_untracked(|state| {
            for ev in &state.events {
                if drops.count as usize >= MAX_DROPS {
                    break;
                }
                match ev {
                    Event::AddDrop { coord } => {
                        let idx = drops.count as usize;
                        drops.coords[idx] = [coord.x as u32, coord.y as u32, 0, 0];
                        drops.count += 1;
                    }
                }
            }
        });
        drops
    }

    fn update_palette_if_needed(&mut self) {
        let p = self.poline.read_untracked();
        let current_hue = *p.abs_hue();
        if current_hue != self.last_hue {
            upload_palette(&self.queue, &self.palette_texture, p.colors());
            self.last_hue = current_hue;
        }
    }

    pub fn draw(&mut self) -> Result<(), ()> {
        // Check for cancel
        let cancelled = self
            .events
            .with_untracked(|state| state.cancel);
        if cancelled {
            return Err(());
        }

        // Collect drops and clear events
        let drops = self.collect_drops();
        (self.clear_events)();

        // Upload drops
        self.queue
            .write_buffer(&self.drops_buf, 0, bytemuck::bytes_of(&drops));

        // Update palette if hue changed
        self.update_palette_if_needed();

        // Get surface texture
        let output = self.surface.get_current_texture().map_err(|_| ())?;
        let surface_view = output.texture.create_view(&Default::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame_encoder"),
            });

        // Select bind groups based on frame parity
        let (sim_bg, sim_target, display_bg) = if self.frame_parity {
            (&self.sim_bg_b, &self.tex_a_view, &self.display_bg_b)
        } else {
            (&self.sim_bg_a, &self.tex_b_view, &self.display_bg_a)
        };

        // Pass 1: Simulation
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("sim_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: sim_target,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            rpass.set_pipeline(&self.sim_pipeline);
            rpass.set_bind_group(0, sim_bg, &[]);
            rpass.draw(0..3, 0..1);
        }

        // Pass 2: Display (color mapping)
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("display_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            rpass.set_pipeline(&self.display_pipeline);
            rpass.set_bind_group(0, display_bg, &[]);
            rpass.draw(0..3, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        self.frame_parity = !self.frame_parity;
        Ok(())
    }
}
