//! Exercise the adapter's real pass boundaries with an ordered fake queue.
use super::*;
use exact_ink::device::{Submission, TextureDesc, TextureUpload};
use exact_ink::instance::Frame;
use std::collections::HashMap;
type Native = exact_ink::device::Selected;
type Buffer = <Native as Device>::Buffer;

pub enum Selected {
    Wgpu(Native),
    Other(Native),
    Fake(Fake),
    Counted(Box<super::memory_tests::Counted>),
}
impl From<Native> for Selected {
    fn from(device: Native) -> Self {
        if matches!(device, Native::Wgpu(_)) {
            Self::Wgpu(device)
        } else {
            Self::Other(device)
        }
    }
}
impl Selected {
    pub fn from_env() -> Result<Self> {
        Native::from_env().map(Into::into)
    }
    fn device(&self) -> &dyn Device<Buffer = Buffer> {
        match self {
            Self::Wgpu(d) | Self::Other(d) => d,
            Self::Fake(d) => d,
            Self::Counted(d) => d.as_ref(),
        }
    }
    fn device_mut(&mut self) -> &mut dyn Device<Buffer = Buffer> {
        match self {
            Self::Wgpu(d) | Self::Other(d) => d,
            Self::Fake(d) => d,
            Self::Counted(d) => d.as_mut(),
        }
    }
    #[cfg(target_os = "macos")]
    pub fn has_layer(&self) -> bool {
        match self {
            Self::Wgpu(d) | Self::Other(d) => d.has_layer(),
            Self::Fake(_) | Self::Counted(_) => false,
        }
    }
    #[cfg(target_os = "macos")]
    pub fn attach_layer(&mut self, layer: exact_ink::device::apple::Layer) -> Result<()> {
        match self {
            Self::Wgpu(d) | Self::Other(d) => d.attach_layer(layer),
            Self::Fake(_) | Self::Counted(_) => unreachable!(),
        }
    }
    #[cfg(target_os = "macos")]
    pub fn present(&mut self, texture: TextureId) -> Result<f64> {
        match self {
            Self::Wgpu(d) | Self::Other(d) => d.present(texture),
            Self::Fake(_) | Self::Counted(_) => unreachable!(),
        }
    }

    #[cfg(all(feature = "scanout", target_os = "linux"))]
    #[allow(unsafe_code)]
    pub unsafe fn attach_scanout(
        &mut self,
        fd: i32,
        connector: u32,
        size: [u32; 2],
        refresh: u32,
    ) -> Result<()> {
        match self {
            Self::Wgpu(d) | Self::Other(d) => unsafe {
                // SAFETY: the caller passes through the native device's contract.
                d.attach_scanout(fd, connector, size, refresh)
            },
            Self::Fake(_) | Self::Counted(_) => unreachable!(),
        }
    }

    #[cfg(all(feature = "scanout", target_os = "linux"))]
    pub fn has_scanout(&self) -> bool {
        match self {
            Self::Wgpu(d) | Self::Other(d) => d.has_scanout(),
            Self::Fake(d) => d.display.is_some(),
            Self::Counted(_) => false,
        }
    }

    #[cfg(all(feature = "scanout", target_os = "linux"))]
    pub fn scan_out(&mut self, texture: TextureId, rotation: exact_ink::Rotation) -> Result<()> {
        match self {
            Self::Wgpu(d) | Self::Other(d) => d.scan_out(texture, rotation),
            Self::Fake(d) => {
                d.display.as_mut().unwrap().push((texture, rotation));
                Ok(())
            }
            Self::Counted(_) => unreachable!(),
        }
    }
}
macro_rules! forward {
    ($($name:ident($($arg:ident: $ty:ty),*) -> $ret:ty;)+) => {
        $(fn $name(&mut self, $($arg: $ty),*) -> $ret { self.device_mut().$name($($arg),*) })+
    };
}
impl Device for Selected {
    type Buffer = Buffer;
    fn info(&self) -> &Info {
        self.device().info()
    }
    fn max_texture_dimension(&self) -> u32 {
        self.device().max_texture_dimension()
    }
    fn create_buffer(&self, bytes: u64) -> Result<Buffer> {
        self.device().create_buffer(bytes)
    }
    fn last_use(&self, id: TextureId) -> Result<Submission> {
        self.device().last_use(id)
    }
    forward! {
        upload_buffer(buffer: &Buffer, offset: u64, data: &[u8]) -> Result<Submission>;
        create_texture(desc: TextureDesc) -> Result<TextureId>;
        upload_texture(id: TextureId, origin: [u32; 2], size: [u32; 2], data: &[u8]) -> Result<Submission>;
        upload_textures(uploads: &[TextureUpload]) -> Result<Submission>;
        generate_mips(id: TextureId) -> Result<Submission>;
        create_target(width: u32, height: u32) -> Result<TextureId>;
        resize_target(id: TextureId, width: u32, height: u32) -> Result<TextureId>;
        copy_region(source: TextureId, origin: [u32; 2], target: TextureId, dst: [u32; 2], size: [u32; 2]) -> Result<Submission>;
        render(frame: &Frame, target: TextureId, clear: Option<[f64; 4]>) -> Result<Submission>;
        readback(id: TextureId) -> Result<Vec<u8>>;
        wait(submission: Submission) -> Result<()>;
        completed() -> Result<Submission>;
        retire_texture(id: TextureId) -> Result<()>;
        trim() -> Result<()>;
    }
}

#[derive(Debug, PartialEq)]
enum Event {
    Upload(TextureId),
    Draw(Vec<TextureId>),
}
#[derive(Default)]
pub struct Fake {
    textures: HashMap<TextureId, Submission>,
    events: Vec<Event>,
    submitted: Submission,
    copies: Vec<([u32; 2], [u32; 2])>,
    retired: Vec<TextureId>,
    /// The frames its display was given, when it has one.
    #[cfg(all(feature = "scanout", target_os = "linux"))]
    display: Option<Vec<(TextureId, exact_ink::Rotation)>>,
}
impl Device for Fake {
    type Buffer = Buffer;
    fn info(&self) -> &Info {
        unreachable!()
    }
    fn create_buffer(&self, _: u64) -> Result<Buffer> {
        unreachable!()
    }
    fn upload_buffer(&mut self, _: &Buffer, _: u64, _: &[u8]) -> Result<Submission> {
        unreachable!()
    }
    fn create_texture(&mut self, _: TextureDesc) -> Result<TextureId> {
        let id = TextureId(self.textures.len() as u64 + 1);
        self.textures.insert(id, Submission(0));
        Ok(id)
    }
    fn upload_texture(
        &mut self,
        id: TextureId,
        _: [u32; 2],
        _: [u32; 2],
        _: &[u8],
    ) -> Result<Submission> {
        self.submitted.0 += 1;
        self.textures.insert(id, self.submitted);
        self.events.push(Event::Upload(id));
        Ok(self.submitted)
    }
    fn last_use(&self, id: TextureId) -> Result<Submission> {
        Ok(self.textures[&id])
    }
    fn generate_mips(&mut self, _: TextureId) -> Result<Submission> {
        Ok(self.submitted)
    }
    fn resize_target(&mut self, id: TextureId, _: u32, _: u32) -> Result<TextureId> {
        Ok(id)
    }
    fn copy_region(
        &mut self,
        _: TextureId,
        origin: [u32; 2],
        _: TextureId,
        dst: [u32; 2],
        size: [u32; 2],
    ) -> Result<Submission> {
        assert_eq!(dst, [0, 0]);
        self.copies.push((origin, size));
        Ok(self.submitted)
    }
    fn render(&mut self, f: &Frame, target: TextureId, _: Option<[f64; 4]>) -> Result<Submission> {
        let mut sampled = Vec::new();
        for batch in &f.batches {
            for id in [batch.bindings.glyph, batch.bindings.color] {
                if id.0 != 0 {
                    assert!(
                        self.events.contains(&Event::Upload(id)),
                        "sampled before upload"
                    );
                    sampled.push(id);
                }
            }
        }
        self.events.push(Event::Draw(sampled));
        self.submitted.0 += 1;
        self.textures.insert(target, self.submitted);
        Ok(self.submitted)
    }
    fn readback(&mut self, _: TextureId) -> Result<Vec<u8>> {
        unreachable!()
    }
    fn wait(&mut self, _: Submission) -> Result<()> {
        panic!("the pass must not wait")
    }
    fn completed(&mut self) -> Result<Submission> {
        Ok(self.submitted)
    }
    fn retire_texture(&mut self, id: TextureId) -> Result<()> {
        self.retired.push(id);
        Ok(())
    }
    fn trim(&mut self) -> Result<()> {
        Ok(())
    }
}

fn glyph(ink: &mut Ink, id: u16) -> TextureId {
    let font = exact_ink::glyph::Font {
        id: [1, 0],
        data: include_bytes!("../../../../scripts/fixtures/fonts/assets/DejaVuSans.ttf"),
        index: 0,
        coords: &[],
        skew: false,
    };
    let g = ink
        .glyphs
        .get(&mut ink.device, &font, id, 24., 0, 0.)
        .unwrap()
        .unwrap();
    let mut i = Instance::new(
        Kind::Glyph,
        Brush::Solid,
        [0., 0., g.size[0] as f32, g.size[1] as f32],
    );
    i.uv = g.entry.uv;
    ink.emit(
        i,
        Bindings {
            glyph: g.entry.texture,
            ..Default::default()
        },
    )
    .unwrap();
    g.entry.texture
}
fn checkpoint(ink: &Ink, before: usize, texture: TextureId) -> usize {
    let Selected::Fake(d) = &ink.device else {
        unreachable!()
    };
    let events = &d.events[before..];
    assert_eq!(events.first(), Some(&Event::Upload(texture)));
    assert_eq!(events.get(1), Some(&Event::Draw(vec![texture])));
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, Event::Upload(_)))
            .count(),
        1
    );
    for e in &events[2..] {
        assert!(matches!(e, Event::Draw(ids) if ids.is_empty()), "{e:?}");
    }
    d.events.len()
}

#[test]
fn every_glyph_pass_flushes_before_sampling_including_masks_found_later() {
    let mut ink = Ink::from_device(Selected::Fake(Fake::default()));
    ink.begin(64., 64., 1.);
    ink.open_group(1.);
    let t = glyph(&mut ink, 36);
    ink.close_group().unwrap();
    let a = checkpoint(&ink, 0, t);
    ink.push_opacity(0.5);
    let t = glyph(&mut ink, 37);
    ink.pop_opacity();
    let b = checkpoint(&ink, a, t);
    let t = glyph(&mut ink, 38);
    ink.backdrop(
        &Shape::rect((0., 0., 32., 32.)),
        &exact_kernel::style::BackdropFilter(vec![exact_kernel::style::BackdropOp::Saturate(0.5)]),
        Transform::identity(),
    )
    .unwrap();
    let c = checkpoint(&ink, b, t);
    let t = glyph(&mut ink, 39);
    ink.flush().unwrap();
    let end = checkpoint(&ink, c, t);
    assert!(ink.error.is_none());
    println!("ordered glyph passes: intermediate group, opacity, backdrop, later masks; {end} queue events");
}

#[test]
fn small_effects_copy_their_region_in_a_large_viewport() {
    let mut ink = Ink::from_device(Selected::Fake(Fake::default()));
    ink.begin(1920., 1080., 1.);
    let pixels = Arc::new(Pixmap::new(1, 1).unwrap());
    ink.island_image(pixels, (120., 80., 20., 10.), Transform::identity(), 1);
    ink.backdrop_filter(
        &Shape::rect((300., 200., 30., 20.)),
        &exact_kernel::style::BackdropFilter(vec![exact_kernel::style::BackdropOp::Blur(4.)]),
        Transform::identity(),
    );
    assert!(ink.error.is_none(), "{:?}", ink.error);
    let Selected::Fake(d) = &ink.device else {
        unreachable!()
    };
    assert_eq!(d.copies.len(), 2);
    let area: u32 = d.copies.iter().map(|(_, size)| size[0] * size[1]).sum();
    println!("small effects: copied {area} pixels, {:?}", d.copies);
    assert!(area < 2000, "copied {area} pixels: {:?}", d.copies);
}

#[test]
fn destination_copies_follow_transforms_clips_and_filter_support() {
    use exact_kernel::style::{BackdropFilter, BackdropOp};
    let mut ink = Ink::from_device(Selected::Fake(Fake::default()));
    ink.begin(1920., 1080., 1.);
    let pixels = Arc::new(Pixmap::new(1, 1).unwrap());
    let clip = Shape::rect((250., 220., 15., 10.));
    ink.push_clip(&clip, Transform::identity());
    ink.island_image(
        pixels.clone(),
        (10., 20., 30., 40.),
        Transform::from_row(0., 1., -1., 0., 300., 200.),
        1,
    );
    ink.backdrop_filter(
        &Shape::rect((100., 100., 600., 400.)),
        &BackdropFilter(vec![BackdropOp::Blur(4.)]),
        Transform::identity(),
    );
    ink.backdrop_filter(
        &Shape::rect((100., 100., 600., 400.)),
        &BackdropFilter(vec![BackdropOp::Blur(f32::MAX)]),
        Transform::identity(),
    );
    ink.pop_clip();
    ink.push_clip(&Shape::rect((0., 0., 1., 1.)), Transform::identity());
    ink.island_image(pixels, (100., 100., 50., 50.), Transform::identity(), 1);
    ink.pop_clip();
    assert!(ink.error.is_none(), "{:?}", ink.error);
    let Selected::Fake(d) = &ink.device else {
        unreachable!()
    };
    assert_eq!(d.copies.len(), 3, "invisible effects need no snapshot");
    assert_eq!(d.copies[0], ([250, 220], [15, 10]));
    let (origin, size) = d.copies[1];
    assert!(
        origin[0] < 250 && origin[1] < 220,
        "blur samples beyond its clip"
    );
    assert!(origin[0] + size[0] > 265 && origin[1] + size[1] > 230);
    assert!(
        size[0] < 60 && size[1] < 60,
        "sampling support: {:?}",
        d.copies[1]
    );
    assert_eq!(d.copies[2], ([100, 100], [600, 400]));
}

#[cfg(all(feature = "scanout", target_os = "linux"))]
#[test]
fn a_display_turns_the_frame_itself_so_no_pass_rotates_it() {
    let mut ink = Ink::from_device(Selected::Fake(Fake {
        display: Some(Vec::new()),
        ..Default::default()
    }));
    let rotation = exact_ink::Rotation::R90;
    ink.begin(64., 32., 1.);
    ink.finish_pixels(rotation).unwrap();
    let Selected::Fake(d) = &ink.device else {
        unreachable!()
    };
    assert_eq!(d.events, [Event::Draw(vec![])]);
    assert_eq!(d.display, Some(vec![(ink.target.unwrap(), rotation)]));
}

#[test]
fn a_canvas_redrawn_at_its_size_keeps_one_texture() {
    fn frame(images: &mut images::Images, device: &mut Fake, pixels: &Arc<Pixmap>) -> TextureId {
        images.begin(device).unwrap();
        images.pixmap(device, pixels, false).unwrap()
    }
    let (mut device, mut images) = (Fake::default(), images::Images::default());
    let snapshot = |w| Arc::new(Pixmap::new(w, 32).unwrap());
    let mut shown = snapshot(64);
    let first = frame(&mut images, &mut device, &shown);
    // Each redraw is a new snapshot, the last one let go: uploaded in place.
    for _ in 0..3 {
        shown = snapshot(64);
        assert_eq!(frame(&mut images, &mut device, &shown), first);
    }
    // The same snapshot drawn again uploads nothing.
    assert_eq!(frame(&mut images, &mut device, &shown), first);
    shown = snapshot(80);
    let resized = frame(&mut images, &mut device, &shown);
    assert_ne!(resized, first);
    assert!(device.retired.is_empty());
    assert_eq!(frame(&mut images, &mut device, &shown), resized);
    assert_eq!(device.retired, [first]);
    assert_eq!(device.textures.len(), 2);
    let uploads = |t| {
        device
            .events
            .iter()
            .filter(|e| **e == Event::Upload(t))
            .count()
    };
    assert_eq!((uploads(first), uploads(resized)), (4, 1));
}

fn bitmap(session: &exact_raster::RasterSession, [w, h]: [u32; 2]) -> Arc<Bitmap> {
    let bytes = u64::from(w) * u64::from(h) * 4;
    let reservation = session.reserve_allocation(bytes).unwrap();
    let image = Arc::new(Bitmap::new(
        Pixmap::new(w, h).unwrap(),
        (w, h),
        reservation.charge(),
    ));
    reservation.commit(bytes).unwrap();
    image
}

#[test]
fn only_an_opted_in_full_size_bitmap_reuses_its_upload_after_leaving_the_frame() {
    let session = exact_raster::Gate::new().session();
    for enabled in [false, true] {
        let size = [17, 9];
        let image = bitmap(&session, size);
        let (mut device, mut images) = (Fake::default(), images::Images::new(enabled));
        images.begin(&mut device).unwrap();
        let full = images.bitmap(&mut device, &image, size).unwrap();
        let reduced = images.bitmap(&mut device, &image, [17, 2]).unwrap();
        images.end(&mut device).unwrap();
        assert_eq!(
            Arc::strong_count(&image),
            3,
            "each active variant owns the decode"
        );
        images.begin(&mut device).unwrap();
        images.end(&mut device).unwrap();
        assert_eq!(
            Arc::strong_count(&image),
            1,
            "neither idle variant owns the decode"
        );
        assert!(device.retired.contains(&reduced));
        assert_eq!(device.retired.contains(&full), !enabled);
        images.begin(&mut device).unwrap();
        let again = images.bitmap(&mut device, &image, size).unwrap();
        assert_eq!(again == full, enabled);
        assert_eq!(device.events.len(), if enabled { 2 } else { 3 });
        images.end(&mut device).unwrap();
        images.begin(&mut device).unwrap();
        images.end(&mut device).unwrap();
        images.begin(&mut device).unwrap();
        drop(image);
        assert_eq!(session.stats().resident_bytes, 0);
        let replacement = bitmap(&session, size);
        assert_ne!(
            images.bitmap(&mut device, &replacement, size).unwrap(),
            again
        );
        images.end(&mut device).unwrap();
        assert!(device.retired.contains(&again));
    }
}

#[test]
fn recently_idle_bitmaps_fit_both_the_mip_byte_and_texture_count_limits() {
    let session = exact_raster::Gate::new().session();
    // 32 complete 256 x 192 mip chains fit in 8 MiB. Tiny images reach the count limit.
    for (size, limit) in [([256, 192], 32), ([1, 1], 64)] {
        let (mut device, mut images) = (Fake::default(), images::Images::new(true));
        let frame = |images: &mut images::Images, device: &mut Fake, picture: &Arc<Bitmap>| {
            images.begin(device).unwrap();
            let texture = images
                .bitmap(device, picture, [picture.width(), picture.height()])
                .unwrap();
            images.end(device).unwrap();
            texture
        };
        let mut pictures: Vec<_> = (0..=limit).map(|_| bitmap(&session, size)).collect();
        let textures: Vec<_> = pictures
            .iter()
            .map(|p| frame(&mut images, &mut device, p))
            .collect();
        assert_eq!(frame(&mut images, &mut device, &pictures[0]), textures[0]);
        assert!(
            device.retired.is_empty(),
            "the active bitmap has its own place"
        );
        images.begin(&mut device).unwrap();
        images.end(&mut device).unwrap();
        assert_eq!(device.retired, [textures[1]]);
        assert_ne!(frame(&mut images, &mut device, &pictures[1]), textures[1]);
        images.begin(&mut device).unwrap();
        images.end(&mut device).unwrap();
        assert_eq!(device.retired, [textures[1], textures[2]]);

        let next = bitmap(&session, size);
        frame(&mut images, &mut device, &next);
        images.begin(&mut device).unwrap();
        drop(pictures.pop());
        images.end(&mut device).unwrap();
        assert_eq!(device.retired, [textures[1], textures[2], textures[limit]]);
        let oversized = bitmap(&session, [2048, 1024]);
        let large = frame(&mut images, &mut device, &oversized);
        images.begin(&mut device).unwrap();
        images.end(&mut device).unwrap();
        assert_eq!(
            device.retired,
            [textures[1], textures[2], textures[limit], large]
        );
    }
}

#[test]
fn a_kept_rows_bitmap_keeps_its_decode_across_a_trim_in_both_modes() {
    let session = exact_raster::Gate::new().session();
    for enabled in [false, true] {
        for hidden in [false, true] {
            let mut ink = Ink::from_device(Selected::Fake(Fake::default()));
            ink.images = images::Images::new(enabled);
            let rect = (0., 0., 4., 4.);
            let kept = bitmap(&session, [4, 4]);
            let other = bitmap(&session, [4, 4]);
            let draw = |ink: &mut Ink, image: &Arc<Bitmap>| {
                ink.image(
                    image,
                    rect,
                    &[],
                    Transform::identity(),
                    None,
                    exact_kernel::ImageRendering::Auto,
                );
            };
            ink.begin(4., 4., 1.);
            ink.row_begin(1, (0., 0.), None);
            draw(&mut ink, &kept);
            ink.row_end(rect);
            draw(&mut ink, &other);
            let Selected::Fake(device) = &ink.device else {
                unreachable!()
            };
            let [Event::Upload(a), Event::Upload(b)] = device.events.as_slice() else {
                panic!("expected both bitmap uploads");
            };
            let (row_texture, other_texture) = (*a, *b);
            ink.end_frame().unwrap();
            drop(kept);
            ink.begin(4., 4., 1.);
            ink.end_frame().unwrap();
            ink.trim(hidden);
            assert_eq!(session.stats().resident_bytes, 2 * other.bytes());
            let Selected::Fake(device) = &ink.device else {
                unreachable!()
            };
            assert!(device.retired.contains(&other_texture));
            assert!(!device.retired.contains(&row_texture));
            ink.begin(4., 4., 1.);
            ink.row_draw(1, (0., 0.), rect);
            assert!(ink
                .encoder
                .take_frame()
                .batches
                .iter()
                .any(|b| b.bindings.image == row_texture));
            ink.row_free(1);
            ink.begin(4., 4., 1.);
            assert_eq!(session.stats().resident_bytes, other.bytes());
            let Selected::Fake(device) = &ink.device else {
                unreachable!()
            };
            assert!(device.retired.contains(&row_texture));
        }
    }
}
