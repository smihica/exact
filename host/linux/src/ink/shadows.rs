//! Outer shadows rastered once per geometry and drawn translated, under
//! `KIOSK_SHADOW_CACHE=1`. The raster is the analytic shadow before the border
//! box's exclusion, which the drawing still applies at its own device position.
use super::*;
use std::collections::HashMap;

#[cfg(test)]
mod tests;

/// Raster storage held for the painter's life. Kept rows hold the textures they
/// drew, so nothing is evicted: past this a new geometry is drawn analytically.
const BUDGET: u64 = 16 << 20;

/// Keep sharp shadows analytic: subpixel filtering can soften their edges.
/// RGBA8 quantization and composition still introduce small differences above
/// this cutoff; pixel comparisons, rather than the cutoff, qualify the result.
const MIN_SIGMA: f32 = 4.;

#[derive(Default)]
pub(super) struct Shadows {
    enabled: bool,
    /// Scale, reach, size, radii and sigma bits: all the raster reads.
    rasters: HashMap<[u32; 9], TextureId>,
    bytes: u64,
    hits: u64,
    full: bool,
}

impl Shadows {
    pub(super) fn new(enabled: bool) -> Self {
        Self {
            enabled,
            ..Self::default()
        }
    }

    fn report(&self, event: &str) {
        eprintln!(
            "exact-ink: shadow cache {event}: {} rasters, {} of {BUDGET} bytes, {} hits",
            self.rasters.len(),
            self.bytes,
            self.hits
        );
    }
}

impl Ink {
    /// `blurred_shadow`'s instance `i`, its quad `reach` beyond the shape, drawn
    /// from its geometry's raster; `i` itself where there is none.
    pub(super) fn cached_shadow(
        &mut self,
        i: Instance,
        reach: f32,
    ) -> Result<(Instance, Bindings)> {
        let scale = i.xform[0];
        let sigma = i.param[0];
        let blur = sigma * scale;
        let size = [i.rect[2], i.rect[3]].map(|n| (n * scale).ceil());
        let limit = self.device.max_texture_dimension() as f32;
        if !self.shadows.enabled
            || self.error.is_some()
            || !blur.is_finite()
            || blur < MIN_SIGMA
            || !size.iter().all(|n| *n <= limit)
        {
            return Ok((i, Bindings::default()));
        }
        let size = size.map(|n| n as u32);
        let r = i.radii;
        let key = [
            scale, reach, i.uv[2], i.uv[3], r[0], r[1], r[2], r[3], sigma,
        ]
        .map(f32::to_bits);
        let texture = if let Some(&texture) = self.shadows.rasters.get(&key) {
            self.shadows.hits += 1;
            if self.shadows.hits.is_power_of_two() {
                self.shadows.report("reused");
            }
            texture
        } else {
            let bytes = u64::from(size[0]) * u64::from(size[1]) * 4;
            if self.shadows.bytes + bytes > BUDGET {
                if !self.shadows.full {
                    self.shadows.full = true;
                    self.shadows.report("full");
                }
                return Ok((i, Bindings::default()));
            }
            let texture = self.raster_shadow(&i, reach, size)?;
            self.shadows.rasters.insert(key, texture);
            self.shadows.bytes += bytes;
            self.shadows
                .report(&format!("rastered {}x{}", size[0], size[1]));
            texture
        };
        // Unit texture coordinates span the raster at its unrounded device origin.
        let mut d = Instance::new(Kind::Shadow, Brush::Image, [0., 0., 1., 1.]);
        d.xform = [
            size[0] as f32,
            0.,
            0.,
            size[1] as f32,
            i.xform[4] + scale * i.rect[0],
            i.xform[5] + scale * i.rect[1],
        ];
        d.flags = i.flags | flags::TINT;
        d.clip1 = i.clip1;
        d.clip_radii = i.clip_radii;
        d.param[0] = 0.;
        d.color = i.color;
        Ok((
            d,
            Bindings {
                image: texture,
                ..Default::default()
            },
        ))
    }

    /// The shadow's alpha at device scale, the quad's origin at the texture's.
    fn raster_shadow(&mut self, i: &Instance, reach: f32, size: [u32; 2]) -> Result<TextureId> {
        let started = Instant::now();
        let scale = i.xform[0];
        let mut r = Instance::new(
            Kind::Shadow,
            Brush::Solid,
            [0., 0., size[0] as f32 / scale, size[1] as f32 / scale],
        );
        r.xform = [scale, 0., 0., scale, 0., 0.];
        r.uv = [reach, reach, i.uv[2], i.uv[3]];
        r.radii = i.radii;
        r.param[0] = i.param[0];
        // An empty border box excludes nothing.
        r.set_clip(
            1,
            RoundedClip {
                rect: [0.; 4],
                radii: [0.; 4],
            },
        )
        .map_err(|_| Error::Invalid("shadow raster clip".into()))?;
        let mut e = Encoder::new(size[0], size[1]);
        e.push(r, Bindings::default())
            .map_err(|_| Error::Invalid("shadow raster clip".into()))?;
        let target = self.device.create_target(size[0], size[1])?;
        let rendered = self.device.render(&e.finish(), target, Some([0.; 4]));
        if rendered.is_err() {
            self.device.retire_texture(target)?;
        }
        rendered?;
        self.layer_ms += started.elapsed().as_secs_f64() * 1000.;
        Ok(target)
    }
}
