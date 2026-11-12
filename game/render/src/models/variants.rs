//! Per-instance texture replacement (`MaterialOverride::textures`): a model
//! material drawn with other textures is a renderer material of its own, one
//! per (material, texture set), so instances with the same set still batch.
use super::{material_bind, MaterialId};
use std::collections::{BTreeMap, BTreeSet};

/// Each variant by its base material's slot and the texture names it binds.
pub(crate) type Variants = BTreeMap<(usize, [Option<String>; 5]), MaterialId>;

impl<const ASSETS: bool> crate::renderer::RendererWithAssets<ASSETS> {
    /// `base` with `replace`'s names in the slots it samples a texture in, or
    /// `None` while one of those textures cannot draw. A material a game's
    /// `CustomMaterial` shades keeps its own.
    pub(crate) fn textured_variant(
        &mut self,
        base: MaterialId,
        replace: &[Option<String>; 5],
    ) -> Option<MaterialId> {
        let material = self.models.materials.get(base.0)?;
        material.bind.as_ref()?;
        if self.models.custom.contains(&base) {
            return Some(base);
        }
        let mut names = material.names.clone();
        for (name, replacement) in names.iter_mut().zip(replace) {
            if let (Some(name), Some(replacement)) = (name.as_mut(), replacement) {
                if !self
                    .models
                    .textures
                    .get(replacement)
                    .is_some_and(|t| t.active)
                {
                    return None;
                }
                name.clone_from(replacement);
            }
        }
        if names == material.names {
            return Some(base);
        }
        let key = (base.0, names);
        if let Some(&id) = self.models.variants.get(&key) {
            return Some(id);
        }
        let data = material.data.clone();
        let slot = self
            .models
            .materials
            .iter()
            .position(|m| m.bind.is_none())
            .unwrap_or(self.models.materials.len());
        let variant = material_bind(
            &self.device,
            &self.queue,
            &self.pipelines.models.as_ref().unwrap().material,
            (&data, &key.1),
            &self.models.textures,
            None,
        );
        if slot == self.models.materials.len() {
            self.models.materials.push(variant);
        } else {
            self.models.materials[slot] = variant;
        }
        self.models.variants.insert(key, MaterialId(slot));
        Some(MaterialId(slot))
    }
}

/// Keep the variants a draw record uses whose base material is live and whose
/// textures are all resident, adding their slots to `live`; the others' slots
/// are reclaimed.
pub(super) fn retain(
    variants: &mut Variants,
    live: &mut BTreeSet<usize>,
    textures: &BTreeMap<String, super::Texture>,
    records: &[crate::DrawInstance],
) {
    let drawn: BTreeSet<_> = records.iter().map(|r| r.material.0).collect();
    variants.retain(|(base, names), id| {
        live.contains(base)
            && drawn.contains(&id.0)
            && names
                .iter()
                .flatten()
                .all(|n| textures.get(n).is_some_and(|t| t.active))
    });
    live.extend(variants.values().map(|id| id.0));
}

/// Bytes of variants no draw record uses: retired, so compaction counts them.
pub(super) fn unused_bytes(
    variants: &Variants,
    materials: &[super::Material],
    records: &[crate::DrawInstance],
) -> u64 {
    let drawn: BTreeSet<_> = records.iter().map(|r| r.material.0).collect();
    variants
        .values()
        .filter(|id| !drawn.contains(&id.0))
        .map(|id| materials.get(id.0).map_or(0, |m| m.bytes))
        .sum()
}
