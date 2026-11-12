//! `bundle:/` as the iOS and tvOS host starts it (exact2 LLP 1133.007 D6):
//! the executor, started as that process, roots it at the folder of
//! declared trees beside the executable, and names none for a widget
//! extension; nothing is named for it here, and `home:` is absent.
//!
//! A binary of its own: the roots are the process's, and the `it` binary's
//! bridges start Mac executors that name the Mac's. Unix only, as `bundle:`
//! paths are (ibex2 `run_namespaced`).
#![cfg(unix)]
use exact_apple::executor::{Executor, Launch};
use ibex2::grant::GrantSet;
use ibex2::stdlib::fs::{run, FsOp, FsResult};

/// What `storage.fs.readFile(path)` answers under `grants` (the door's
/// operation, past the TypeScript prelude).
fn read(grants: &str, path: &str) -> String {
    let grants = GrantSet::parse(grants).unwrap();
    match run(&grants, None, FsOp::ReadFile, path, None, None) {
        Ok(FsResult::Bytes(bytes)) => String::from_utf8(bytes).unwrap(),
        other => format!("{other:?}"),
    }
}

fn start_in(executable: &std::path::Path) -> Executor {
    let launch = Launch {
        processes: false,
        executable: Some(executable.to_path_buf()),
    };
    Executor::start_as(None, "", None, launch)
}

#[test]
fn an_ios_executor_roots_bundle_at_the_declared_trees() {
    let app = std::env::temp_dir().join(format!(
        "exact-bundle-root-{}/Shipped.app",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&app);
    let file = app.join("ExactResources/Resources/shipped/hello.txt");
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&file, "shipped").unwrap();
    std::fs::write(app.join("Info.plist"), "plist").unwrap();
    let grants = "fs.read bundle:/Resources/shipped\nfs.read bundle:/Info.plist\nfs.read home:/.t3";
    let _app = start_in(&app.join("Shipped"));
    assert_eq!(
        read(grants, "bundle:/Resources/shipped/hello.txt"),
        "shipped"
    );
    // The app's own files are beside the root.
    let beside = read(grants, "bundle:/Info.plist");
    assert!(beside.contains("os error 2"), "{beside}");
    let home = read(grants, "home:/.t3/x");
    assert!(
        home.contains("unsupported: no home: directory here"),
        "{home}"
    );
    // A widget extension has no folder of its own and reads none of the
    // app's: its refresh reads `app:` only (exact-js's snapshot tests).
    let widget = app.join("PlugIns/ExactWidgets.appex/ExactWidgets");
    std::fs::create_dir_all(widget.parent().unwrap()).unwrap();
    let _widget = start_in(&widget);
    let refused = read(grants, "bundle:/Resources/shipped/hello.txt");
    assert!(
        refused.contains("unsupported: no bundle: directory here"),
        "{refused}"
    );
    let _app = start_in(&app.join("Shipped"));
    // An app that declared nothing has no bundle: directory.
    std::fs::remove_dir_all(app.join("ExactResources")).unwrap();
    let _bare = start_in(&app.join("Shipped"));
    let none = read(grants, "bundle:/Resources/shipped/hello.txt");
    assert!(
        none.contains("unsupported: no bundle: directory here"),
        "{none}"
    );
    std::fs::remove_dir_all(app.parent().unwrap()).unwrap();
}
