//! Direct KMS scanout through wgpu's DRM surface (VK_KHR_display): the compose
//! target reaches the screen with one GPU copy, no readback.
use super::*;

pub(super) struct Scanout {
    surface: ::wgpu::Surface<'static>,
    config: ::wgpu::SurfaceConfiguration,
    pipeline: ::wgpu::RenderPipeline,
    layout: ::wgpu::BindGroupLayout,
}

const COPY: &str = "
@group(0) @binding(0) var frame: texture_2d<f32>;
@vertex fn vs_main(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    return vec4<f32>(f32(i32(i & 1u) * 4 - 1), f32(i32(i >> 1u) * 4 - 1), 0.0, 1.0);
}
@fragment fn fs_main(@builtin(position) p: vec4<f32>) -> @location(0) vec4<f32> {
    let at = vec2<i32>(p.xy);
    return textureLoad(frame, AT, 0);
}
";
// A panel mounted sideways shows the logical portrait frame turned clockwise.
const TURNED: &str = "vec2<i32>(at.y, i32(textureDimensions(frame).y) - 1 - at.x)";

impl Wgpu {
    /// Take the connector's display for presentation. `size` and `refresh`
    /// (millihertz) name one of its modes.
    ///
    /// # Safety
    /// `fd` is the DRM master of the card that scans the connector out and
    /// outlives the device; nothing else reads its events or flips its planes
    /// afterwards, also when this fails: Vulkan may already drive the display.
    pub unsafe fn attach_scanout(
        &mut self,
        fd: i32,
        connector: u32,
        size: [u32; 2],
        refresh: u32,
        turned: bool,
    ) -> Result<()> {
        // SAFETY: the caller's contract above.
        let surface = unsafe {
            self.instance
                .create_surface_unsafe(::wgpu::SurfaceTargetUnsafe::Drm {
                    fd,
                    plane: 0,
                    connector_id: connector,
                    width: size[0],
                    height: size[1],
                    refresh_rate: refresh,
                })
        }
        .map_err(|e| Error::Device(e.to_string()))?;
        let capabilities = surface.get_capabilities(&self.adapter);
        // The dumb-buffer path shows the target's bytes as they are.
        let format = capabilities
            .formats
            .iter()
            .copied()
            .find(|format| !format.is_srgb())
            .ok_or(Error::Unsupported("a linear scanout format"))?;
        let config = ::wgpu::SurfaceConfiguration {
            usage: ::wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            color_space: Default::default(),
            width: size[0],
            height: size[1],
            present_mode: ::wgpu::PresentMode::Fifo,
            // Two images: an acquire waits for the previous flip, so input shows a frame sooner.
            desired_maximum_frame_latency: 1,
            alpha_mode: ::wgpu::CompositeAlphaMode::Opaque,
            view_formats: Vec::new(),
        };
        let layout = self
            .device
            .create_bind_group_layout(&::wgpu::BindGroupLayoutDescriptor {
                label: Some("ink scanout"),
                entries: &[::wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ::wgpu::ShaderStages::FRAGMENT,
                    ty: ::wgpu::BindingType::Texture {
                        sample_type: ::wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: ::wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                }],
            });
        let pipeline = checked(&self.device, &self.errors, || {
            surface.configure(&self.device, &config);
            let shader = self
                .device
                .create_shader_module(::wgpu::ShaderModuleDescriptor {
                    label: Some("ink scanout"),
                    source: ::wgpu::ShaderSource::Wgsl(
                        COPY.replace("AT", if turned { TURNED } else { "at" })
                            .into(),
                    ),
                });
            let pipeline_layout =
                self.device
                    .create_pipeline_layout(&::wgpu::PipelineLayoutDescriptor {
                        label: Some("ink scanout"),
                        bind_group_layouts: &[Some(&layout)],
                        immediate_size: 0,
                    });
            self.device
                .create_render_pipeline(&::wgpu::RenderPipelineDescriptor {
                    label: Some("ink scanout"),
                    layout: Some(&pipeline_layout),
                    vertex: ::wgpu::VertexState {
                        module: &shader,
                        entry_point: Some("vs_main"),
                        compilation_options: Default::default(),
                        buffers: &[],
                    },
                    primitive: Default::default(),
                    depth_stencil: None,
                    multisample: Default::default(),
                    fragment: Some(::wgpu::FragmentState {
                        module: &shader,
                        entry_point: Some("fs_main"),
                        compilation_options: Default::default(),
                        targets: &[Some(format.into())],
                    }),
                    multiview_mask: None,
                    cache: None,
                })
        })?;
        // Seen on Mesa 26.2.6 (v3dv over vc4): the last frame presented before
        // the screen rests stayed off it until the next present. Mesa starts
        // reading the display's flip events only once an acquire has had to
        // wait, and a third present on two images makes that happen here; every
        // later frame then appeared.
        for _ in 0..3 {
            match surface.get_current_texture() {
                ::wgpu::CurrentSurfaceTexture::Success(t)
                | ::wgpu::CurrentSurfaceTexture::Suboptimal(t) => self.queue.present(t),
                other => return Err(Error::Device(format!("acquire scanout: {other:?}"))),
            }
        }
        failure(&self.errors)?;
        self.scanout = Some(Scanout {
            surface,
            config,
            pipeline,
            layout,
        });
        Ok(())
    }

    pub fn has_scanout(&self) -> bool {
        self.scanout.is_some()
    }

    /// Queue the texture for the next refresh. Blocks while the display holds
    /// every image, which paces the caller to the refresh.
    pub fn scan_out(&mut self, texture: TextureId) -> Result<()> {
        let scanout = self.scanout.as_ref().ok_or(Error::Unsupported("scanout"))?;
        let group = self.device.create_bind_group(&::wgpu::BindGroupDescriptor {
            label: Some("ink scanout"),
            layout: &scanout.layout,
            entries: &[::wgpu::BindGroupEntry {
                binding: 0,
                resource: ::wgpu::BindingResource::TextureView(&self.texture(texture)?.view),
            }],
        });
        let mut acquired = scanout.surface.get_current_texture();
        if matches!(
            acquired,
            ::wgpu::CurrentSurfaceTexture::Outdated | ::wgpu::CurrentSurfaceTexture::Lost
        ) {
            scanout.surface.configure(&self.device, &scanout.config);
            acquired = scanout.surface.get_current_texture();
        }
        let output = match acquired {
            ::wgpu::CurrentSurfaceTexture::Success(t)
            | ::wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
            other => return Err(Error::Device(format!("acquire scanout: {other:?}"))),
        };
        let view = output.texture.create_view(&Default::default());
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&::wgpu::RenderPassDescriptor {
                label: Some("ink scanout"),
                color_attachments: &[Some(::wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: ::wgpu::Operations {
                        load: ::wgpu::LoadOp::Clear(::wgpu::Color::BLACK),
                        store: ::wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pass.set_pipeline(&scanout.pipeline);
            pass.set_bind_group(0, &group, &[]);
            pass.draw(0..3, 0..1);
        }
        let submission = self.submit(encoder.finish())?;
        self.textures.get_mut(&texture).unwrap().last = submission;
        self.queue.present(output);
        failure(&self.errors)
    }
}
