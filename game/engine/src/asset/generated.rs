use super::{AssetState, AssetStore, Assets, MaterialData, MeshData, Model, Node};
use crate::{Data, DataError, Mesh, Reader, World, Writer};
use std::collections::BTreeMap;

// Keep the geometry/identity executor out of primitive-only worlds.
#[derive(Clone, Copy)]
pub(super) struct IdentityCodec {
    write: fn(&Assets, &mut dyn Writer),
    read: fn(&Assets, &mut dyn Reader) -> Result<(), DataError>,
    restart: fn(&mut AssetStore),
}
impl AssetStore {
    pub(crate) fn restart_generated(&mut self) {
        if let Some(codec) = self.identity {
            (codec.restart)(self);
        }
    }
    pub(crate) fn identify(&mut self, name: &str, digest: u64) {
        self.identities.insert(name.into(), digest);
        self.identity = Some(&IdentityCodec {
            write: |assets, w| {
                w.field("assetIdentity");
                assets.identities.write(w);
            },
            read: |assets, r| {
                let mut saved = BTreeMap::<String, u64>::new();
                saved.read(r)?;
                for name in saved.keys().chain(assets.identities.keys()) {
                    if saved.get(name) != assets.identities.get(name) {
                        return Err(DataError::new(format!("restore refused: asset `{name}` identity differs; recreate from the original level and generator")));
                    }
                }
                Ok(())
            },
            restart: |assets| {
                // Only generated models carry identities; delivered models and
                // levels persist. Retirement also invalidates presentation caches.
                let names: Vec<_> = assets
                    .identities
                    .keys()
                    .filter(|name| name.ends_with(".model"))
                    .cloned()
                    .collect();
                for name in names {
                    assets.identities.remove(&name);
                    assets.declared.remove(&name);
                    assets.retire_name(name);
                }
                if assets.identities.is_empty() {
                    assets.identity = None;
                }
            },
        });
    }
    pub(crate) fn write_identity(&self, w: &mut dyn Writer) {
        if let Some(codec) = self.identity {
            (codec.write)(self, w);
        }
    }
    pub(crate) fn require_identity(&self, seen: bool) -> Result<(), DataError> {
        if !seen {
            if let Some(name) = self.identities.keys().next() {
                return Err(DataError::new(format!(
                    "restore refused: asset `{name}` identity is missing"
                )));
            }
        }
        Ok(())
    }
    pub(crate) fn read_identity(&self, r: &mut dyn Reader) -> Result<(), DataError> {
        if let Some(codec) = self.identity {
            return (codec.read)(self, r);
        }
        r.begin_struct()?;
        if let Some(name) = r.field()? {
            return Err(DataError::new(format!(
                "restore refused: asset `{name}` was not reconstructed before restore"
            )));
        }
        Ok(())
    }
}
impl World {
    /// Register immutable CPU geometry during setup; entities retain only its name.
    /// Reconstruct from the same declared level/seed before restoring a save. Saves
    /// retain its content identity, never vertices; changed generators refuse by name.
    /// Repeating an identical registration reuses the existing shared allocation.
    pub fn generated(&mut self, name: &str, mesh: MeshData) -> Result<Mesh, String> {
        self.generated_model(
            name,
            Model {
                bounds: mesh.bounds,
                meshes: vec![mesh],
                materials: vec![MaterialData {
                    metallic: 0.,
                    ..Default::default()
                }],
                nodes: vec![Node {
                    mesh: Some(0),
                    ..Default::default()
                }],
                ..Default::default()
            },
        )
    }
    /// `generated` for a whole model: several meshes, nodes and materials, which
    /// may sample textures by name (`model.textures`, indexed by each material's
    /// texture slots) — shared `.tex` assets such as `art/textures/soil.png`, which
    /// are requested as the model's dependencies. The model is drawable once they
    /// arrive; declare them in `Game::ASSETS` to have them before setup.
    /// Skinned rigs and clips ([`crate::rig`]) register the same way.
    pub fn generated_model(&mut self, name: &str, model: Model) -> Result<Mesh, String> {
        self.sim_writes(format_args!("generated `{name}`"));
        if self.tick() != 0 {
            return Err(format!("generated `{name}`: register during setup"));
        }
        if !super::asset_name(name) || !name.ends_with(".model") {
            return Err(format!("generated `{name}`: expected a .model asset name"));
        }
        model
            .validate()
            .map_err(|e| format!("generated `{name}`: {e}"))?;
        let digest = crate::hash::of(&model);
        if let Some(prior) = self.assets.models.get(name) {
            if self.assets.identities.get(name) != Some(&digest)
                || crate::hash::of(prior.model.as_ref()) != digest
            {
                return Err(format!(
                    "generated `{name}`: immutable name already registered with different content"
                ));
            }
            return Ok(Mesh::asset(name));
        }
        self.assets.request(name);
        for texture in &model.textures {
            self.assets.request(texture);
            self.assets.required.insert(texture.clone());
        }
        let ready = model
            .textures
            .iter()
            .all(|t| self.assets.states.get(t) == Some(&AssetState::Loaded));
        self.assets.identify(name, digest);
        // A retired cosmetic name taken by generated geometry is no longer cosmetic.
        self.assets.cosmetic.remove(name);
        self.assets.declared.insert(name.into());
        let model_textures = model.textures.clone();
        self.assets.models.insert(name.into(), model.into());
        self.assets.set_dependencies(name, model_textures);
        let state = if ready {
            AssetState::Loaded
        } else {
            AssetState::Pending
        };
        self.assets.states.insert(name.into(), state);
        Ok(Mesh::asset(name))
    }
}
impl World {
    /// `Present::cosmetic_model`: presentation residency, never an identity,
    /// a declaration or a requirement, so nothing here reaches the simulation.
    pub(crate) fn cosmetic_model(
        &mut self,
        name: &str,
        build: impl FnOnce() -> Model,
    ) -> Result<(), String> {
        let assets = &self.assets;
        if assets.cosmetic.contains(name)
            && assets.models.contains_key(name)
            && !assets.identities.contains_key(name)
        {
            return Ok(());
        }
        if !super::asset_name(name) || !name.ends_with(".model") {
            return Err(format!("cosmetic `{name}`: expected a .model asset name"));
        }
        if assets.declared.contains(name)
            || assets.identities.contains_key(name)
            || (assets.models.contains_key(name) && !assets.cosmetic.contains(name))
        {
            return Err(format!(
                "cosmetic `{name}`: the name is a declared, generated or delivered model"
            ));
        }
        let model = build();
        model
            .validate()
            .map_err(|e| format!("cosmetic `{name}`: {e}"))?;
        let assets = &mut *self.assets;
        if !assets.cosmetic.contains(name) {
            assets.cosmetic.insert(name.into());
        }
        for texture in &model.textures {
            assets.request(texture);
        }
        // A texture that already failed is not delivered again: the model fails now.
        let failed = model
            .textures
            .iter()
            .find_map(|t| match assets.states.get(t) {
                Some(AssetState::Failed(e)) => Some(e.clone()),
                _ => None,
            });
        let ready = model
            .textures
            .iter()
            .all(|t| assets.states.get(t) == Some(&AssetState::Loaded));
        assets.set_dependencies(name, model.textures.clone());
        assets.models.insert(name.into(), model.into());
        assets.states.insert(
            name.into(),
            match failed {
                Some(reason) => AssetState::Failed(reason),
                None if ready => AssetState::Loaded,
                None => AssetState::Pending,
            },
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn textured(texture: &str) -> Model {
        let mesh = MeshData {
            positions: vec![0., 0., 0., 0., 0., 1., 1., 0., 0.],
            normals: [0., 1., 0.].repeat(3),
            uvs: vec![0.; 6],
            indices: vec![0, 1, 2],
            bounds: [0., 0., 0., 1., 0., 1.],
            ..Default::default()
        };
        Model {
            bounds: mesh.bounds,
            meshes: vec![mesh],
            materials: vec![MaterialData {
                base_color_texture: Some(0),
                ..Default::default()
            }],
            textures: vec![texture.into()],
            nodes: vec![Node {
                mesh: Some(0),
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    // A texture that already failed is not delivered again: a cosmetic model
    // registered over it fails at once instead of waiting forever.
    #[test]
    fn a_cosmetic_model_over_a_failed_texture_fails_at_once() {
        let mut w = World::new(60, 0);
        w.assets
            .states
            .insert("gone.tex".into(), AssetState::Failed("gone".into()));
        w.cosmetic_model("late.model", || textured("gone.tex"))
            .unwrap();
        assert_eq!(
            w.assets.states.get("late.model"),
            Some(&AssetState::Failed("gone".into()))
        );
    }

    // A retired cosmetic name later registered as generated geometry is the
    // generated model's: a cosmetic registration is refused without building.
    #[test]
    fn a_retired_cosmetic_name_taken_by_generated_geometry_is_refused() {
        let mut w = World::new(60, 0);
        w.cosmetic_model("skin.model", || textured("a.tex"))
            .unwrap();
        w.assets.retire(&Default::default());
        assert!(w.assets.models.get("skin.model").is_none());
        w.generated_model("skin.model", textured("b.tex")).unwrap();
        let error = w
            .cosmetic_model("skin.model", || unreachable!("never built"))
            .unwrap_err();
        assert!(error.contains("generated"), "{error}");
        assert!(!w.assets.cosmetic.contains("skin.model"));
    }
}
