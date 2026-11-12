//! SVG 2 §8.10: a `use` has the object bounding box of what it instances,
//! placed where it draws, so what is measured by that box (a `filter`, a
//! `mask`, `transform-box: fill-box`) has one. Expected values are
//! Chrome 156's `getBBox()` for the `use` (`10,10,20,10`) and the regions it
//! derives from it.
use super::*;

/// An `svg` of 100 × 100 whose `defs` hold a `filter` `#f` (a blur), a
/// `mask` `#m` (white over the whole `svg`) and a rect `#r` at `10,10`,
/// 20 × 10; `place` adds the elements under test and returns the one to
/// read.
fn scene(place: impl FnOnce(&mut Doc) -> Vec<u32>) -> (Kernel, u32) {
    use NodeType::*;
    let mut d = Doc::new();
    let defs = d.node(SvgDefs, &[], &[]);
    let filter = d.node(SvgFilter, &[], &[(PropId::Id, "f")]);
    let blur = d.node(
        SvgFe,
        &[],
        &[(PropId::Fe, "feGaussianBlur"), (PropId::StdDeviation, "1")],
    );
    d.children(filter, &[blur]);
    let mask = d.node(SvgMask, &[], &[(PropId::Id, "m")]);
    let white = d.node(
        SvgRect,
        &[
            (StyleId::Width, "100"),
            (StyleId::Height, "100"),
            (StyleId::Fill, "#ffffff"),
        ],
        &[],
    );
    d.children(mask, &[white]);
    let rect = d.node(
        SvgRect,
        &[
            (StyleId::X, "10"),
            (StyleId::Y, "10"),
            (StyleId::Width, "20"),
            (StyleId::Height, "10"),
            (StyleId::Fill, "#dc2626"),
        ],
        &[(PropId::Id, "r")],
    );
    d.children(defs, &[filter, mask, rect]);
    let svg = d.node(
        Svg,
        &[(StyleId::Width, "100"), (StyleId::Height, "100")],
        &[(PropId::ViewBox, "0 0 100 100")],
    );
    let mut body = vec![defs];
    body.extend(place(&mut d));
    d.children(svg, &body);
    (d.kernel(&[svg], 400.0), svg)
}

fn near(a: (f32, f32, f32, f32), b: (f32, f32, f32, f32), what: &str) {
    let close = |x: f32, y: f32| (x - y).abs() < 1e-3;
    assert!(
        close(a.0, b.0) && close(a.1, b.1) && close(a.2, b.2) && close(a.3, b.3),
        "{what}: {a:?} != {b:?}"
    );
}

/// A `use` of `#r` with `rows`, inside a `g` when `grouped`, and the
/// element that carries `rows`.
fn use_of_r(d: &mut Doc, rows: &[(StyleId, &str)], grouped: bool) -> (Vec<u32>, u32) {
    if grouped {
        let u = d.node(NodeType::SvgUse, &[], &[(PropId::Href, "#r")]);
        let g = d.node(NodeType::SvgGroup, rows, &[]);
        d.children(g, &[u]);
        (vec![g], g)
    } else {
        let u = d.node(NodeType::SvgUse, rows, &[(PropId::Href, "#r")]);
        (vec![u], u)
    }
}

#[test]
fn a_use_and_its_group_take_the_box_of_what_they_instance() {
    for grouped in [false, true] {
        let what = if grouped { "a group of a use" } else { "a use" };
        let mut on = 0;
        let (k, svg) = scene(|d| {
            let (body, id) = use_of_r(d, &[(StyleId::Filter, "url(#f)")], grouped);
            on = id;
            body
        });
        let s = resolve(&k, svg);
        let f = find(&s.items, on)
            .unwrap()
            .filter
            .as_ref()
            .expect("a filter");
        // −10%/−10%/120%/120% of the box 10,10,20,10.
        near(f.region, (8.0, 9.0, 24.0, 12.0), what);
        assert_eq!(f.primitives.len(), 1, "{what}: the blur");

        let (k, svg) = scene(|d| {
            let (body, id) = use_of_r(d, &[(StyleId::Filter, "blur(1px)")], grouped);
            on = id;
            body
        });
        let s = resolve(&k, svg);
        let f = find(&s.items, on)
            .unwrap()
            .filter
            .as_ref()
            .expect("a filter");
        near(f.region, (7.0, 7.0, 26.0, 16.0), what);
        assert_eq!(f.primitives.len(), 1, "{what}: the blur");

        let (k, svg) = scene(|d| {
            let (body, id) = use_of_r(d, &[(StyleId::SvgMask, "url(#m)")], grouped);
            on = id;
            body
        });
        let s = resolve(&k, svg);
        let m = find(&s.items, on).unwrap().mask.as_ref().expect("a mask");
        near(m.region, (8.0, 9.0, 24.0, 12.0), what);
        assert!(!m.items.is_empty(), "{what}: the mask's content");
    }
}

#[test]
fn a_use_box_is_where_the_instance_draws() {
    // `x`/`y` move the instance and its box with it.
    let mut on = 0;
    let (k, svg) = scene(|d| {
        on = d.node(
            NodeType::SvgUse,
            &[
                (StyleId::X, "5"),
                (StyleId::Y, "20"),
                (StyleId::Filter, "url(#f)"),
            ],
            &[(PropId::Href, "#r")],
        );
        vec![on]
    });
    let s = resolve(&k, svg);
    let f = find(&s.items, on).unwrap().filter.as_ref().unwrap();
    near(f.region, (13.0, 29.0, 24.0, 12.0), "x 5, y 20");

    // `fill-box` turns an instance about its own middle: 90° about
    // (20, 15) puts the 20 × 10 rect at 15,5, 10 × 20.
    let (k, svg) = scene(|d| {
        on = d.node(
            NodeType::SvgUse,
            &[
                (StyleId::TransformBox, "fill-box"),
                (StyleId::TransformOrigin, "center"),
                (StyleId::Rotate, "90"),
            ],
            &[(PropId::Href, "#r")],
        );
        vec![on]
    });
    let s = resolve(&k, svg);
    let t = find(&s.items, on).unwrap().transform.expect("a rotation");
    assert_eq!(t.origin, (20.0, 15.0));
}

#[test]
fn a_filter_or_mask_on_text_has_a_region_that_holds_its_glyphs() {
    use NodeType::*;
    for rows in [
        (StyleId::Filter, "url(#f)"),
        (StyleId::Filter, "blur(1px)"),
        (StyleId::SvgMask, "url(#m)"),
    ] {
        let mut on = 0;
        let (k, svg) = scene(|d| {
            on = d.node(
                SvgText,
                &[(StyleId::FontSize, "10"), rows],
                &[
                    (PropId::TextX, "10"),
                    (PropId::TextY, "50"),
                    (PropId::Text, "Hi"),
                ],
            );
            vec![on]
        });
        let s = resolve(&k, svg);
        let item = find(&s.items, on).unwrap();
        let region = match (&item.filter, &item.mask) {
            (Some(f), _) => {
                assert!(!f.primitives.is_empty(), "{rows:?}: the filter draws");
                f.region
            }
            (_, Some(m)) => m.region,
            _ => panic!("{rows:?}: no filter or mask"),
        };
        // Chrome's box for "Hi" at 10px in its default sans-serif is about
        // 10,41, 11 × 12; the region holds at least that.
        let (x, y, w, h) = region;
        assert!(
            x <= 10.0 && y <= 41.0 && x + w >= 21.0 && y + h >= 53.0,
            "{rows:?}: {region:?}"
        );
    }
}
