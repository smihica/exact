//! A share received on Apple (LLP 1133.024 D2–D4): the claim path the share
//! extension's inbox and the driver's `share:` form both take. Due work up to
//! `now_ms` settles first; if a change action yields for receipt application,
//! the claim is deferred without mutating the inbox. Otherwise the tokens are
//! set pending, the location commits at the navigation root, and only after the
//! navigation receipt succeeds do files copy into `app:/tmp/shared/` (leaving
//! inbox originals intact until the entire commit batch succeeds).
use super::*;
use exact_runner::share_in::{Claimed, Pending, ShareTarget};

impl<D: DataSource> Host<D> {
    /// `{"root": <navigation root>, "target": <share_target>, "share": {…}}`,
    /// with `inbox` the directory a `share.json` share's files wait in (none
    /// for the driver's, whose `files` are paths it copies from).
    /// Returns `(batch_json, unlanded)` where `unlanded` is `Some` when an inbox
    /// share's `Navigate` committed (per LLP 1133.024 D3) but landing its files
    /// failed afterward so `Bridge::share_land` can retry without re-navigating.
    pub fn receive_share(
        &mut self,
        request: &serde_json::Value,
        inbox: Option<&std::path::Path>,
        now_ms: f64,
    ) -> (String, Option<(Claimed, bool)>) {
        let Some(target) = ShareTarget::from_json(&request["target"]) else {
            return (
                self.commit(
                    &[],
                    Some(
                        "share: it needs the navigation root, the app's share_target and a share"
                            .into(),
                    ),
                ),
                None,
            );
        };
        let Some(share) = Pending::from_json(&request["share"], inbox) else {
            let msg = if inbox.is_some() {
                "share: invalid: unsupported or malformed share record"
            } else {
                "share: it needs the navigation root, the app's share_target and a share"
            };
            return (self.commit(&[], Some(msg.into())), None);
        };
        let Some(root) = request["root"].as_u64() else {
            return (
                self.commit(
                    &[],
                    Some(
                        "share: it needs the navigation root, the app's share_target and a share"
                            .into(),
                    ),
                ),
                None,
            );
        };
        self.now_ms = now_ms.max(self.now_ms);
        // Settle due work before claiming: if a change action yields (`needs_apply`)
        // or a prior held step fails, commit its receipts and keep the inbox untouched
        // so Swift applies the batch and retries without deleting or duplicating the share.
        let pre = self.runner.advance_timed(self.now_ms);
        if pre.needs_apply || pre.stopped_on_request {
            return (
                self.commit(
                    &pre.receipts,
                    Some("share: deferred until pending work is applied".into()),
                ),
                None,
            );
        }
        if let Some(err) = pre.error {
            return (self.commit(&pre.receipts, Some(format!("{err:?}"))), None);
        }
        let mut receipts = pre.receipts;
        if inbox.is_some() {
            // Validate the navigation root before claiming tokens or copying files
            // so a stale or invalid root leaves counters, tokens, and disk untouched.
            if self.runner.kernel().node(root as ViewId).is_none()
                || !self
                    .runner
                    .handlers_of(root as ViewId)
                    .contains(&exact_plan::EventKind::Navigate)
            {
                return (
                    self.commit(
                        &receipts,
                        Some("share: the navigation root is not ready".into()),
                    ),
                    None,
                );
            }
            let mut claimed = match self.runner.claim_share_checked(&target, share) {
                Ok(claimed) => claimed,
                Err(e) => return (self.commit(&receipts, Some(e)), None),
            };
            claimed.inbox = inbox.map(std::path::PathBuf::from);
            claimed.inbox_inode = parse_inode(&request["inboxInode"]);
            let mut error = None;
            let mut token_receipts = Vec::new();
            let mut set_tokens: Vec<String> = Vec::new();
            for (token, files) in claimed.tokens.clone() {
                self.set_shared_checked(&token, files, &mut token_receipts, &mut error);
                if error.is_some() {
                    break;
                }
                set_tokens.push(token);
            }
            let mut navigated = false;
            if error.is_none() {
                match self
                    .runner
                    .dispatch(root as ViewId, Event::Navigate(claimed.location.clone()))
                {
                    Ok(receipt) => {
                        receipts.extend(token_receipts);
                        receipts.push(Timed {
                            at_ms: self.now_ms,
                            receipt,
                        });
                        navigated = true;
                    }
                    Err(e) => error = Some(format!("{e:?}")),
                }
            }
            if !navigated {
                for token in set_tokens {
                    let _ = self.runner.clear_shared(&token);
                }
                return (self.commit(&receipts, error), None);
            }
            // LLP 1133.024 D3: present the navigated route and text immediately
            // with `pending > 0`; attachment copies run on the follow-up `landShare()`
            // call instead of blocking navigation on file I/O.
            let raw_has_files = request["share"]["files"]
                .as_array()
                .is_some_and(|a| !a.is_empty());
            let needs_land = claimed.has_files() || raw_has_files;
            let (mut batch, err) = self.commit_tree(&receipts, None, Batch::new());
            let arrange = self.arrange_after_commit(&mut batch);
            self.group_after_commit(&mut batch);
            let (json, ok) =
                self.commit_finish_checked(batch, err.or(arrange), Some("share: land: "));
            return (json, (needs_land || !ok).then_some((claimed, true)));
        }
        let Some(mut claimed) = self.runner.claim_share(&target, share) else {
            return (
                self.commit(
                    &receipts,
                    Some("share: the app declares no routes, so no route reads a share".into()),
                ),
                None,
            );
        };
        let mut error = None;
        for (token, files) in claimed.tokens.clone() {
            self.set_shared(&token, files, &mut receipts, &mut error);
        }
        if error.is_none() {
            match self
                .runner
                .dispatch(root as ViewId, Event::Navigate(claimed.location.clone()))
            {
                Ok(receipt) => receipts.push(Timed {
                    at_ms: self.now_ms,
                    receipt,
                }),
                Err(e) => error = Some(format!("{e:?}")),
            }
        }
        if error.is_none() {
            self.land(&mut claimed, false, &mut receipts, &mut error);
        }
        (self.commit(&receipts, error), None)
    }

    /// A cold share's copies, once storage is configured (or a warm share's
    /// attachment landing / retry after `Navigate` committed). Returns
    /// `(batch_json, ok)` where `ok` is computed after layout, Arrange, `persist()`,
    /// and `resize_rounds` so Rust and Swift stay in lockstep.
    pub fn land_share(&mut self, claimed: &mut Claimed, from_inbox: bool) -> (String, bool) {
        if claimed.landed.is_none() {
            self.runner.log(claimed.line.clone());
        }
        let (mut receipts, mut error) = (Vec::new(), None);
        self.land(claimed, from_inbox, &mut receipts, &mut error);
        let (mut batch, err) = self.commit_tree(&receipts, error, Batch::new());
        let arrange = self.arrange_after_commit(&mut batch);
        self.group_after_commit(&mut batch);
        self.commit_finish_checked(batch, err.or(arrange), None)
    }

    fn share_temp_dir(&mut self) -> std::path::PathBuf {
        self.app_roots()
            .map(|[_, _, temporary]| temporary)
            .unwrap_or_else(|| crate::picker::roots()[2].clone())
    }

    fn land(
        &mut self,
        claimed: &mut Claimed,
        from_inbox: bool,
        receipts: &mut Vec<Timed>,
        error: &mut Option<String>,
    ) {
        if from_inbox {
            self.land_checked(claimed, receipts, error);
        } else {
            let dest = self.share_temp_dir().join("shared");
            for (token, files) in claimed.copy(&dest, false) {
                self.set_shared(&token, files, receipts, error);
            }
        }
    }

    fn land_checked(
        &mut self,
        claimed: &mut Claimed,
        receipts: &mut Vec<Timed>,
        error: &mut Option<String>,
    ) {
        // Reuse already copied & synced files if a previous attempt failed only at
        // layout, Arrange, resize, or store persistence, avoiding duplicate copies.
        if let Some(landed) = claimed.landed.clone() {
            self.set_shared_all_checked(&landed, receipts, error);
            return;
        }
        if !claimed.has_files() {
            claimed.landed = Some(Vec::new());
            return;
        }
        let dest = self.share_temp_dir().join("shared");
        // Keep inbox originals intact until Swift retires the inbox directory
        // after this entire batch succeeds and applies.
        match claimed.copy_checked(&dest) {
            Ok((landed, created)) => {
                let mut err = None;
                self.set_shared_all_checked(&landed, receipts, &mut err);
                if let Some(e) = err {
                    *error = error.take().or(Some(e));
                    for path in created {
                        let _ = std::fs::remove_file(path);
                    }
                } else {
                    claimed.landed = Some(landed);
                }
            }
            Err(e) => *error = error.take().or(Some(e)),
        }
    }

    fn set_shared_all_checked(
        &mut self,
        updates: &[(String, exact_runner::share_in::SharedFiles)],
        receipts: &mut Vec<Timed>,
        error: &mut Option<String>,
    ) {
        for (token, files) in updates {
            if !exact_runner::share_in::is_token(token) {
                *error = error.take().or(Some(format!(
                    "share: refused: {token:?} is not a share token"
                )));
                return;
            }
            if let Err(why) = files.validate() {
                *error = error.take().or(Some(format!("share: refused: {why}")));
                return;
            }
        }
        match self
            .runner
            .set_shared(updates.iter().map(|(t, f)| (t.as_str(), f.clone())))
        {
            Ok(Some(receipt)) => receipts.push(Timed {
                at_ms: self.now_ms,
                receipt,
            }),
            Ok(None) => {}
            Err(e) => *error = error.take().or(Some(format!("{e:?}"))),
        }
    }

    fn set_shared_checked(
        &mut self,
        token: &str,
        files: exact_runner::share_in::SharedFiles,
        receipts: &mut Vec<Timed>,
        error: &mut Option<String>,
    ) {
        if !exact_runner::share_in::is_token(token) {
            *error = error.take().or(Some(format!(
                "share: refused: {token:?} is not a share token"
            )));
            return;
        }
        if let Err(why) = files.validate() {
            *error = error.take().or(Some(format!("share: refused: {why}")));
            return;
        }
        self.set_shared(token, files, receipts, error);
    }

    fn set_shared(
        &mut self,
        token: &str,
        files: exact_runner::share_in::SharedFiles,
        receipts: &mut Vec<Timed>,
        error: &mut Option<String>,
    ) {
        match self.runner.set_shared([(token, files)]) {
            Ok(Some(receipt)) => receipts.push(Timed {
                at_ms: self.now_ms,
                receipt,
            }),
            Ok(None) => {}
            Err(e) => *error = error.take().or(Some(format!("{e:?}"))),
        }
    }
}

pub(crate) fn parse_inode(value: &serde_json::Value) -> Option<(u64, u64)> {
    let pair = value.as_array()?;
    if pair.len() != 2 {
        return None;
    }
    Some((pair[0].as_u64()?, pair[1].as_u64()?))
}
