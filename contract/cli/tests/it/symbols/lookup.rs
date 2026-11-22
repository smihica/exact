use super::{json_at, spelling, Fixture};
use serde_json::Value;
use std::{
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

fn lookup(path: &Path, filters: &[&str]) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_contract"))
        .args(["symbols", "--editor"])
        .arg(path)
        .args(filters)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn filtered_editor_symbols_batch_matches_with_authored_ranges() {
    let f = Fixture::new("element-lookup");
    let source = r#"component Editor
  state ready = true
  state rows = ["draft"]
  action save(row: string)
    ready = false
  view
    column
      for row, i in rows key=row
        if ready
          button "Save draft" press=save(row) testId="save"
            aria-label=(ready
              ? "Édit draft"
              : "Wait")
        else
          button press=save(row) testId="discard"
            text "Discard draft"
        button "Save elsewhere" press=save(row)
      button "Save draft" testId="no-handler"
component Other
  state saved = false
  action save
    saved = true
  view
    button "Save draft" press=save testId="other"
"#;
    let root = f.write("app.contract", source);
    let graph = lookup(
        &root,
        &[
            "--component",
            "Editor",
            "--tag",
            "button",
            "--has",
            "press",
            "--has",
            "testId",
            "--where",
            "press=save(row)",
            "--match",
            "Discard draft",
            "--match",
            "Save draft",
            "--match",
            "Édit draft",
        ],
    );
    let elements = graph["elements"].as_array().unwrap();
    assert!(graph.get("definitions").is_none());
    assert_eq!(elements.len(), 2, "{graph}");
    let save = &elements[0];
    assert_eq!(save["text"][0]["source"], "\"Save draft\"");
    assert_eq!(spelling(&json_at(save, &save["text"][0])), "\"Save draft\"");
    assert_eq!(save["attrs"][0]["binding"]["source"], "save(row)");
    assert_eq!(
        spelling(&json_at(save, &save["attrs"][0]["binding"])),
        "save(row)"
    );
    let label = &save["attrs"][2]["binding"];
    assert_eq!(
        label["source"],
        "(ready\n              ? \"Édit draft\"\n              : \"Wait\")"
    );
    assert_eq!(label["line"], 11);
    assert_eq!(label["end_line"], 13);
    assert_eq!(label["end_col"], 24);
    assert_eq!(save["context"][0]["source"], "for row, i in rows key=row");
    assert_eq!(
        spelling(&json_at(save, &save["context"][0])),
        "for row, i in rows key=row"
    );
    assert_eq!(save["context"][1]["source"], "if ready");
    assert_eq!(save["context"][1]["branch"], "then");
    let discard = &elements[1];
    assert_eq!(discard["context"][1]["branch"], "else");
    assert_eq!(
        spelling(&json_at(discard, &discard["text"][0])),
        "\"Discard draft\""
    );
    assert_eq!(std::fs::read_to_string(root).unwrap(), source);
}

#[test]
fn lookup_keeps_written_bindings_removed_by_module_processing() {
    let root =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../apps/geofence-fixture/app.contract");
    let graph = lookup(
        &root,
        &["--has", "geofenceenter", "--where", "geofenceenter=arrived"],
    );
    let elements = graph["elements"].as_array().unwrap();
    assert_eq!(elements.len(), 1, "{graph}");
    let main = &elements[0];
    assert_eq!(main["tag"], "main");
    let event = main["attrs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|attr| attr["name"] == "geofenceenter")
        .unwrap();
    assert_eq!(spelling(&json_at(main, event)), "geofenceenter");
    assert_eq!(spelling(&json_at(main, &event["binding"])), "arrived");
}

#[test]
fn lookup_finds_anatomy_parts_as_written_attributes() {
    let f = Fixture::new("lookup-parts");
    let root = f.write(
        "app.contract",
        r#"component App
  parts
    surface: control
    back: control
  view
    column
      button "Save" part="surface" testId="save"
      button "Cancel" part=("back")
      button "Help"
"#,
    );
    let graph = lookup(&root, &["--has", "part"]);
    let elements = graph["elements"].as_array().unwrap();
    assert_eq!(elements.len(), 2, "{graph}");
    for (i, filter) in ["part=\"surface\"", "part=(\"back\")"].iter().enumerate() {
        let graph = lookup(&root, &["--where", filter]);
        assert_eq!(graph["elements"], serde_json::json!([elements[i]]));
    }
    for (element, value) in elements.iter().zip(["surface", "back"]) {
        let part = &element["attrs"][0];
        assert_eq!(part["name"], "part");
        assert_eq!(spelling(&json_at(element, part)), "part");
        assert_eq!(part["string"]["value"], value);
        assert_eq!(
            spelling(&json_at(element, &part["string"])),
            format!("\"{value}\"")
        );
    }
}

#[test]
fn lookup_finds_bare_input_attributes_at_their_names() {
    let f = Fixture::new("lookup-input-flags");
    let root = f.write(
        "app.contract",
        r#"component App
  view
    column
      input type="checkbox" switch aria-label="Notifications"
      input type="file" multiple aria-label="Attachments"
      input type="file" aria-label="Avatar"
"#,
    );
    for name in ["switch", "multiple"] {
        let graph = lookup(&root, &["--has", name]);
        let elements = graph["elements"].as_array().unwrap();
        assert_eq!(elements.len(), 1, "{graph}");
        let element = &elements[0];
        let attr = &element["attrs"][1];
        assert_eq!(attr["name"], name);
        assert_eq!(spelling(&json_at(element, attr)), name);
        assert!(attr.get("binding").is_none());
    }
}

#[test]
fn lookup_uses_the_tags_positional_text() {
    let f = Fixture::new("positional-text");
    let root = f.write(
        "app.contract",
        r#"component App
  view
    column
      select
        option "Specification" value="spec"
      svg viewBox="0 0 100 100"
        text x=0 y=0
          tspan "+4.88%"
"#,
    );
    for (tag, label) in [("option", "Specification"), ("tspan", "+4.88%")] {
        let graph = lookup(&root, &["--tag", tag, "--match", label]);
        let elements = graph["elements"].as_array().unwrap();
        assert_eq!(elements.len(), 1, "{graph}");
        let element = &elements[0];
        assert_eq!(
            spelling(&json_at(element, &element["text"][0])),
            format!("\"{label}\"")
        );
    }
}

#[test]
fn lookup_orders_match_arms_as_written() {
    let f = Fixture::new("arm-order");
    let root = f.write(
        "app.contract",
        r#"component App
  state picked = some("Selected")
  view
    match picked
      case none
        text "Empty"
      case some(label)
        text label
"#,
    );
    let graph = lookup(&root, &["--tag", "text"]);
    let texts: Vec<_> = graph["elements"]
        .as_array()
        .unwrap()
        .iter()
        .map(|element| element["text"][0]["source"].as_str().unwrap())
        .collect();
    assert_eq!(texts, ["\"Empty\"", "label"]);
}

#[test]
fn lookup_unions_independent_filter_groups_in_source_order() {
    let f = Fixture::new("lookup-groups");
    let root = f.write(
        "app.contract",
        r#"component Editor
  view
    column
      button "Save" testId="save"
      button "Cancel"
component Preview
  view
    column
      button "Save"
      button "Cancel" testId="cancel"
"#,
    );
    let graph = lookup(
        &root,
        &[
            "--component",
            "Preview",
            "--tag",
            "button",
            "--match",
            "Cancel",
            "--or",
            "--component",
            "Editor",
            "--tag",
            "button",
            "--match",
            "Save",
            "--or",
            "--where",
            "testId=\"save\"",
        ],
    );
    let buttons: Vec<_> = graph["elements"]
        .as_array()
        .unwrap()
        .iter()
        .map(|element| {
            (
                element["component"].as_str().unwrap(),
                element["text"][0]["source"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(buttons, [("Editor", "\"Save\""), ("Preview", "\"Cancel\"")]);
}

#[test]
fn symbols_help_respects_flag_values() {
    let output = Command::new(env!("CARGO_BIN_EXE_contract"))
        .args(["symbols", "--editor", "--help"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    assert!(String::from_utf8_lossy(&output.stdout).contains("contract symbols app.contract"));

    let f = Fixture::new("lookup-help-value");
    let root = f.write(
        "app.contract",
        "component App\n  view\n    text \"--help\"\n",
    );
    let graph = lookup(&root, &["--match", "--help"]);
    assert_eq!(graph["elements"][0]["text"][0]["source"], "\"--help\"");
}

#[test]
fn lookup_uses_buffer_and_import_ranges_even_when_loading_refuses() {
    let f = Fixture::new("lookup-recovery");
    let root = f.write(
        "app.contract",
        "component App\n  view\n    text \"On disk\"\n",
    );
    let row = f.write(
        "row.contract",
        "component Row\n  view\n    button\n      text \"Imported\"\n",
    );
    let buffer = r#"use Row as Alias from "./row.contract"
use Missing from "./missing.contract"
component App
  view
    column
      button "In buffer"
      Alias()
"#;
    let mut child = Command::new(env!("CARGO_BIN_EXE_contract"))
        .args(["symbols", "--stdin", "--editor"])
        .arg(&root)
        .args(["--tag", "button"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(buffer.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    let graph: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(String::from_utf8_lossy(&output.stderr).contains("missing.contract"));
    assert!(graph["errors"].is_array());
    let elements = graph["elements"].as_array().unwrap();
    assert_eq!(elements.len(), 2, "{graph}");
    assert_eq!(elements[0]["file"], root.to_string_lossy().as_ref());
    assert_eq!(elements[0]["text"][0]["source"], "\"In buffer\"");
    assert_eq!(
        elements[1]["file"],
        row.canonicalize().unwrap().to_string_lossy().as_ref()
    );
    assert_eq!(elements[1]["component"], "Row");
    assert_eq!(
        spelling(&json_at(&elements[1], &elements[1]["text"][0])),
        "\"Imported\""
    );
}
