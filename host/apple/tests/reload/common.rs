//! What the reload tests share: a source with no data, a plan of one view
//! or of one action, a boot and the running plan's digest.

use exact_apple::abi::{Bridge, Hooks};

pub struct NoData;

impl exact_runner::DataSource for NoData {
    fn query(
        &mut self,
        name: &str,
        _: &[exact_plan::Value],
    ) -> Result<exact_plan::Value, exact_runner::DataError> {
        Err(exact_runner::DataError::UnknownSource(name.into()))
    }
}

pub fn plan(view: &str) -> Vec<u8> {
    source(&format!("component A\n  view\n    {view}\n"))
}

pub fn source(text: &str) -> Vec<u8> {
    contract::compile(text).unwrap().encode()
}

/// A plan whose box plays a CSS animation, which only the animations family
/// plays.
pub const ANIMATED: &str = "keyframes spin\n  from opacity=0\n  to opacity=1\ncomponent A\n  view\n    text \"a\" animation=\"spin 1s infinite\"\n";

/// A plan with a tool, which only the tool door and the runner's tool seam serve.
pub const TOOLS: &str = "component A\n  state n = 0\n  tool bump writes \"Add one.\" = bump\n  action bump\n    n = n + 1\n  view\n    text \"a\"\n";

/// A plan that can remove a content index entry, which only the content index
/// serves.
pub const SEARCH: &str = "shape ID\n  id: string\ncomponent A\n  mutation removed as shape ID\n  action remove\n    send removed = deleteContentIndexEntry(\"k\")\n  view\n    text \"a\"\n";

/// A plan that can start a Live Activity, which only the activities family
/// serves; it reads nothing at boot, so its candidate is not held for data.
pub const ACTIVITIES: &str = "shape ID\n  id: string\nshape S\n  n: number\ncomponent A\n  mutation started as shape ID\n  action start\n    send started = startActivity(\"k\", S(n=1))\n  view\n    text \"a\"\n";

/// A plan whose button starts a trace, which only the traces family serves.
pub const TRACING: &str =
    "component A\n  action go\n    startTrace_EXPERIMENTAL()\n  view\n    button \"b\" press=go\n";

/// A plan whose button's press runs `act`.
pub fn acted(act: &str) -> Vec<u8> {
    contract::compile(&format!(
        "component A\n  action go\n    {act}\n  view\n    button \"b\" press=go\n"
    ))
    .unwrap()
    .encode()
}

/// The media family's four products, a plan for each (LLP 1047.001 D10).
pub fn media() -> [(Vec<u8>, &'static str); 4] {
    [
        (plan("video src=\"assets/a.mp4\""), "video"),
        (plan("iframe src=\"https://example.test\""), "web_views"),
        (acted("stopSounds()"), "sound"),
        (acted("requestMIDIAccess()"), "midi"),
    ]
}

/// A widget's entry plan: only the widgets family boots one (LLP 1126 D2).
pub fn entry() -> Vec<u8> {
    let widget = contract::Widget {
        kind: "card".into(),
        title: None,
        description: None,
        families: vec!["systemSmall".into()],
        watch_families: vec![],
        component: "Card".into(),
        props: "Props".into(),
        refresh: "refresh".into(),
        requires_secrets: false,
        preview: serde_json::json!({"title": "preview"}),
        configuration: None,
    };
    let card = "shape Props\n  title: string\ncomponent Card\n  props\n    title: string\n  view\n    text title\n";
    contract::snapshot::compile(card, &widget).unwrap().encode()
}

/// Boot `entry` as the headless session does, and its reply.
pub fn boot_entry(entry: &[u8]) -> String {
    let mut bridge = Bridge::<NoData>::new();
    bridge.input_write(&[entry, b"card"].concat());
    let out = bridge.boot_entry(entry.len(), 4, 0, 0, NoData, Hooks::none(), 158.0, 158.0);
    String::from_utf8_lossy(bridge.output_bytes(out as usize)).into_owned()
}

pub fn boot(bridge: &mut Bridge<NoData>, plan: &[u8]) {
    let out = bridge.boot(plan, NoData, Hooks::none(), 390.0, 844.0);
    let reply = String::from_utf8_lossy(bridge.output_bytes(out as usize)).into_owned();
    assert!(!reply.contains("\"error\":\""), "{reply}");
}

pub fn digest(bridge: &mut Bridge<NoData>) -> String {
    let out = bridge.plan_digest();
    String::from_utf8_lossy(bridge.output_bytes(out as usize)).into_owned()
}
