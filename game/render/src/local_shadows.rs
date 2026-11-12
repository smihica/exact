//! Shadow maps for local lights: one depth layer per shadowed spot light and six
//! (a cube's faces) per shadowed point light, nearest lights first, at most
//! `MAX_VIEWS` layers a frame. Each layer culls its casters on the GPU, four
//! views per cull pass as the camera and cascades are, and draws them depth-only.
use crate::LightInput;
use exact_gpu::wgpu;
use glam::camera::rh::proj::directx;
use glam::{Mat4, Vec3};

/// Shadow layers a frame: a shadowed point light takes six, a spot one.
pub(crate) const MAX_VIEWS: usize = 8;
/// Bytes the layers may take: eight at 2048², two at 4096².
const BUDGET: u64 = 128 << 20;
// Spots wider than this shadow only their central cone.
const MAX_FOV: f32 = 170. * std::f32::consts::PI / 180.;

/// This frame's local shadow views and the layer each light starts at.
#[derive(Default)]
pub(crate) struct Plan {
    pub views: Vec<Mat4>,
    /// Per frame light: first layer, and a shadow texel's width per metre of
    /// distance (the normal offset's scale).
    pub layers: Vec<Option<(u32, f32)>>,
    /// Width and height of every layer.
    pub resolution: u32,
}

impl Plan {
    pub fn of(&mut self, lights: &[LightInput], settings: &crate::LocalShadows) {
        self.views.clear();
        self.layers.clear();
        let resolution = settings.resolution.clamp(128, 4096);
        self.resolution = resolution;
        let layer = 4 * u64::from(resolution) * u64::from(resolution);
        let max_views = MAX_VIEWS.min((BUDGET / layer) as usize);
        for light in lights.iter().take(crate::MAX_LIGHTS) {
            let need = if light.cone.is_some() { 1 } else { 6 };
            if !light.shadows
                || light.intensity.is_nan()
                || light.intensity <= 0.
                || light.range.is_nan()
                || light.range <= 0.
                || !light.position.is_finite()
                || self.views.len() + need > max_views
            {
                self.layers.push(None);
                continue;
            }
            let near = (light.range * 1e-3).clamp(0.01, 0.05);
            let first = self.views.len() as u32;
            let texel = match light.cone {
                Some([_, outer]) => {
                    let fov = (2. * outer.clamp(-1., 1.).acos()).clamp(0.01, MAX_FOV);
                    let direction = light.direction.normalize_or(-Vec3::Z);
                    let up = if direction.y.abs() > 0.99 {
                        Vec3::Z
                    } else {
                        Vec3::Y
                    };
                    self.views.push(
                        directx::perspective(fov, 1., near, light.range)
                            * glam::camera::rh::view::look_to_mat4(light.position, direction, up),
                    );
                    2. * (fov * 0.5).tan() / resolution as f32
                }
                None => {
                    let projection =
                        directx::perspective(std::f32::consts::FRAC_PI_2, 1., near, light.range);
                    // Face order +X −X +Y −Y +Z −Z: lights.wgsl picks by major axis.
                    for (axis, up) in [
                        (Vec3::X, -Vec3::Y),
                        (-Vec3::X, -Vec3::Y),
                        (Vec3::Y, Vec3::Z),
                        (-Vec3::Y, -Vec3::Z),
                        (Vec3::Z, -Vec3::Y),
                        (-Vec3::Z, -Vec3::Y),
                    ] {
                        self.views.push(
                            projection
                                * glam::camera::rh::view::look_to_mat4(light.position, axis, up),
                        );
                    }
                    2. / resolution as f32
                }
            };
            self.layers.push(Some((first, texel)));
        }
    }
}

/// The depth layers, a camera binding per layer, and the culls that list each
/// layer's casters (`cull.rs`, four views each).
pub(crate) struct LocalMaps {
    pub count: u32,
    pub resolution: u32,
    pub layers: Vec<wgpu::TextureView>,
    pub sample: wgpu::TextureView,
    pub cameras: Vec<wgpu::BindGroup>,
    uniform: wgpu::Buffer,
}

impl LocalMaps {
    pub fn new(
        device: &wgpu::Device,
        count: u32,
        resolution: u32,
        camera_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("game local shadows"),
            size: wgpu::Extent3d {
                width: resolution,
                height: resolution,
                depth_or_array_layers: count,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let sample = texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });
        let layers = (0..count)
            .map(|i| {
                texture.create_view(&wgpu::TextureViewDescriptor {
                    dimension: Some(wgpu::TextureViewDimension::D2),
                    base_array_layer: i,
                    array_layer_count: Some(1),
                    ..Default::default()
                })
            })
            .collect();
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("game local shadow cameras"),
            size: u64::from(count) * 256,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let cameras = (0..count)
            .map(|i| {
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("game local shadow camera"),
                    layout: camera_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: &uniform,
                            offset: u64::from(i) * 256,
                            size: std::num::NonZeroU64::new(64),
                        }),
                    }],
                })
            })
            .collect();
        Self {
            count,
            resolution,
            layers,
            sample,
            cameras,
            uniform,
        }
    }

    pub fn write(&self, queue: &wgpu::Queue, views: &[Mat4]) {
        let mut data = vec![0f32; self.count as usize * 64];
        for (i, view) in views.iter().enumerate() {
            data[i * 64..i * 64 + 16].copy_from_slice(&view.to_cols_array());
        }
        queue.write_buffer(&self.uniform, 0, crate::buffers::bytes(&data));
    }
}

/// A one-texel depth array standing in for absent shadow maps in group 1.
pub(crate) fn placeholder(device: &wgpu::Device) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("game no shadow map"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
        .create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        })
}

#[cfg(test)]
mod tests {
    use super::Plan;
    use crate::{LightInput, LocalShadows};
    use glam::Vec3;

    fn spot(x: f32) -> LightInput {
        LightInput {
            position: Vec3::new(x, 4., 0.),
            color: Vec3::ONE,
            intensity: 1.,
            range: 10.,
            direction: -Vec3::Y,
            cone: Some([0.99, 0.95]),
            shadows: true,
            ..Default::default()
        }
    }

    #[test]
    fn the_local_resolution_sizes_the_layers_within_the_budget() {
        let lights = [spot(0.), spot(1.), spot(2.)];
        let mut plan = Plan::default();
        let of = |plan: &mut Plan, resolution| {
            let settings = LocalShadows {
                resolution,
                ..Default::default()
            };
            plan.of(&lights, &settings);
            (
                plan.resolution,
                plan.views.len(),
                plan.layers[0].map(|(_, texel)| texel),
            )
        };
        let (resolution, views, texel) = of(&mut plan, 1024);
        assert_eq!((resolution, views), (1024, 3));
        let (_, _, half) = of(&mut plan, 2048);
        assert_eq!(
            half.unwrap() * 2.,
            texel.unwrap(),
            "a texel per metre halves"
        );
        assert_eq!(of(&mut plan, 9000).0, 4096);
        assert_eq!(of(&mut plan, 16).0, 128);
        // 64 MiB a layer at 4096²: two fit, the nearest lights first.
        let (_, views, _) = of(&mut plan, 4096);
        assert_eq!(views, 2);
        assert!(plan.layers[2].is_none());
        // A point light's six layers do not fit at 4096².
        let point = [LightInput {
            cone: None,
            ..spot(0.)
        }];
        plan.of(
            &point,
            &LocalShadows {
                resolution: 4096,
                ..Default::default()
            },
        );
        assert!(plan.views.is_empty());
    }
}
