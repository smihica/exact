//! A received share at the bridge (LLP 1133.024 D2–D4): claimed by the
//! running app, or before the boot, whose location and tokens it becomes.
use super::*;
use exact_runner::share_in::{Claims, Pending, ShareTarget};

impl<D: DataSource> Bridge<D> {
    /// `exact_share`: the JSON in the input buffer, `{"root", "target",
    /// "share", "inbox"?}` (`inbox`: the directory a `share.json` share's
    /// files wait in, moved from there). Running, a batch with the location
    /// delivered at `root`; before the boot, the launch location and the
    /// tokens are set for it and an empty batch answers.
    pub fn share(&mut self, len: usize, now_ms: f64) -> u32 {
        let request: serde_json::Value =
            serde_json::from_slice(&self.input[..len.min(self.input.len())]).unwrap_or_default();
        let inbox = request["inbox"].as_str().map(std::path::PathBuf::from);
        let out = match self.host.as_mut() {
            Some(h) => {
                if inbox.is_some() && self.cold_share.is_some() {
                    "{\"ops\":[],\"timers\":false,\"motion\":false,\"error\":\"share: a previous share is still waiting to land\"}"
                        .to_string()
                } else {
                    let (json, unlanded) = h.receive_share(&request, inbox.as_deref(), now_ms);
                    if let Some(pending) = unlanded {
                        self.cold_share = Some(pending);
                    }
                    json
                }
            }
            None => {
                let outcome: Result<(exact_runner::share_in::Claimed, bool), String> =
                    match ShareTarget::from_json(&request["target"])
                        .zip(Pending::from_json(&request["share"], inbox.as_deref()))
                    {
                        None => Err("share: invalid: unsupported or malformed share record".into()),
                        Some((target, share)) => {
                            let seed = self.launch_facts.place.as_ref().map_or(0.0, |p| p.seed);
                            let mut claims = Claims::new(seed);
                            let claimed = if inbox.is_some() {
                                claims.claim_checked(&target, share)
                            } else {
                                Ok(claims.claim(&target, share))
                            };
                            claimed.map(|mut claimed| {
                                claimed.inbox = inbox.clone();
                                claimed.inbox_inode =
                                    crate::host::share_in::parse_inode(&request["inboxInode"]);
                                self.launch = Some(claimed.location.clone());
                                self.launch_facts.shared = claimed.tokens.clone();
                                self.launch_facts.claims = Some(claims);
                                (claimed, inbox.is_some())
                            })
                        }
                    };
                match outcome {
                    Ok(claimed) => {
                        self.cold_share = Some(claimed);
                        "{\"ops\":[],\"timers\":false,\"motion\":false}".to_string()
                    }
                    Err(e) => {
                        self.cold_share = None;
                        serde_json::json!({
                            "ops": [],
                            "timers": false,
                            "motion": false,
                            "error": e,
                        })
                        .to_string()
                    }
                }
            }
        };
        self.emit(out)
    }

    /// `exact_share_land`: a share claimed before the boot lands once
    /// storage is configured, a batch (an error when no cold share was claimed).
    /// Keeps `cold_share` intact when landing fails so a retry lands against
    /// the original cold delivery without re-navigating.
    pub fn share_land(&mut self) -> u32 {
        let out = match (self.host.as_mut(), self.cold_share.as_mut()) {
            (Some(h), Some((claimed, from_inbox))) => {
                let (json, ok) = h.land_share(claimed, *from_inbox);
                if ok {
                    self.cold_share = None;
                    self.launch_facts.shared.clear();
                    self.launch_facts.claims = None;
                }
                json
            }
            (None, _) => {
                "{\"ops\":[],\"timers\":false,\"motion\":false,\"error\":\"share: host not booted\"}"
                    .to_string()
            }
            (Some(_), None) => {
                "{\"ops\":[],\"timers\":false,\"motion\":false,\"error\":\"share: no cold share claimed\"}"
                    .to_string()
            }
        };
        self.emit(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use exact_runner::{DataError, DataSource, Value};

    struct NoData;
    impl DataSource for NoData {
        fn query(&mut self, source: &str, _: &[Value]) -> Result<Value, DataError> {
            Err(DataError::UnknownSource(source.into()))
        }
    }

    const APP: &str = r#"routes nav
  home "/"
  share "/share"

component App
  state navCount = 0
  derive cur = top(nav)
  derive shared = sharedFiles(cur, "files")
  action go(loc: string)
    navCount = navCount + 1
    nav = go(nav, loc)
  action back
    nav = back(nav)
  view
    main navigationKey=`${cur.id}` navigationBack="back" navigate=go testId="root"
      text `${cur.name}:${navCount}:${shared.files.length}:${shared.pending}:${searchParam(cur, "text")}` testId="status"
"#;

    const CHANGE_APP: &str = r#"routes nav
  home "/"
  share "/share"

component App
  state armed = false
  state seen = 0
  derive cur = top(nav)
  action arm
    armed = true
  action onArm
    seen = seen + 1
  action go(loc: string)
    nav = go(nav, loc)
  action back
    nav = back(nav)
  task watch when armed key=armed
    change(onArm)
  view
    main navigationKey=`${cur.id}` navigationBack="back" navigate=go testId="root"
      button "Arm" press=arm testId="arm"
      text `${cur.name}:${seen}` testId="status"
"#;

    fn call_share(
        bridge: &mut Bridge<NoData>,
        req: &serde_json::Value,
        now_ms: f64,
    ) -> serde_json::Value {
        let bytes = serde_json::to_vec(req).unwrap();
        let len = bridge.input_write(&bytes);
        let out = bridge.share(len, now_ms);
        serde_json::from_slice(bridge.output_bytes(out as usize)).unwrap()
    }

    fn call_land(bridge: &mut Bridge<NoData>) -> serde_json::Value {
        let out = bridge.share_land();
        serde_json::from_slice(bridge.output_bytes(out as usize)).unwrap()
    }

    #[test]
    fn cold_share_rejects_unsupported_version_and_empty_land_errors() {
        let plan = contract::compile(APP).unwrap().encode();
        let mut bridge = Bridge::<NoData>::new();
        let target = serde_json::json!({ "action": "/share", "params": { "text": "text" } });

        let bad = call_share(
            &mut bridge,
            &serde_json::json!({
                "target": target,
                "share": { "v": 2, "text": "future" },
                "inbox": "/tmp/fake-inbox"
            }),
            0.0,
        );
        assert!(bad["error"].is_string(), "expected error batch: {bad}");

        let boot_len = bridge.boot(&plan, NoData, Hooks::none(), 390.0, 844.0);
        let boot: serde_json::Value =
            serde_json::from_slice(bridge.output_bytes(boot_len as usize)).unwrap();
        assert!(boot["error"].is_null(), "{boot}");

        let land = call_land(&mut bridge);
        assert!(land["error"].is_string(), "expected error batch: {land}");
    }

    fn run_in_child(test_name: &str, env_var: &str) -> bool {
        if std::env::var(env_var).as_deref() == Ok("1") {
            return false;
        }
        let result = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", test_name, "--test-threads=1"])
            .env(env_var, "1")
            .output()
            .unwrap();
        let out = String::from_utf8_lossy(&result.stdout);
        assert!(
            result.status.success() && out.contains("1 passed"),
            "{out}{}",
            String::from_utf8_lossy(&result.stderr)
        );
        true
    }

    fn status_text(bridge: &Bridge<NoData>) -> String {
        let k = bridge.host.as_ref().unwrap().runner().kernel();
        let key = k.find_by_test_id("status")[0];
        k.node_by_key(key)
            .unwrap()
            .props
            .str(exact_kernel::PropId::Text)
            .unwrap_or_default()
            .to_string()
    }

    #[test]
    fn inbox_attachments_are_kept_on_stat_and_landing_failures() {
        // Isolate process-global `picker::ROOTS` in a child process (as `answers_tests.rs` does).
        if run_in_child(
            "abi::share::tests::inbox_attachments_are_kept_on_stat_and_landing_failures",
            "EXACT_SHARE_RETAIN_CHILD",
        ) {
            return;
        }
        let plan = contract::compile(APP).unwrap().encode();
        let dir =
            std::env::temp_dir().join(format!("exact-apple-share-retain-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let inbox = dir.join("inbox");
        std::fs::create_dir_all(&inbox).unwrap();
        let attachment = inbox.join("1.txt");
        std::fs::write(&attachment, b"precious").unwrap();
        let blocked_tmp = dir.join("blocked-tmp");
        std::fs::write(&blocked_tmp, b"file").unwrap();
        crate::picker::set_roots(dir.join("data"), dir.join("cache"), blocked_tmp.clone());

        let target = serde_json::json!({
            "action": "/share",
            "params": {
                "text": "text",
                "files": [{ "name": "files", "accept": ["text/plain"] }]
            }
        });
        let share = serde_json::json!({
            "v": 1,
            "text": "hello",
            "files": [{ "file": "1.txt", "name": "note.txt", "type": "text/plain", "member": "files" }],
            "dropped": {}
        });

        // Missing/unstatable inbox attachment must return an error on cold claim
        // instead of dropping the file and returning success.
        let missing_share = serde_json::json!({
            "v": 1,
            "files": [{ "file": "missing.txt", "name": "note.txt", "type": "text/plain", "member": "files" }],
            "dropped": {}
        });
        let mut bad_cold = Bridge::<NoData>::new();
        let stat_err = call_share(
            &mut bad_cold,
            &serde_json::json!({ "target": target, "share": missing_share, "inbox": inbox.to_string_lossy() }),
            0.0,
        );
        assert!(
            stat_err["error"].is_string(),
            "expected stat error: {stat_err}"
        );

        // Cold claim -> boot (sees pending=1, files=0) -> land fails -> error batch,
        // inbox attachment stays intact -> retry after storage is writable lands the
        // live token (files=1, pending=0) without re-navigating.
        let mut bridge = Bridge::<NoData>::new();
        let cold = call_share(
            &mut bridge,
            &serde_json::json!({ "target": target, "share": share, "inbox": inbox.to_string_lossy() }),
            0.0,
        );
        assert!(cold["error"].is_null(), "{cold}");
        let boot_len = bridge.boot(&plan, NoData, Hooks::none(), 390.0, 844.0);
        let boot: serde_json::Value =
            serde_json::from_slice(bridge.output_bytes(boot_len as usize)).unwrap();
        assert!(boot["error"].is_null(), "{boot}");
        assert_eq!(status_text(&bridge), "share:0:0:1:hello");

        let land = call_land(&mut bridge);
        assert!(land["error"].is_string(), "expected land error: {land}");
        assert!(
            attachment.exists(),
            "cold land failure must not remove inbox file"
        );
        assert_eq!(status_text(&bridge), "share:0:0:1:hello");

        // While a cold share is waiting to land, warm inbox shares are deferred instead of clobbering it.
        let root_id = {
            let k = bridge.host.as_ref().unwrap().runner().kernel();
            k.node_by_key(k.find_by_test_id("root")[0]).unwrap().id
        };
        let blocked_warm = call_share(
            &mut bridge,
            &serde_json::json!({
                "root": root_id,
                "target": target,
                "share": share,
                "inbox": inbox.to_string_lossy()
            }),
            5.0,
        );
        assert!(blocked_warm["error"].is_string(), "{blocked_warm}");

        // Cold landing retry after storage becomes writable lands the original cold share
        // and updates `sharedFiles` to 1 file, 0 pending.
        crate::picker::set_roots(dir.join("data"), dir.join("cache"), dir.join("good-tmp"));
        let retried_land = call_land(&mut bridge);
        assert!(
            retried_land["error"].is_null(),
            "cold land retry should succeed: {retried_land}"
        );
        assert_eq!(status_text(&bridge), "share:0:1:0:hello");
        assert!(attachment.exists());

        // Warm claim on invalid root fails before mutating tokens or copying files.
        crate::picker::set_roots(dir.join("data"), dir.join("cache"), blocked_tmp);
        let bad_root = call_share(
            &mut bridge,
            &serde_json::json!({
                "root": 999_999,
                "target": target,
                "share": share,
                "inbox": inbox.to_string_lossy()
            }),
            8.0,
        );
        assert!(bad_root["error"].is_string(), "{bad_root}");
        assert_eq!(status_text(&bridge), "share:0:1:0:hello");

        // Warm claim -> missing/unstatable attachment errors without navigating.
        let warm_stat = call_share(
            &mut bridge,
            &serde_json::json!({
                "root": root_id,
                "target": target,
                "share": missing_share,
                "inbox": inbox.to_string_lossy()
            }),
            9.0,
        );
        assert!(
            warm_stat["error"].is_string(),
            "expected warm stat error: {warm_stat}"
        );
        assert_eq!(status_text(&bridge), "share:0:1:0:hello");

        // Warm claim follows LLP 1133.024 D3 two-step delivery: `call_share` delivers
        // route & text immediately with `pending=1` without blocking on attachment copies,
        // `call_land` fails while storage is blocked (keeping inbox file intact), and a
        // retry after storage is writable lands the live token (`files=1`, `pending=0`)
        // without incrementing `navCount` again!
        let warm = call_share(
            &mut bridge,
            &serde_json::json!({
                "root": root_id,
                "target": target,
                "share": share,
                "inbox": inbox.to_string_lossy()
            }),
            10.0,
        );
        assert!(
            warm["error"].is_null(),
            "warm navigate should succeed immediately: {warm}"
        );
        assert!(
            attachment.exists(),
            "warm navigate must not remove inbox file"
        );
        assert_eq!(status_text(&bridge), "share:1:0:1:hello");

        let warm_land_fail = call_land(&mut bridge);
        assert!(
            warm_land_fail["error"].is_string(),
            "expected warm land error while storage blocked: {warm_land_fail}"
        );
        assert!(
            attachment.exists(),
            "warm land failure must not remove inbox file"
        );
        assert_eq!(status_text(&bridge), "share:1:0:1:hello");

        crate::picker::set_roots(dir.join("data"), dir.join("cache"), dir.join("good-tmp-2"));
        let warm_retry = call_land(&mut bridge);
        assert!(
            warm_retry["error"].is_null(),
            "warm land retry should succeed: {warm_retry}"
        );
        assert_eq!(status_text(&bridge), "share:1:1:0:hello");
        assert!(attachment.exists());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn warm_share_defers_when_change_action_yields_needs_apply_then_succeeds() {
        if run_in_child(
            "abi::share::tests::warm_share_defers_when_change_action_yields_needs_apply_then_succeeds",
            "EXACT_SHARE_DEFER_CHILD",
        ) {
            return;
        }
        let plan = contract::compile(CHANGE_APP).unwrap().encode();
        let dir =
            std::env::temp_dir().join(format!("exact-apple-share-defer-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let inbox = dir.join("inbox");
        std::fs::create_dir_all(&inbox).unwrap();
        std::fs::write(inbox.join("1.txt"), b"hello").unwrap();
        crate::picker::set_roots(dir.join("data"), dir.join("cache"), dir.join("tmp"));

        let mut bridge = Bridge::<NoData>::new();
        let boot_len = bridge.boot(&plan, NoData, Hooks::none(), 390.0, 844.0);
        let boot: serde_json::Value =
            serde_json::from_slice(bridge.output_bytes(boot_len as usize)).unwrap();
        assert!(boot["error"].is_null(), "{boot}");

        let (root_id, arm_id) = {
            let k = bridge.host.as_ref().unwrap().runner().kernel();
            (
                k.node_by_key(k.find_by_test_id("root")[0]).unwrap().id,
                k.node_by_key(k.find_by_test_id("arm")[0]).unwrap().id,
            )
        };
        // Press "Arm" so `change(armed, onArm)` becomes due and yields `needs_apply`.
        let press_len = bridge.dispatch(arm_id, 0, 0, 10.0);
        let press: serde_json::Value =
            serde_json::from_slice(bridge.output_bytes(press_len as usize)).unwrap();
        assert!(press["error"].is_null(), "{press}");

        let target = serde_json::json!({
            "action": "/share",
            "params": { "files": [{ "name": "files", "accept": ["text/plain"] }] }
        });
        let req = serde_json::json!({
            "root": root_id,
            "target": target,
            "share": {
                "v": 1,
                "files": [{ "file": "1.txt", "name": "note.txt", "type": "text/plain", "member": "files" }],
                "dropped": {}
            },
            "inbox": inbox.to_string_lossy()
        });

        // First claim sees `needs_apply`, commits the change receipt, and defers without losing the share.
        let first = call_share(&mut bridge, &req, 20.0);
        assert_eq!(
            first["error"].as_str(),
            Some("share: deferred until pending work is applied"),
            "{first}"
        );
        assert!(inbox.join("1.txt").exists());

        // Retrying after the deferred receipt is applied navigates cleanly and lands attachments.
        let second = call_share(&mut bridge, &req, 20.0);
        assert!(second["error"].is_null(), "{second}");
        let landed = call_land(&mut bridge);
        assert!(landed["error"].is_null(), "{landed}");
        assert!(inbox.join("1.txt").exists());

        // Directory attachments fail as `share: invalid: ` so they are retired instead of
        // retrying forever at the head of the warm queue.
        let subdir = inbox.join("not-a-file");
        std::fs::create_dir_all(&subdir).unwrap();
        let bad_req = serde_json::json!({
            "root": root_id,
            "target": target,
            "share": {
                "v": 1,
                "files": [{ "file": "not-a-file", "name": "dir", "type": "text/plain", "member": "files" }],
                "dropped": {}
            },
            "inbox": inbox.to_string_lossy()
        });
        let bad = call_share(&mut bridge, &bad_req, 21.0);
        assert!(
            bad["error"]
                .as_str()
                .is_some_and(|e| e.starts_with("share: invalid: ")),
            "expected permanent invalid error for directory attachment: {bad}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn carried_reload_keeps_landed_cold_share_instead_of_reverting_to_pending() {
        if run_in_child(
            "abi::share::tests::carried_reload_keeps_landed_cold_share_instead_of_reverting_to_pending",
            "EXACT_SHARE_CARRIED_CHILD",
        ) {
            return;
        }
        let plan = contract::compile(APP).unwrap().encode();
        let dir =
            std::env::temp_dir().join(format!("exact-apple-share-carried-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let inbox = dir.join("inbox");
        std::fs::create_dir_all(&inbox).unwrap();
        std::fs::write(inbox.join("1.txt"), b"hello").unwrap();
        crate::picker::set_roots(dir.join("data"), dir.join("cache"), dir.join("tmp"));

        let mut bridge = Bridge::<NoData>::new();
        let target = serde_json::json!({
            "action": "/share",
            "params": { "text": "text", "files": [{ "name": "files", "accept": ["text/plain"] }] }
        });
        let req = serde_json::json!({
            "target": target,
            "share": {
                "v": 1,
                "text": "hello",
                "files": [{ "file": "1.txt", "name": "note.txt", "type": "text/plain", "member": "files" }],
                "dropped": {}
            },
            "inbox": inbox.to_string_lossy()
        });
        let cold = call_share(&mut bridge, &req, 0.0);
        assert!(cold["error"].is_null(), "{cold}");
        let boot_len = bridge.boot(&plan, NoData, Hooks::none(), 390.0, 844.0);
        let boot: serde_json::Value =
            serde_json::from_slice(bridge.output_bytes(boot_len as usize)).unwrap();
        assert!(boot["error"].is_null(), "{boot}");
        let land = call_land(&mut bridge);
        assert!(land["error"].is_null(), "{land}");
        assert_eq!(status_text(&bridge), "share:0:1:0:hello");
        assert!(bridge.launch_facts.shared.is_empty());

        // A state-preserving reload must keep the landed share (`files=1, pending=0`),
        // not re-seed pending tokens from stale `launch_facts.shared`.
        let carried = bridge.host.as_ref().unwrap().carry();
        let _ = bridge.input(plan.len());
        bridge.input[..plan.len()].copy_from_slice(&plan);
        let prep_len = bridge.prepare_plan_carried(
            plan.len(),
            NoData,
            Hooks::none(),
            390.0,
            844.0,
            None,
            Some(carried),
            None,
            crate::abi::boot::TreeBase::Live,
        );
        assert!(prep_len > 0);
        let commit_len = bridge.commit_plan();
        let committed: serde_json::Value =
            serde_json::from_slice(bridge.output_bytes(commit_len as usize)).unwrap();
        assert!(committed["error"].is_null(), "{committed}");
        assert_eq!(status_text(&bridge), "share:0:1:0:hello");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
