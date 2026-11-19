//! The content-region seam (LLP 1047.001 §9.8, C13): a region's layout,
//! publication and native projection behind [`ContentRegions`], which only
//! an entry's [`crate::link::Links`] names, so a binary whose entry
//! registers no region holds none of it.
//!
//! The host's calls stay here as fronts. Each answers as the host does
//! with no region registered before it asks the seam, so a host without a
//! region makes no indirect call on its layout and commit paths.
use super::*;
use crate::content_region::{ContentRegionRegistration, NativeProjectionLimits};
use exact_kernel::Kernel;
use std::any::Any;

/// A host's content region. The provided methods are what a host with no
/// region answers; `LinkedRegions` (`content_region/host.rs`) implements
/// every one.
pub trait ContentRegions<D: DataSource> {
    /// The region `registration` names, as the host keeps it, before any
    /// text layout; `native`, a native projection's limits (a test's).
    fn boot(
        &self,
        _: &mut Kernel,
        registration: Option<ContentRegionRegistration>,
        native: Option<NativeProjectionLimits>,
    ) -> Result<Option<Box<dyn Any>>, String> {
        match (registration, native) {
            (None, None) => Ok(None),
            (None, Some(_)) => Err("native registration missing".into()),
            (Some(_), _) => Err(
                "an entry that registers a content region links `content_regions`: name it in app.json's `link`"
                    .into(),
            ),
        }
    }
    /// Lay `root` out as the region's container, staging what it publishes.
    fn region_layout(
        &self,
        _: &mut Host<D>,
        _root: ViewId,
        _offer: Offer,
        _: &mut Batch,
    ) -> Result<(), String> {
        Ok(())
    }
    /// Whether the region projects its content as native views.
    fn native_mode(&self, _: &Host<D>) -> bool {
        false
    }
    /// Whether the selected native projection is current.
    fn native_current(&self, _: &Host<D>) -> bool {
        false
    }
    /// Whether `id` is a view of the selected native projection.
    fn native_selected_id(&self, _: &Host<D>, _id: ViewId) -> bool {
        false
    }
    /// Whether `id` is inside the region's content.
    fn native_captured_id(&self, _: &Host<D>, _id: ViewId) -> bool {
        false
    }
    /// A captured node's block moved in or out of the content.
    fn native_moved(&self, _: &mut Host<D>) {}
    /// The region's owner or content left the tree: its projection retires.
    fn native_retire_removed_owner(&self, _: &mut Host<D>, _: &mut Batch) {}
    /// A commit's receipts, for what they changed of the content.
    fn native_note_receipts(&self, _: &mut Host<D>, _: &[Timed]) {}
    /// Prepare the next native candidate.
    fn native_prepare_candidate(&self, _: &mut Host<D>) -> Result<(), String> {
        Ok(())
    }
    /// The selected projection's lists, as the batch's collections.
    fn native_collections_json(&self, _: &Host<D>) -> Result<String, String> {
        Err("native consumer missing".into())
    }
}

/// No content region: the host alone.
pub struct NoRegions;
impl<D: DataSource> ContentRegions<D> for NoRegions {}

impl<D: DataSource> Host<D> {
    pub(super) fn region_layout(
        &mut self,
        root: ViewId,
        offer: Offer,
        batch: &mut Batch,
    ) -> Result<(), String> {
        let regions = self.regions;
        regions.region_layout(self, root, offer, batch)
    }
    pub(super) fn native_mode(&self) -> bool {
        self.content_region.is_some() && self.regions.native_mode(self)
    }
    pub(super) fn native_current(&self) -> bool {
        self.content_region.is_some() && self.regions.native_current(self)
    }
    pub(super) fn native_selected_id(&self, id: ViewId) -> bool {
        self.content_region.is_some() && self.regions.native_selected_id(self, id)
    }
    pub(super) fn native_protected_id(&self, id: ViewId) -> bool {
        self.native_selected_id(id) || self.native_captured_id(id)
    }
    pub(super) fn native_captured_id(&self, id: ViewId) -> bool {
        self.content_region.is_some() && self.regions.native_captured_id(self, id)
    }
    pub(super) fn native_moved(&mut self) {
        if self.content_region.is_some() {
            let regions = self.regions;
            regions.native_moved(self);
        }
    }
    pub(super) fn native_retire_removed_owner(&mut self, batch: &mut Batch) {
        if self.content_region.is_some() {
            let regions = self.regions;
            regions.native_retire_removed_owner(self, batch);
        }
    }
    pub(super) fn native_note_receipts(&mut self, receipts: &[Timed]) {
        if self.content_region.is_some() {
            let regions = self.regions;
            regions.native_note_receipts(self, receipts);
        }
    }
    pub(super) fn native_prepare_candidate(&mut self) -> Result<(), String> {
        if self.content_region.is_none() {
            return Ok(());
        }
        let regions = self.regions;
        regions.native_prepare_candidate(self)
    }
    pub(super) fn native_collections_json(&self) -> Result<String, String> {
        self.regions.native_collections_json(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Without the family, a host boots no region and refuses an entry that
    /// registers one, naming what to link; with it, the registration reaches
    /// the region (whose owner this empty tree lacks). `Links::of` names the
    /// family only for `Capability::ContentRegions`.
    #[test]
    fn a_region_boots_only_where_content_regions_are_linked() {
        use crate::link::{Capability, Links, Uses};
        let registration = ContentRegionRegistration {
            activate: None,
            owner: "owner",
            content: "content",
            pending: "pending",
        };
        let unlinked = "links `content_regions`";
        let mut kernel = Kernel::with_monospace();
        let none: &dyn ContentRegions<()> = &NoRegions;
        assert!(none.boot(&mut kernel, None, None).unwrap().is_none());
        let refused = none
            .boot(&mut kernel, Some(registration), None)
            .unwrap_err();
        assert!(refused.contains(unlinked), "{refused}");
        for (uses, linked) in [
            (Uses::NONE, false),
            (Uses::NONE.with(Capability::ContentRegions), true),
        ] {
            let regions = Links::<()>::of(uses).regions;
            assert!(regions.boot(&mut kernel, None, None).unwrap().is_none());
            let answer = regions
                .boot(&mut kernel, Some(registration), None)
                .unwrap_err();
            assert_eq!(!answer.contains(unlinked), linked, "{answer}");
        }
    }
}
