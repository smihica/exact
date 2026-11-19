//! LLP 1030.004 D2: a durable write announces its topic for the app's other
//! sessions, and a read in an answer watches it.
#![cfg(exact_js_engine)]
use super::storage::{call, Root, GRANTS};
use exact_runner::{DataSource, Store};

#[test]
fn a_write_announces_its_topics_and_a_read_watches_them() {
    let root = Root::new();
    let mut m = root.module();
    let mut store = Store::new(GRANTS, vec![]);
    m.activate().unwrap();
    let note = ["app:/data/note".to_string(), "app:/data".into()];

    // `file` writes the note, then reads it back.
    call(&mut m, &mut store, "file", "saved");
    assert_eq!(
        store.take_announced(),
        note,
        "the file and the directory that lists it"
    );
    store.take_topics();
    call(&mut m, &mut store, "read", "");
    assert_eq!(store.take_topics(), note);
    assert!(
        store.take_announced().is_empty(),
        "a read announces nothing"
    );

    call(&mut m, &mut store, "add", "row");
    assert_eq!(store.take_announced(), ["sqlite:app:/data/notes.db"]);
    store.take_topics();
    call(&mut m, &mut store, "types", "");
    assert_eq!(store.take_topics(), ["sqlite:app:/data/notes.db"]);
    assert!(
        store.take_announced().is_empty(),
        "a CREATE TABLE IF NOT EXISTS changes no rows, so two windows reading do not re-ask each other"
    );
}

/// A statement announces when it changed the database, a row or the schema,
/// and its `changes` count only its own rows.
#[test]
fn a_statement_announces_when_it_changes_a_row_or_the_schema() {
    let root = Root::new();
    let mut m = root.module();
    let mut store = Store::new(GRANTS, vec![]);
    m.activate().unwrap();
    let mut run = |op: &str, sql: &str| {
        let counted = call(&mut m, &mut store, op, sql);
        let announced = store.take_announced();
        assert!(
            announced.is_empty() || announced == ["sqlite:app:/data/notes.db"],
            "{announced:?}"
        );
        let counted: Vec<_> = counted
            .split("; ")
            .map(|r| {
                let (changes, keys) = r.split_once(' ').unwrap();
                assert_eq!(keys, "changes,lastInsertRowid", "the app's keys");
                changes.to_string()
            })
            .collect();
        (counted.join(" "), !announced.is_empty())
    };
    let execute = "kept-execute";
    for (sql, expected) in [
        ("CREATE TABLE IF NOT EXISTS kept (body TEXT)", ("0", true)),
        ("INSERT INTO kept VALUES ('a')", ("1", true)),
        // `sqlite3_changes` would still say 1 here.
        ("CREATE TABLE IF NOT EXISTS kept (body TEXT)", ("0", false)),
        ("INSERT INTO kept VALUES ('b')", ("1", true)),
        (
            "UPDATE kept SET body = 'c' WHERE body = 'none'",
            ("0", false),
        ),
        ("DELETE FROM kept WHERE body = 'none'", ("0", false)),
        ("ALTER TABLE kept ADD COLUMN at INTEGER", ("0", true)),
        ("DROP TABLE kept", ("0", true)),
    ] {
        let (changes, announced) = run(execute, sql);
        assert_eq!((changes.as_str(), announced), expected, "{sql}");
    }
    let batch = "CREATE TABLE IF NOT EXISTS notes (body TEXT);INSERT INTO notes VALUES ('kept')";
    assert_eq!(run("kept-batch", batch), ("0 1".into(), true));
    let batch =
        "CREATE TABLE IF NOT EXISTS notes (body TEXT);DELETE FROM notes WHERE body = 'none'";
    assert_eq!(run("kept-batch", batch), ("0 0".into(), false));
}
