//! Interactive launch samples the machine; headless launch uses the agent facts.
use chrono::{Offset, TimeZone};
use exact_runner::time::{Launch, Place, WallTime};
use std::time::{SystemTime, UNIX_EPOCH};

const AGENT_EPOCH: f64 = 1_767_225_600_000.;

/// An IANA name from `TZ` as the C library reads it: a leading `:` and a
/// `zoneinfo/` path are forms of a name; a POSIX rule (`PST8PDT,M3.2.0,…`) is not one.
fn zone_name(value: &str) -> Option<String> {
    let name = value.strip_prefix(':').unwrap_or(value);
    let name = name.rsplit_once("zoneinfo/").map_or(name, |(_, n)| n);
    let name = name
        .strip_prefix("posix/")
        .or_else(|| name.strip_prefix("right/"))
        .unwrap_or(name);
    name.parse::<chrono_tz::Tz>()
        .ok()
        .map(|zone| zone.name().to_string())
}

pub(crate) fn launch(
    interactive: bool,
    env: impl Fn(&str) -> Option<String>,
) -> Result<(Launch, Vec<String>), String> {
    let mut notes = Vec::new();
    let mut place = if interactive {
        let (locale, format_locale) = exact_runner::time::posix_locales(&env);
        let mut bytes = [0; 4];
        getrandom::fill(&mut bytes).map_err(|e| format!("launch seed: {e}"))?;
        Place {
            locale,
            time_zone: env("TZ")
                .and_then(|tz| zone_name(&tz))
                .or_else(|| iana_time_zone::get_timezone().ok())
                .unwrap_or_else(|| "UTC".into()),
            seed: f64::from(u32::from_ne_bytes(bytes)),
            format_locale,
        }
    } else {
        Place {
            locale: env("EXACT_AGENT_LOCALE").unwrap_or_else(|| "en-US".into()),
            time_zone: env("EXACT_AGENT_TIME_ZONE").unwrap_or_else(|| "UTC".into()),
            seed: exact_runner::time::agent_seed(env("EXACT_AGENT_SEED"))?,
            format_locale: env("EXACT_AGENT_FORMAT_LOCALE").unwrap_or_default(),
        }
    };
    // Each fact falls back alone: a zone the runner cannot use keeps the locale.
    let zone = place
        .time_zone
        .parse::<chrono_tz::Tz>()
        .unwrap_or_else(|_| {
            notes.push(format!(
                "terminal zone refused: {}; using UTC",
                place.time_zone
            ));
            place.time_zone = "UTC".into();
            chrono_tz::UTC
        });
    if place.validate().is_err() && !place.format_locale.is_empty() {
        let without = Place {
            format_locale: String::new(),
            ..place.clone()
        };
        if without.validate().is_ok() {
            notes.push(format!(
                "terminal format locale refused: {}; using the locale",
                place.format_locale
            ));
            place = without;
        }
    }
    if place.validate().is_err() {
        notes.push(format!(
            "terminal locale refused: {}; using en-US",
            place.locale
        ));
        place.locale = "en-US".into();
    }
    if place.validate().is_err() {
        place = Place {
            seed: place.seed,
            ..Place::default()
        };
    }
    let epoch = if interactive {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as f64
    } else {
        env("EXACT_AGENT_EPOCH").map_or(AGENT_EPOCH, |s| {
            s.parse::<u64>().map_or(f64::NAN, |n| n as f64)
        })
    };
    let offset = zone
        .timestamp_millis_opt(epoch as i64)
        .single()
        .map_or(0., |at| {
            f64::from(at.offset().fix().local_minus_utc()) / 60.
        });
    let mut time = WallTime {
        epoch_at_zero: epoch,
        utc_offset: offset,
    };
    if time.validate().is_err() {
        notes.push("terminal time refused; using the agent epoch in UTC".into());
        time = WallTime {
            epoch_at_zero: AGENT_EPOCH,
            utc_offset: 0.,
        };
        place.time_zone = "UTC".into();
    }
    Ok((
        Launch {
            time: Some(time),
            place: Some(place),
            ..Launch::NONE
        },
        notes,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interactive_launch_separates_message_and_format_locales() {
        for (all, messages, time, lang, expected) in [
            ("", "", "de_DE.UTF-8", "en_US.UTF-8", ("en-US", "de-DE")),
            ("", "fr_FR.UTF-8", "", "en_US.UTF-8", ("fr-FR", "en-US")),
            ("ja_JP.UTF-8", "fr_FR", "de_DE", "en_US", ("ja-JP", "ja-JP")),
            ("", "fr_FR", "", "", ("fr-FR", "")),
            ("C.UTF-8", "fr_FR", "de_DE", "en_US", ("en-US", "en-US")),
            ("", "", "", "", ("en-US", "")),
        ] {
            let (facts, notes) = launch(true, |key| {
                Some(
                    match key {
                        "LC_ALL" => all,
                        "LC_MESSAGES" => messages,
                        "LC_TIME" => time,
                        "LANG" => lang,
                        "TZ" => "UTC",
                        "EXACT_AGENT" => "1",
                        key if key.starts_with("EXACT_AGENT_") => {
                            panic!("interactive launch read {key}")
                        }
                        _ => return None,
                    }
                    .into(),
                )
            })
            .unwrap();
            assert!(notes.is_empty(), "{notes:?}");
            let place = facts.place.unwrap();
            assert_eq!(
                (place.locale.as_str(), place.format_locale.as_str()),
                expected
            );
        }
    }

    #[test]
    fn interactive_launch_draws_a_fresh_u32_seed() {
        let seeds: Vec<_> = (0..4)
            .map(|_| {
                let (facts, notes) =
                    launch(true, |key| (key == "TZ").then(|| "UTC".into())).unwrap();
                assert!(notes.is_empty(), "{notes:?}");
                let place = facts.place.unwrap();
                assert!(place.validate().is_ok(), "{place:?}");
                place.seed
            })
            .collect();
        assert!(seeds.iter().any(|seed| *seed != seeds[0]), "{seeds:?}");
    }

    #[test]
    fn headless_facts_use_the_agent_epoch_and_zone_without_reading_the_machine() {
        let (facts, notes) = launch(false, |key| match key {
            "EXACT_AGENT_EPOCH" => Some("1790000000000".into()),
            "EXACT_AGENT_TIME_ZONE" => Some("America/Toronto".into()),
            "EXACT_AGENT_LOCALE" => Some("fr-CA".into()),
            "EXACT_AGENT_FORMAT_LOCALE" => Some("fr-FR".into()),
            "EXACT_AGENT_SEED" => Some("42".into()),
            _ => panic!("headless launch read {key}"),
        })
        .unwrap();
        assert!(notes.is_empty());
        let time = facts.time.unwrap();
        assert_eq!(
            (time.epoch_at_zero, time.utc_offset),
            (1_790_000_000_000., -240.)
        );
        let place = facts.place.unwrap();
        assert_eq!(place.seed, 42.);
        assert_eq!(
            (place.locale.as_str(), place.format_locale.as_str()),
            ("fr-CA", "fr-FR")
        );
        let (default, _) = launch(false, |_| None).unwrap();
        assert_eq!(default.time.unwrap().epoch_at_zero, AGENT_EPOCH);
        assert_eq!(default.place.unwrap().seed, 1.);
    }

    #[test]
    fn headless_seed_accepts_the_u32_range_and_refuses_invalid_values() {
        for (value, expected) in [("0", 0.), ("4294967295", 4_294_967_295.)] {
            let (facts, notes) = launch(false, |key| {
                (key == "EXACT_AGENT_SEED").then(|| value.into())
            })
            .unwrap();
            assert!(notes.is_empty(), "{notes:?}");
            let place = facts.place.unwrap();
            assert_eq!(place.seed, expected);
            assert!(place.validate().is_ok());
        }
        for value in [
            "4294967296",
            "9007199254740991",
            "-1",
            "1.5",
            "NaN",
            "seven",
            "",
        ] {
            let error = launch(false, |key| {
                (key == "EXACT_AGENT_SEED").then(|| value.into())
            })
            .unwrap_err();
            assert!(error.contains("EXACT_AGENT_SEED"), "{value}: {error}");
            assert!(error.contains("0 through 2^32 - 1"), "{value}: {error}");
        }
    }

    #[test]
    fn refused_places_and_times_fall_back_to_valid_facts_with_a_note() {
        for (key, value) in [
            ("EXACT_AGENT_LOCALE", "bad@locale"),
            ("EXACT_AGENT_FORMAT_LOCALE", "bad@locale"),
            ("EXACT_AGENT_TIME_ZONE", "Bad/Zone"),
            ("EXACT_AGENT_EPOCH", "not a date"),
        ] {
            let (facts, notes) = launch(false, |k| (k == key).then(|| value.into())).unwrap();
            assert_eq!(notes.len(), 1);
            assert!(facts.place.unwrap().validate().is_ok());
            assert!(facts.time.unwrap().validate().is_ok());
        }
    }

    #[test]
    fn a_tz_form_or_rule_keeps_the_locale() {
        let env = |tz: &'static str| {
            move |key: &str| match key {
                "TZ" => Some(tz.to_string()),
                "LANG" => Some("fr_FR.UTF-8".to_string()),
                _ => None,
            }
        };
        let (facts, notes) = launch(true, env(":America/Los_Angeles")).unwrap();
        let place = facts.place.unwrap();
        assert_eq!(
            (place.locale.as_str(), place.time_zone.as_str()),
            ("fr-FR", "America/Los_Angeles")
        );
        assert!(notes.is_empty(), "{notes:?}");
        assert_eq!(
            zone_name("/usr/share/zoneinfo/Europe/Paris").as_deref(),
            Some("Europe/Paris")
        );
        // A POSIX rule is not a name: the machine's zone stands in, the locale stays.
        let (facts, _) = launch(true, env("PST8PDT,M3.2.0,M11.1.0")).unwrap();
        let place = facts.place.unwrap();
        assert_eq!(place.locale, "fr-FR");
        assert!(place.validate().is_ok());
    }
}
