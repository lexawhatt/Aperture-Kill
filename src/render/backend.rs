use std::error::Error;
use std::fmt;
use std::sync::Arc;

use winit::window::Window;

use super::plan::RenderPlan;
use super::sprites::{self, SpriteAsset, SpriteId, SpriteSize};
use super::{RenderScene, Renderer};

pub struct RenderBackend {
    inner: WgpuBackend,
}

impl RenderBackend {
    pub fn new(window: Arc<Window>) -> Result<Self, RenderBackendError> {
        WgpuBackend::new(window).map(|inner| Self { inner })
    }

    pub fn label(&self) -> &'static str {
        "wgpu"
    }

    pub fn render(
        &mut self,
        renderer: &Renderer,
        scene: RenderScene<'_>,
    ) -> Result<(), RenderBackendError> {
        let plan = RenderPlan::build(scene.world);

        self.inner.render(renderer, scene, &plan)
    }
}

struct WgpuBackend {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    frame_texture: wgpu::Texture,
    frame_texture_view: wgpu::TextureView,
    frame_bind_group: wgpu::BindGroup,
    frame_pixels: Vec<u32>,
    frame_size: (u32, u32),
    _sprite_resources: WgpuSpriteResources,
}

impl WgpuBackend {
    fn new(window: Arc<Window>) -> Result<Self, RenderBackendError> {
        let size = window.inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);
        let instance =
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
        let surface = instance.create_surface(window).map_err(|err| {
            RenderBackendError::new(format!("failed to create wgpu surface: {err}"))
        })?;
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
            apply_limit_buckets: false,
        }))
        .map_err(|err| RenderBackendError::new(format!("failed to request wgpu adapter: {err}")))?;
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("portals-wgpu-device"),
            ..Default::default()
        }))
        .map_err(|err| RenderBackendError::new(format!("failed to request wgpu device: {err}")))?;
        let config = surface
            .get_default_config(&adapter, width, height)
            .ok_or_else(|| RenderBackendError::new("wgpu surface is unsupported by adapter"))?;

        surface.configure(&device, &config);

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("portals-wgpu-frame-bind-group-layout"),
            entries: &[
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
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("portals-wgpu-frame-pipeline-layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::include_wgsl!("frame_blit.wgsl"));
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("portals-wgpu-frame-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("portals-wgpu-frame-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let (frame_texture, frame_texture_view, frame_bind_group) =
            Self::create_frame_texture(&device, &bind_group_layout, &sampler, width, height);
        let frame_pixels = vec![0; (width as usize).saturating_mul(height as usize)];
        let sprite_resources = WgpuSpriteResources::new(&device, &queue)?;

        Ok(Self {
            surface,
            device,
            queue,
            config,
            pipeline,
            bind_group_layout,
            sampler,
            frame_texture,
            frame_texture_view,
            frame_bind_group,
            frame_pixels,
            frame_size: (width, height),
            _sprite_resources: sprite_resources,
        })
    }

    fn render(
        &mut self,
        renderer: &Renderer,
        scene: RenderScene<'_>,
        plan: &RenderPlan,
    ) -> Result<(), RenderBackendError> {
        self.resize_if_needed(scene.width, scene.height);
        self.ensure_framebuffer(scene.width, scene.height);

        renderer.draw_scene_with_plan(&mut self.frame_pixels, scene, plan);
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.frame_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            pixels_as_bytes(&self.frame_pixels),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(self.frame_size.0 * 4),
                rows_per_image: Some(self.frame_size.1),
            },
            wgpu::Extent3d {
                width: self.frame_size.0,
                height: self.frame_size.1,
                depth_or_array_layers: 1,
            },
        );

        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.surface.configure(&self.device, &self.config);
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                return Err(RenderBackendError::new(
                    "wgpu validation error while acquiring frame",
                ));
            }
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("portals-wgpu-frame-encoder"),
            });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("portals-wgpu-frame-render-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.035,
                            g: 0.039,
                            b: 0.055,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.frame_bind_group, &[]);
            pass.draw(0..3, 0..1);
        }

        self.queue.submit(Some(encoder.finish()));
        self.queue.present(frame);

        Ok(())
    }

    fn create_frame_texture(
        device: &wgpu::Device,
        bind_group_layout: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
        width: u32,
        height: u32,
    ) -> (wgpu::Texture, wgpu::TextureView, wgpu::BindGroup) {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("portals-wgpu-frame-texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8Unorm,
            usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("portals-wgpu-frame-bind-group"),
            layout: bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        });

        (texture, texture_view, bind_group)
    }

    fn resize_if_needed(&mut self, width: u32, height: u32) {
        let width = width.max(1);
        let height = height.max(1);

        if self.config.width == width && self.config.height == height {
            return;
        }

        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
    }

    fn ensure_framebuffer(&mut self, width: u32, height: u32) {
        let width = width.max(1);
        let height = height.max(1);

        if self.frame_size == (width, height) {
            return;
        }

        let (texture, texture_view, bind_group) = Self::create_frame_texture(
            &self.device,
            &self.bind_group_layout,
            &self.sampler,
            width,
            height,
        );
        self.frame_texture = texture;
        self.frame_texture_view = texture_view;
        self.frame_bind_group = bind_group;
        self.frame_pixels
            .resize((width as usize).saturating_mul(height as usize), 0);
        self.frame_size = (width, height);
    }
}

struct WgpuSpriteResources {
    _bind_group_layout: wgpu::BindGroupLayout,
    _sampler: wgpu::Sampler,
    _textures: Vec<WgpuSpriteTexture>,
    _manifest_score: usize,
}

impl WgpuSpriteResources {
    fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Result<Self, RenderBackendError> {
        sprites::validate_manifest().map_err(RenderBackendError::new)?;

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("portals-wgpu-sprite-bind-group-layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("portals-wgpu-sprite-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let textures = sprites::gpu_ready_assets()
            .map(|asset| Self::create_texture(device, queue, &bind_group_layout, &sampler, asset))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            _bind_group_layout: bind_group_layout,
            _sampler: sampler,
            _textures: textures,
            _manifest_score: sprites::manifest_score(),
        })
    }

    fn create_texture(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bind_group_layout: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
        asset: SpriteAsset,
    ) -> Result<WgpuSpriteTexture, RenderBackendError> {
        let bytes = asset.raw_bytes().ok_or_else(|| {
            RenderBackendError::new(format!("sprite '{}' is not GPU-ready", asset.label()))
        })?;
        let size = asset.size();
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(asset.label()),
            size: wgpu::Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytes,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(size.width * 4),
                rows_per_image: Some(size.height),
            },
            wgpu::Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
        );
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(asset.label()),
            layout: bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        });

        Ok(WgpuSpriteTexture {
            _id: asset.id(),
            _size: size,
            _frame_count: asset.frames().len(),
            _texture: texture,
            _texture_view: texture_view,
            _bind_group: bind_group,
        })
    }
}

struct WgpuSpriteTexture {
    _id: SpriteId,
    _size: SpriteSize,
    _frame_count: usize,
    _texture: wgpu::Texture,
    _texture_view: wgpu::TextureView,
    _bind_group: wgpu::BindGroup,
}

#[cfg(target_endian = "little")]
fn pixels_as_bytes(pixels: &[u32]) -> &[u8] {
    let byte_len = std::mem::size_of_val(pixels);

    // The software renderer writes 0x00RRGGBB pixels. On little-endian hosts this
    // memory is BB GG RR 00, which matches `Bgra8Unorm`; the shader forces alpha.
    // SAFETY: `u8` may view any initialized memory, and the returned slice is tied
    // to the source slice lifetime.
    unsafe { std::slice::from_raw_parts(pixels.as_ptr().cast::<u8>(), byte_len) }
}

#[cfg(not(target_endian = "little"))]
compile_error!("WGPU framebuffer upload expects little-endian 0x00RRGGBB pixel memory.");

#[derive(Debug)]
pub struct RenderBackendError {
    message: String,
}

impl RenderBackendError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for RenderBackendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl Error for RenderBackendError {}
