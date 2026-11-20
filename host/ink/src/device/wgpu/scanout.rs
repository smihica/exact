//! Direct KMS scanout through wgpu's DRM surface (VK_KHR_display): the compose
//! target reaches the screen with one GPU copy, no readback. That copy also
//! turns the frame for a rotated panel.
use super::*;
use crate::Rotation;

pub(super) struct Scanout {
    surface: ::wgpu::Surface<'static>,
    config: ::wgpu::SurfaceConfiguration,
    pipeline: ::wgpu::RenderPipeline,
    layout: ::wgpu::BindGroupLayout,
}

const COPY: &str = "
@group(0) @binding(0) var frame: texture_2d<f32>;
struct Triangle {
    @builtin(position) position: vec4<f32>,
    @location(0) @interpolate(flat) turn: u32,
}
@vertex fn vs_main(@builtin(vertex_index) i: u32, @builtin(instance_index) turn: u32) -> Triangle {
    let position = vec4<f32>(f32(i32(i & 1u) * 4 - 1), f32(i32(i >> 1u) * 4 - 1), 0.0, 1.0);
    return Triangle(position, turn);
}
@fragment fn fs_main(v: Triangle) -> @location(0) vec4<f32> {
    let at = vec2<i32>(v.position.xy);
    let last = vec2<i32>(textureDimensions(frame)) - vec2<i32>(1);
    var src = at;
    switch v.turn {
        case 1u: { src = vec2<i32>(at.y, last.y - at.x); }
        case 2u: { src = last - at; }
        case 3u: { src = vec2<i32>(last.x - at.y, at.x); }
        default: {}
    }
    return textureLoad(frame, src, 0);
}
";

fn layout(device: &::wgpu::Device) -> ::wgpu::BindGroupLayout {
    device.create_bind_group_layout(&::wgpu::BindGroupLayoutDescriptor {
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
    })
}

fn pipeline(
    device: &::wgpu::Device,
    layout: &::wgpu::BindGroupLayout,
    format: ::wgpu::TextureFormat,
) -> ::wgpu::RenderPipeline {
    let pipeline_layout = device.create_pipeline_layout(&::wgpu::PipelineLayoutDescriptor {
        label: Some("ink scanout"),
        bind_group_layouts: &[Some(layout)],
        immediate_size: 0,
    });
    let shader = device.create_shader_module(::wgpu::ShaderModuleDescriptor {
        label: Some("ink scanout"),
        source: ::wgpu::ShaderSource::Wgsl(COPY.into()),
    });
    device.create_render_pipeline(&::wgpu::RenderPipelineDescriptor {
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
}

/// Draw `frame` over the whole of `panel`.
fn copy(
    device: &::wgpu::Device,
    encoder: &mut ::wgpu::CommandEncoder,
    pipeline: &::wgpu::RenderPipeline,
    rotation: Rotation,
    layout: &::wgpu::BindGroupLayout,
    frame: &::wgpu::TextureView,
    panel: &::wgpu::TextureView,
) {
    let group = device.create_bind_group(&::wgpu::BindGroupDescriptor {
        label: Some("ink scanout"),
        layout,
        entries: &[::wgpu::BindGroupEntry {
            binding: 0,
            resource: ::wgpu::BindingResource::TextureView(frame),
        }],
    });
    let mut pass = encoder.begin_render_pass(&::wgpu::RenderPassDescriptor {
        label: Some("ink scanout"),
        color_attachments: &[Some(::wgpu::RenderPassColorAttachment {
            view: panel,
            depth_slice: None,
            resolve_target: None,
            ops: ::wgpu::Operations {
                load: ::wgpu::LoadOp::Clear(::wgpu::Color::BLACK),
                store: ::wgpu::StoreOp::Store,
            },
        })],
        ..Default::default()
    });
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, &group, &[]);
    // The instance index carries the quarter turn without a per-frame buffer write.
    let turn = rotation as u32;
    pass.draw(0..3, turn..turn + 1);
}

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
            // Let the next frame render while the display scans out its predecessor.
            desired_maximum_frame_latency: 2,
            alpha_mode: ::wgpu::CompositeAlphaMode::Opaque,
            view_formats: Vec::new(),
        };
        let layout = layout(&self.device);
        let pipeline = checked(&self.device, &self.errors, || {
            surface.configure(&self.device, &config);
            pipeline(&self.device, &layout, format)
        })?;
        // On Mesa's direct display the last picture may wait for another present until
        // the first blocking acquire starts the flip-event reader. Cycle one frame
        // past the requested swapchain depth before handing the surface to the painter.
        for _ in 0..config.desired_maximum_frame_latency + 2 {
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

    /// Queue the texture for the next refresh, turned onto the panel by
    /// `rotation`. Blocks while the display holds every image, which paces
    /// the caller to the refresh.
    pub fn scan_out(&mut self, texture: TextureId, rotation: Rotation) -> Result<()> {
        let scanout = self.scanout.as_ref().ok_or(Error::Unsupported("scanout"))?;
        let frame = &self.texture(texture)?.view;
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
        copy(
            &self.device,
            &mut encoder,
            &scanout.pipeline,
            rotation,
            &scanout.layout,
            frame,
            &view,
        );
        let submission = self.submit(encoder.finish())?;
        self.textures.get_mut(&texture).unwrap().last = submission;
        self.queue.present(output);
        failure(&self.errors)
    }
}

#[cfg(test)]
#[test]
fn the_copy_turns_the_frame_as_the_blit_does_and_a_touch_lands_on_the_pixel_shown() {
    let mut d = Wgpu::new().expect("GPU for scanout pixels");
    let frame = d.create_texture(TextureDesc::rgba(3, 2)).unwrap();
    let colors: Vec<u8> = (1..=6).flat_map(|v| [v * 20, 0, 0, 255]).collect();
    d.upload_texture(frame, [0, 0], [3, 2], &colors).unwrap();
    let layout = layout(&d.device);
    let pipeline = checked(&d.device, &d.errors, || {
        pipeline(&d.device, &layout, ::wgpu::TextureFormat::Rgba8Unorm)
    })
    .unwrap();
    for rotation in [Rotation::R0, Rotation::R90, Rotation::R180, Rotation::R270] {
        let [w, h] = rotation.size([3, 2]);
        let blitted = d.create_target(w, h).unwrap();
        rotation.blit(&mut d, frame, [3, 2], blitted).unwrap();
        let panel = d.create_target(w, h).unwrap();
        let mut encoder = d.device.create_command_encoder(&Default::default());
        copy(
            &d.device,
            &mut encoder,
            &pipeline,
            rotation,
            &layout,
            &d.textures[&frame].view,
            &d.textures[&panel].view,
        );
        d.submit(encoder.finish()).unwrap();
        let shown = d.readback(panel).unwrap();
        assert_eq!(shown, d.readback(blitted).unwrap(), "{rotation:?}");
        for (i, pixel) in shown.as_chunks::<4>().0.iter().enumerate() {
            let [x, y] = [i as u32 % w, i as u32 / w].map(|v| v as f32 + 0.5);
            let (x, y) = rotation.input(Some(x / w as f32), Some(y / h as f32));
            let touched = (y.unwrap() * 2.) as usize * 3 + (x.unwrap() * 3.) as usize;
            assert_eq!(pixel[..], colors[touched * 4..][..4], "{rotation:?} {i}");
        }
    }
}
