//! Shared by the art baker and generated game build scripts.
use std::path::Path;

pub(super) fn write_changed(path: &Path, bytes: &[u8]) -> Result<(), String> {
    use std::io::Write;
    if std::fs::read(path).ok().as_deref() == Some(bytes) {
        return Ok(());
    }
    let tmp = path.with_file_name(format!(
        ".{}.{}.tmp",
        path.file_name().unwrap().to_string_lossy(),
        std::process::id()
    ));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&tmp)
        .map_err(|e| format!("{}: {e}", tmp.display()))?;
    let result = file
        .write_all(bytes)
        .and_then(|()| std::fs::rename(&tmp, path));
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result.map_err(|e| format!("{}: {e}", path.display()))
}

/// Validate all tuning files together and publish one indivisible asset transaction.
pub fn bake_game_tables<G: exact_game::Game>(app: impl AsRef<Path>) -> Result<(), String> {
    if G::TABLES.is_empty() {
        return Ok(());
    }
    let mut rows = std::collections::BTreeMap::new();
    for table in G::TABLES {
        let path = app.as_ref().join(table.name);
        println!("cargo:rerun-if-changed={}", path.display());
        let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        check_size(table.name, bytes.len())?;
        let text = String::from_utf8(bytes).map_err(|e| format!("{}: {e}", table.name))?;
        rows.insert(table.name.to_owned(), text);
    }
    let text = exact_game::json::to_string(&rows).map_err(|e| e.to_string())?;
    exact_game::asset::validate_tables::<G>(&text).map_err(|e| e.to_string())?;
    let output = app
        .as_ref()
        .join("assets")
        .join(exact_game::asset::TABLE_ASSET);
    std::fs::create_dir_all(output.parent().unwrap()).map_err(|e| e.to_string())?;
    write_changed(&output, text.as_bytes())
}

/// Refuse oversized carriers before decoding or writing them.
pub fn check_size(name: &str, size: usize) -> Result<(), String> {
    if size > 64 * 1024 * 1024 {
        Err(format!("asset `{name}` exceeds 64 MiB"))
    } else {
        Ok(())
    }
}

/// Validate the game's single declared JSON level using its `Data` derive, then
/// deliver it separately from executable code. No source bytes enter the module.
pub fn bake_game_level<G: exact_game::Game>(app: impl AsRef<Path>) -> Result<(), String> {
    let receipt = app.as_ref().join(".shells/level.json");
    let Some(level) = G::LEVEL else {
        return match std::fs::remove_file(&receipt) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(format!("{}: {e}", receipt.display())),
        };
    };
    let source = app.as_ref().join(level.name);
    println!("cargo:rerun-if-changed={}", source.display());
    let bytes = std::fs::read(&source).map_err(|e| format!("{}: {e}", source.display()))?;
    check_size(level.name, bytes.len())?;
    let text = std::str::from_utf8(&bytes).map_err(|e| format!("{}: {e}", level.name))?;
    level.decode(text).map_err(|e| e.to_string())?;
    let output = app.as_ref().join("assets").join(level.name);
    std::fs::create_dir_all(output.parent().unwrap()).map_err(|e| e.to_string())?;
    write_changed(&output, &bytes)?;
    // Input inventories must distinguish this output from handwritten assets.
    let name = exact_game::json::to_string(&level.name.to_owned()).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(receipt.parent().unwrap()).map_err(|e| e.to_string())?;
    write_changed(&receipt, name.as_bytes())
}
