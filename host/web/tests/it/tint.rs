//! A tint (LLP 1011 §3, LLP 1062 D6): a raster's alpha masking the
//! registered `--exact-tint`, which the browser also transitions.

use exact_runner::Event;
use exact_web::Host;

fn view_with_test_id(host: &Host<caltrain_data::Caltrain>, test_id: &str) -> u32 {
    let k = host.runner().kernel();
    let key = k.find_by_test_id(test_id)[0];
    k.node_by_key(key).unwrap().id
}

#[test]
fn video_fit_keeps_the_ua_default_separate_from_authored_clears() {
    for (declaration, expected) in [
        ("", "contain"),
        (r#"object-fit="unset""#, "fill"),
        (r#"object-fit="""#, "fill"),
        (r#"object-fit="cover""#, "cover"),
    ] {
        let source =
            format!("component App\n  view\n    video width=160 height=80 {declaration}\n");
        let plan = contract::compile(&source).unwrap();
        let (host, _) = Host::boot(&plan.encode(), (), Default::default(), "/").unwrap();
        let html = host.document().unwrap().root;
        assert!(
            html.contains(&format!("object-fit:{expected};")),
            "{declaration}: {html}"
        );
    }
}

#[test]
fn a_tinted_raster_is_its_alpha_masking_the_tint() {
    let plan = contract::compile(
        r##"component App
  state wide = false
  action toggle
    wide = not wide
  view
    column
      button "Toggle" press=toggle testId="toggle"
      image "assets/mark.png" testId="mark" width=80 height=40 object-fit=(wide ? "cover" : "contain") object-position=(wide ? "right bottom" : "7px 11px") -exact-tint-color="light-dark(#000000, #ffffff)"
      image "assets/mark.png" testId="down" object-fit="scale-down" -exact-tint-color="#ff0000" transition="-exact-tint-color 200ms linear"
      image "assets/mark.png" testId="plain" object-fit="cover"
      image "symbol:search" testId="symbol" -exact-tint-color="#ff0000"
"##,
    )
    .unwrap();
    let (mut host, first) = Host::boot(
        &plan.encode(),
        caltrain_data::Caltrain,
        Default::default(),
        "/",
    )
    .unwrap();
    let create = |test_id: &str| {
        let id = view_with_test_id(&host, test_id);
        let at = first.find(&format!("\"id\":{id},\"tag\":\"img\"")).unwrap();
        first[at..].split("\"op\":").next().unwrap().to_string()
    };
    let mark = create("mark");
    assert!(mark.contains("--exact-tint:light-dark("), "{mark}");
    assert!(
        mark.contains("background-color:var(--exact-tint);mask-image:url(\\\"assets/mark.png\\\");mask-size:var(--exact-tint-size);"),
        "{mark}"
    );
    assert!(
        mark.contains("mask-position:var(--exact-object-position,50% 50%);mask-origin:content-box;mask-clip:content-box;--exact-tint-offset:-100000px 0;object-position:var(--exact-tint-offset,var(--exact-object-position));"),
        "{mark}"
    );
    assert!(mark.contains("--exact-object-position:7px 11px;"), "{mark}");
    assert!(mark.contains("--exact-tint-size:contain;"), "{mark}");
    assert!(
        mark.contains("\"src\":\"assets/mark.png\""),
        "the page still loads the picture: {mark}"
    );
    let down = create("down");
    assert!(down.contains("--exact-tint-size:var(--exact-tint-fit,contain);"));
    assert!(
        down.contains("transition:--exact-tint 0.2s linear 0s;"),
        "{down}"
    );
    assert!(!create("plain").contains("mask-image"));
    assert!(
        !create("symbol").contains("mask-image:url("),
        "a symbol's mask is the glue's"
    );
    let wide = host.dispatch(view_with_test_id(&host, "toggle"), Event::Press);
    assert!(
        wide.contains("--exact-tint-size:cover;"),
        "the mask follows object-fit: {wide}"
    );
    assert!(
        wide.contains("--exact-object-position:100% 100%;"),
        "{wide}"
    );
}

#[test]
fn untinted_rasters_with_box_paint_stay_bare_in_batches_and_documents() {
    exact_web::link(exact_web_capabilities::linked!(shadows));
    let plan = contract::compile(
        r##"component App
  view
    row
      image "assets/mark.png" testId="natural" aria-label="Mark" background-color="#ffeedd"
      image "assets/mark.png" testId="auto" height=24 border-width=3 border-style="solid" border-color="#00aa00"
      image "assets/mark.png" testId="sized" width=80 height=48 padding=6 box-shadow="2px 3px 4px #000000"
"##,
    )
    .unwrap();
    let (host, first) = Host::boot(
        &plan.encode(),
        caltrain_data::Caltrain,
        Default::default(),
        "/",
    )
    .unwrap();
    let doc = host.document().unwrap().root;
    // A host wrapper would replace the image's intrinsic sizing, including
    // its auto axis and its flex-item sizing. Both projections stay bare.
    assert_eq!(first.matches("\"op\":\"create\"").count(), 4);
    assert_eq!(doc.matches('<').count(), 5); // row, three void images, /row
    for name in ["natural", "auto", "sized"] {
        let id = view_with_test_id(&host, name);
        assert!(first.contains(&format!("\"id\":{id},\"tag\":\"img\"")));
        let at = doc.find(&format!(" data-view=\"{id}\"")).unwrap();
        let start = doc[..at].rfind('<').unwrap();
        let end = at + doc[at..].find('>').unwrap();
        let image = &doc[start..end];
        assert!(image.starts_with("<img "), "{image}");
        assert!(image.contains("src=\"assets/mark.png\""), "{image}");
        assert!(!image.contains("mask-image"), "{image}");
        assert!(!image.contains("object-position"), "{image}");
        let css = image
            .split("style=\"")
            .nth(1)
            .unwrap()
            .split('"')
            .next()
            .unwrap();
        if name != "sized" {
            assert!(!css.split(';').any(|d| d.starts_with("width:")), "{image}");
        }
        if name == "natural" {
            assert!(!css.split(';').any(|d| d.starts_with("height:")), "{image}");
            assert!(image.contains("alt=\"Mark\""), "{image}");
        }
    }
}

/// LLP 1116.001 D2: `AccentColor` (and `LinkText`) is the nearest authored
/// `accent-color`, as UIKit's inherited tint: the page reads the
/// `--exact-accent` the accent writes, else the browser's own accent. A link
/// is the accent with none authored too (LLP 1115.000 D3.5): one blue for
/// links, where the browser's own `LinkText` was a second.
#[test]
fn the_accent_colour_role_is_the_nearest_accent_color() {
    let plan = contract::compile(
        r##"component App
  view
    column testId="root" accent-color="-exact-system-teal"
      view testId="fill" height=8 accent-color="AccentColor" background-color="AccentColor"
      text "Link" testId="ink" color="LinkText"
      text "Plain" testId="plain" color="CanvasText"
"##,
    )
    .unwrap();
    let (host, first) = Host::boot(
        &plan.encode(),
        caltrain_data::Caltrain,
        Default::default(),
        "/",
    )
    .unwrap();
    let create = |test_id: &str| {
        let id = view_with_test_id(&host, test_id);
        let at = first.find(&format!("\"id\":{id},")).unwrap();
        first[at..].split("\"op\":").next().unwrap().to_string()
    };
    let root = create("root");
    assert!(
        root.contains("--exact-accent:var(--exact-system-teal,"),
        "{root}"
    );
    let fill = create("fill");
    assert!(
        fill.contains("accent-color:var(--exact-accent, AccentColor);")
            && !fill.contains("--exact-accent:"),
        "the accent role must inherit without a custom-property cycle: {fill}"
    );
    assert!(
        fill.contains("background-color:var(--exact-accent, AccentColor);"),
        "{fill}"
    );
    let ink = create("ink");
    assert!(
        ink.contains("color:var(--exact-linktext, var(--exact-accent, AccentColor));"),
        "{ink}"
    );
    let plain = create("plain");
    assert!(plain.contains("color:CanvasText;"), "{plain}");
}

/// theme2 2026-10-23 (after `lower-symbol-tint`): a symbol's own `-exact-tint-color` is no paint of it on the web
/// either: lowering answers it with `currentcolor`, which the glue's mask paints in the symbol's `color`, as Apple
/// draws it. A native button's face keeps its tint (LLP 1115 D4), as a raster does.
#[test]
fn a_symbols_own_tint_is_painted_in_its_colour_and_a_native_face_keeps_its_tint() {
    let plan = contract::compile(
        r##"component App
  action go
  view
    column color="#0000ff"
      image "symbol:search" testId="symbol" -exact-tint-color="#ff0000"
      button press=go testId="lock"
        image "symbol:lock" testId="face" -exact-tint-color="#ff0000"
        text "Lock"
      image "assets/mark.png" testId="raster" -exact-tint-color="#ff0000"
"##,
    )
    .unwrap();
    let (host, first) = Host::boot(
        &plan.encode(),
        caltrain_data::Caltrain,
        Default::default(),
        "/",
    )
    .unwrap();
    let create = |test_id: &str| {
        let id = view_with_test_id(&host, test_id);
        let at = first.find(&format!("\"id\":{id},\"tag\":\"img\"")).unwrap();
        first[at..].split("\"op\":").next().unwrap().to_string()
    };
    let symbol = create("symbol");
    assert!(symbol.contains("--exact-tint:currentcolor"), "{symbol}");
    assert!(
        !symbol.contains("#ff0000") && !symbol.contains("255, 0, 0"),
        "{symbol}"
    );
    for kept in ["face", "raster"] {
        let made = create(kept);
        assert!(
            !made.contains("--exact-tint:currentcolor"),
            "{kept}: {made}"
        );
        assert!(
            made.contains("--exact-tint:rgba(255,0,0,1)"),
            "{kept}: {made}"
        );
    }
}

#[test]
fn image_positions_use_the_nodes_own_value_and_the_browser_environment() {
    let positions = [
        "var(--missing) 75%",
        "var(--missing, bogus) 75%",
        "var(--missing, var(--other, bogus)) 75%",
        "var(--missing, 7px 11px)",
        "var(--shift, bogus) 75%",
    ];
    let svg = "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='40' height='20'%3E%3Cpath d='M0 0H40V20H0Z' fill='red'/%3E%3C/svg%3E";
    let mut source = String::from("component App\n");
    for (i, position) in positions.iter().enumerate() {
        source.push_str(&format!("  state p{i} = \"{position}\"\n"));
    }
    source.push_str("  view\n    column --shift=\"7px\" object-position=\"37px 23px\"\n");
    for i in 0..positions.len() {
        source.push_str(&format!(r##"      image "{svg}" width=160 height=80 object-fit="none" object-position=p{i} -exact-tint-color="#ff0000"
"##));
    }
    for attrs in [
        r##"object-position="env(-exact-layout-margins-inset-left, 90px) top" -exact-tint-color="#ff0000""##,
        r#"object-position="env(-exact-layout-margins-inset-left, 90px) top""#,
        "",
    ] {
        source.push_str(&format!(
            r#"      image "{svg}" width=160 height=80 object-fit="none" {attrs}
"#
        ));
    }
    let plan = contract::compile(&source).unwrap();
    let (host, _) = Host::boot(&plan.encode(), (), Default::default(), "/").unwrap();
    let html = host.document().unwrap().root;
    let script = r#"
import { strict as assert } from 'node:assert';
import { suiteBrowser } from './scripts/agent-launch.mjs';
import { drivable } from './scripts/coverage.mjs';
import { decodePng } from './scripts/png.mjs';
if (!(await drivable(['chromium'])).length) process.exit(0);
const browser = await suiteBrowser('chromium');
try {
  const page = await browser.newPage({ deviceScaleFactor: 1 });
  await page.setContent('<style>body{background:white}img{display:block}</style>' + process.argv[1]);
  await page.waitForFunction(() => [...document.images].every(image => image.complete));
  const expected = [[60, 30, 99, 49], [60, 30, 99, 49], [60, 30, 99, 49], [7, 11, 46, 30],
                    [7, 45, 46, 64], [0, 0, 39, 19], [0, 0, 39, 19], [60, 30, 99, 49]];
  for (const [i, wanted] of expected.entries()) {
    const image = decodePng(await page.locator('img').nth(i).screenshot());
    let bounds = [160, 80, -1, -1];
    for (let y = 0; y < 80; y++) for (let x = 0; x < 160; x++) {
      const at = (y * 160 + x) * 4;
      if (image.data[at] - image.data[at + 1] > 128) {
        bounds = [Math.min(bounds[0], x), Math.min(bounds[1], y),
                  Math.max(bounds[2], x), Math.max(bounds[3], y)];
      }
    }
    assert.deepEqual(bounds, wanted, `position ${i}`);
  }
} finally { await browser.close(); await browser.browser.close(); }
"#;
    let output = std::process::Command::new("bun")
        .args(["-e", script, &html])
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    eprint!("{}", String::from_utf8_lossy(&output.stderr));
}
