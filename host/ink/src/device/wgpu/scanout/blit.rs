//! One copy from Ink's logical image to the panel, including its quarter turn.
use crate::Rotation;
use ::wgpu as gpu;

pub(super) struct Blit {
    pipeline: gpu::RenderPipeline,
    layout: gpu::BindGroupLayout,
}

impl Blit {
    pub(super) fn new(device: &gpu::Device, format: gpu::TextureFormat) -> Self {
        let layout = device.create_bind_group_layout(&gpu::BindGroupLayoutDescriptor {
            label: Some("Ink panel copy"),
            entries: &[gpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: gpu::ShaderStages::FRAGMENT,
                ty: gpu::BindingType::Texture {
                    sample_type: gpu::TextureSampleType::Float { filterable: false },
                    view_dimension: gpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&gpu::PipelineLayoutDescriptor {
            label: Some("Ink panel copy"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(gpu::include_wgsl!("blit.wgsl"));
        let pipeline = device.create_render_pipeline(&gpu::RenderPipelineDescriptor {
            label: Some("Ink panel copy"),
            layout: Some(&pipeline_layout),
            vertex: gpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(gpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(gpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: gpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        Self { pipeline, layout }
    }

    pub(super) fn copy(
        &self,
        device: &gpu::Device,
        encoder: &mut gpu::CommandEncoder,
        source: &gpu::TextureView,
        target: &gpu::TextureView,
        rotation: Rotation,
    ) {
        let group = device.create_bind_group(&gpu::BindGroupDescriptor {
            label: Some("Ink panel copy"),
            layout: &self.layout,
            entries: &[gpu::BindGroupEntry {
                binding: 0,
                resource: gpu::BindingResource::TextureView(source),
            }],
        });
        let mut pass = encoder.begin_render_pass(&gpu::RenderPassDescriptor {
            label: Some("Ink panel copy"),
            color_attachments: &[Some(gpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: gpu::Operations {
                    // The triangle overwrites every pixel of the acquired image.
                    load: gpu::LoadOp::Clear(gpu::Color::BLACK),
                    store: gpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &group, &[]);
        let turn = u32::from(rotation.degrees() / 90);
        pass.draw(0..3, turn..turn + 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::{Device, TextureDesc};

    fn readback(device: &gpu::Device, queue: &gpu::Queue, texture: &gpu::Texture) -> Vec<u8> {
        let row = texture.width() * 4;
        let stride =
            row.div_ceil(gpu::COPY_BYTES_PER_ROW_ALIGNMENT) * gpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let buffer = device.create_buffer(&gpu::BufferDescriptor {
            label: Some("panel copy readback"),
            size: u64::from(stride) * u64::from(texture.height()),
            usage: gpu::BufferUsages::COPY_DST | gpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            texture.as_image_copy(),
            gpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: gpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(stride),
                    rows_per_image: None,
                },
            },
            texture.size(),
        );
        queue.submit([encoder.finish()]);
        let (send, receive) = std::sync::mpsc::channel();
        buffer.slice(..).map_async(gpu::MapMode::Read, move |r| {
            send.send(r).unwrap();
        });
        device.poll(gpu::PollType::wait_indefinitely()).unwrap();
        receive.recv().unwrap().unwrap();
        let mut pixels: Vec<u8> = buffer
            .slice(..)
            .get_mapped_range()
            .unwrap()
            .chunks(stride as usize)
            .flat_map(|r| &r[..row as usize])
            .copied()
            .collect();
        if texture.format() == gpu::TextureFormat::Bgra8Unorm {
            for pixel in pixels.as_chunks_mut::<4>().0 {
                pixel.swap(0, 2);
            }
        }
        pixels
    }

    #[test]
    fn quarter_turn_copy_matches_rotation_then_nearest_scaling() {
        let mut ink = crate::Wgpu::new().expect("GPU for panel copy pixels");
        let source = ink.create_texture(TextureDesc::rgba(3, 2)).unwrap();
        let pixels: Vec<u8> = [
            [240, 12, 60, 255],
            [16, 80, 32, 128],
            [0, 0, 0, 0],
            [5, 10, 20, 64],
            [30, 210, 90, 255],
            [60, 20, 100, 160],
        ]
        .into_iter()
        .flatten()
        .collect();
        ink.upload_texture(source, [0, 0], [3, 2], &pixels).unwrap();
        let source_view = ink.scanout_view(source).unwrap();
        for format in [
            gpu::TextureFormat::Rgba8Unorm,
            gpu::TextureFormat::Bgra8Unorm,
        ] {
            let copy = Blit::new(&ink.device, format);
            let previous = gpu::util::TextureBlitter::new(&ink.device, format);
            for (rotation, order) in [
                (Rotation::R0, [0, 1, 2, 3, 4, 5]),
                (Rotation::R90, [3, 0, 4, 1, 5, 2]),
                (Rotation::R180, [5, 4, 3, 2, 1, 0]),
                (Rotation::R270, [2, 5, 1, 4, 0, 3]),
            ] {
                let turned_size = rotation.size([3, 2]);
                let turned = if rotation == Rotation::R0 {
                    source
                } else {
                    let target = ink.create_target(turned_size[0], turned_size[1]).unwrap();
                    rotation.blit(&mut ink, source, [3, 2], target).unwrap();
                    target
                };
                let turned_view = ink.scanout_view(turned).unwrap();
                // Odd sizes put sample centres on texel boundaries after a flip.
                for size in [turned_size, [7, 5], [2, 1], [1, 4]] {
                    let target = || {
                        ink.device.create_texture(&gpu::TextureDescriptor {
                            label: Some("panel copy fixture"),
                            size: gpu::Extent3d {
                                width: size[0],
                                height: size[1],
                                depth_or_array_layers: 1,
                            },
                            mip_level_count: 1,
                            sample_count: 1,
                            dimension: gpu::TextureDimension::D2,
                            format,
                            usage: gpu::TextureUsages::RENDER_ATTACHMENT
                                | gpu::TextureUsages::COPY_DST
                                | gpu::TextureUsages::COPY_SRC,
                            view_formats: &[],
                        })
                    };
                    let (actual, expected) = (target(), target());
                    // A complete copy must replace old pixels even where the source is transparent.
                    for (texture, color) in [(&actual, [9, 17, 25, 64]), (&expected, [255; 4])] {
                        ink.queue.write_texture(
                            texture.as_image_copy(),
                            &color.repeat((size[0] * size[1]) as usize),
                            gpu::TexelCopyBufferLayout {
                                offset: 0,
                                bytes_per_row: Some(size[0] * 4),
                                rows_per_image: None,
                            },
                            texture.size(),
                        );
                    }
                    let mut encoder = ink.device.create_command_encoder(&Default::default());
                    copy.copy(
                        &ink.device,
                        &mut encoder,
                        &source_view,
                        &actual.create_view(&Default::default()),
                        rotation,
                    );
                    previous.copy(
                        &ink.device,
                        &mut encoder,
                        &turned_view,
                        &expected.create_view(&Default::default()),
                    );
                    ink.queue.submit([encoder.finish()]);
                    let actual = readback(&ink.device, &ink.queue, &actual);
                    assert_eq!(
                        actual,
                        readback(&ink.device, &ink.queue, &expected),
                        "{rotation:?}, {format:?}, {size:?}"
                    );
                    if size == turned_size {
                        let expected: Vec<_> = order
                            .into_iter()
                            .flat_map(|i| pixels[i * 4..i * 4 + 4].iter().copied())
                            .collect();
                        assert_eq!(actual, expected, "unscaled {rotation:?}, {format:?}");
                    }
                }
                if turned != source {
                    ink.retire_texture(turned).unwrap();
                }
            }
        }
    }
}
