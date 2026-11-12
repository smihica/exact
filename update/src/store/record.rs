//! The record: which entry launches, which one is last good, the failure
//! count, and the floor of the stream this client accepted.
//!
//! @ref LLP 1026 D11 / LLP 1030 D9 (an unknown record codec)

use super::disk::read_bounded;
use super::Embedded;
use std::path::Path;

/// The record, in memory. Its JSON is `{ "codec", "selected", "pending",
/// "lastGood", "failures", "stream": { "channel", "compatibilityId",
/// "seq" }, "bad" }`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Record {
    pub(super) selected: Option<String>,
    /// An entry on disk whole that no launch boots until the app activates
    /// it — the `app-decides` policy (LLP 1030.000 D4): `check` puts it
    /// here instead of `selected`, and `activate` promotes it.
    pub(super) pending: Option<String>,
    pub(super) last_good: Option<String>,
    pub(super) failures: u64,
    pub(super) compatibility_id: String,
    pub(super) channel: String,
    /// The highest stream sequence this client has admitted, including an
    /// embedded generation or a head found current while entry zero ran.
    pub(super) seq: u64,
    pub(super) digest: Option<String>,
    /// Entries that failed to reach first pixel twice. A head that names one is
    /// refused rather than staged again, so a bad bundle cannot loop a client
    /// through the same download every launch.
    pub(super) bad: Vec<String>,
}

impl Record {
    pub(super) fn fresh(embedded: &Embedded) -> Record {
        Record {
            selected: None,
            pending: None,
            last_good: None,
            failures: 0,
            compatibility_id: embedded.compatibility_id.clone(),
            channel: embedded.channel.clone(),
            seq: embedded.seq,
            digest: embedded.entry_digest.clone(),
            bad: Vec::new(),
        }
    }

    pub(super) fn to_json(&self) -> String {
        let quoted = |v: &Option<String>| match v {
            Some(s) => serde_json::Value::String(s.clone()),
            None => serde_json::Value::Null,
        };
        let value = serde_json::json!({
            "codec": crate::STORE_CODEC,
            "selected": quoted(&self.selected),
            "pending": quoted(&self.pending),
            "lastGood": quoted(&self.last_good),
            "failures": self.failures,
            "stream": { "channel": self.channel, "compatibilityId": self.compatibility_id, "seq": self.seq, "digest": self.digest },
            "bad": self.bad,
        });
        format!("{value}\n")
    }
}

/// What was on disk where the record lives.
pub(super) enum RecordFile {
    /// No record, or one that is not a record of any codec (unreadable, not
    /// JSON, or with no integer codec): start fresh.
    Fresh,
    /// An older codec from this pre-1.0 crate. Its selections are not read;
    /// open replaces it with this binary's clean entry-zero record.
    Obsolete,
    /// A record written by a binary with another store codec (LLP 1030 D9).
    Foreign,
    /// This codec's record.
    Ours(Record),
}

pub(super) const MAX_RECORD_BYTES: u64 = 1024 * 1024;

pub(super) fn read_record(path: &Path) -> RecordFile {
    let Ok(raw) = read_bounded(path, MAX_RECORD_BYTES) else {
        return RecordFile::Fresh;
    };
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(&raw) else {
        return RecordFile::Fresh;
    };
    let Some(object) = value.as_object() else {
        return RecordFile::Fresh;
    };
    // Only an integer codec above ours is another binary's (LLP 1030 D9). A
    // record with no such codec is damaged, not foreign: freezing on it would
    // stop this updater for good.
    match object.get("codec").and_then(|v| v.as_u64()) {
        Some(codec) if codec == crate::STORE_CODEC => {}
        Some(codec) if codec < crate::STORE_CODEC => return RecordFile::Obsolete,
        Some(_) => return RecordFile::Foreign,
        None => return RecordFile::Fresh,
    }
    let stream = object.get("stream").and_then(|v| v.as_object());
    RecordFile::Ours(Record {
        selected: object
            .get("selected")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        pending: object
            .get("pending")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        last_good: object
            .get("lastGood")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        failures: object.get("failures").and_then(|v| v.as_u64()).unwrap_or(0),
        compatibility_id: stream
            .and_then(|s| s.get("compatibilityId"))
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        channel: stream
            .and_then(|s| s.get("channel"))
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        seq: stream
            .and_then(|s| s.get("seq"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        digest: stream
            .and_then(|s| s.get("digest"))
            .and_then(|v| v.as_str())
            .map(str::to_string),
        bad: object
            .get("bad")
            .and_then(|v| v.as_array())
            .map(|list| {
                list.iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default(),
    })
}
