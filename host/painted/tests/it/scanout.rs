use exact_ink::{Rotation, Wgpu};
use exact_painted::app::{boot_scanout_presenter, Config};
use exact_painted::presenter::PanelImage;
use tiny_skia::ColorU8;

#[test]
fn mirror_changes_where_ink_rotates_the_picture() {
    let plan = contract::compile(
        r##"component App
  view
    box width=48 height=32 background-color="#ff0000"
      box width=12 height=8 background-color="#00ff00"
"##,
    )
    .unwrap()
    .encode();
    let mut config = Config {
        output_rotation: Rotation::R90,
        scale: 1.,
        ..Config::from_bake::<false>(&plan, "{}")
    };
    let (mut p, warning) =
        boot_scanout_presenter::<()>(&mut config, (48., 32.), Wgpu::new().unwrap()).unwrap();
    assert!(warning.is_none(), "{warning:?}");

    for mirror in [false, true, false] {
        p.repaint();
        let frame = p.panel_frame(true, mirror).unwrap();
        let PanelImage::Ink { view, rotation, .. } = frame.image() else {
            panic!("Ink scanout returned a CPU frame");
        };
        let texture = view.texture();
        if mirror {
            assert_eq!(*rotation, Rotation::R0);
            assert_eq!((texture.width(), texture.height()), (32, 48));
            let pixels = frame.mirror().unwrap();
            assert_eq!((pixels.width(), pixels.height()), (32, 48));
            assert_eq!(
                pixels.pixel(28, 6).unwrap().demultiply(),
                ColorU8::from_rgba(0, 255, 0, 255)
            );
            assert_eq!(
                pixels.pixel(3, 6).unwrap().demultiply(),
                ColorU8::from_rgba(255, 0, 0, 255)
            );
        } else {
            assert_eq!(*rotation, Rotation::R90);
            assert_eq!((texture.width(), texture.height()), (48, 32));
            assert!(frame.mirror().is_none());
        }
        assert!(p.panel_complete(&frame));
    }
}
