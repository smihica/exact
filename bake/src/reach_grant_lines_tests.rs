use super::{passkey_rp_ids, payment_lines, sign_in};

/// Every line `exact add` advises for each package's grants (`grantLines`
/// in scripts/grant-advice.mjs), and for a `net.fetch` and an `fs.read` grant,
/// placeholders filled, parses as the app's grants do: the runner's whole
/// set, the device table, and the sign-in, passkey and payment grammars.
/// A line in another namespace's form (`device.auth.apple purpose.<key>`)
/// would fail the device table.
#[test]
fn grant_lines_parse() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let script = r#"
        import { readdirSync, readFileSync } from 'node:fs';
        import { grantLines } from './scripts/grant-advice.mjs';
        const all = readdirSync('packages').flatMap((p) => { try { return [[p, JSON.parse(readFileSync(`packages/${p}/exact.json`, 'utf8')).grants ?? []]]; } catch { return []; } });
        all.push(['(net.fetch)', ['net.fetch']], ['(fs.read)', ['fs.read']]);
        console.log(JSON.stringify(all.map(([p, g]) => [p, grantLines(g)])));
    "#;
    let out = std::process::Command::new("bun")
        .current_dir(&root)
        .args(["--eval", script])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let packages: Vec<(String, Vec<Vec<String>>)> = serde_json::from_slice(&out.stdout).unwrap();
    let filled = |line: &str| {
        let target = match line.split_whitespace().next() {
            Some("net.fetch") => "https://example.com",
            Some("fs.read") => "/data",
            _ => "",
        };
        line.replace("<key>", "k")
            .replace("<client id>", "com.example.app")
            .replace("<rp id>", "example.com")
            .replace("<merchant id>", "merchant.com.example.app")
            .replace("<its target>", target)
    };
    let mut checked = 0;
    for (package, grants) in &packages {
        for lines in grants {
            let spec = lines
                .iter()
                .map(|l| filled(l))
                .collect::<Vec<_>>()
                .join("\n");
            assert!(
                !spec.contains('<'),
                "{package}: `{spec}` has a placeholder this test does not fill"
            );
            let why = exact_runner::grants::parse(&spec)
                .err()
                .map(|e| e.join("; "))
                .or_else(|| {
                    exact_runner::device::device_grants(&spec)
                        .err()
                        .map(|e| e.join("; "))
                })
                .or_else(|| sign_in(&spec, "ios").err())
                .or_else(|| passkey_rp_ids(&spec).err())
                .or_else(|| payment_lines(&spec).err());
            assert!(
                why.is_none(),
                "{package}: `{spec}`: {}",
                why.unwrap_or_default()
            );
            checked += 1;
        }
    }
    let names = |p: &str| packages.iter().any(|(n, _)| n == p);
    for p in [
        "sign-in",
        "payments",
        "push",
        "notifications",
        "(net.fetch)",
        "(fs.read)",
    ] {
        assert!(names(p), "{p} was checked");
    }
    assert!(checked > 30, "{checked} lines");
    // The form the advice must not take: the device table refuses it.
    assert!(exact_runner::device::device_grants("device.auth.apple purpose.k").is_err());
}
