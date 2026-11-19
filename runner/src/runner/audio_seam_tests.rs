//! The audio seam's unlinked half (LLP 1047.001 D11): a plan that plays
//! nothing runs the same without the audio code, and the host's audio
//! entries answer as a runner with nothing to play.
use super::audio_seam::NoAudio;
use super::*;
use exact_kernel::Kernel;

const STILL: &str = r#"component App
  state n = 0
  action bump
    n = n + 1
  view
    column
      button "Go" testId="tap" press=bump
      text `${n}`
"#;

fn boot(source: &str, audio: &'static dyn Audio<()>) -> Runner<()> {
    Runner::boot_with_delivery_linked(
        RunnerLinks::ALL,
        ModuleLinks {
            audio,
            ..ModuleLinks::ALL
        },
        contract::compile(source).unwrap(),
        (),
        Kernel::with_monospace(),
        None,
        Vec::new(),
        Default::default(),
        Default::default(),
        "/",
    )
    .unwrap()
}

/// What the runner said and handed the host while it acted and its clock
/// ran.
fn run(audio: &'static dyn Audio<()>) -> (Vec<String>, Vec<String>) {
    let mut r = boot(STILL, audio);
    r.act("bump", vec![]).unwrap();
    r.advance(250.0).unwrap();
    let commands = r.take_commands().into_iter().map(|c| c.name).collect();
    assert!(r.take_sounds().is_empty() && r.take_midi().is_empty());
    assert!(!r.transport_playing());
    // Without the line's time.
    let journal = r
        .journal()
        .map(|l| l.split_once(' ').map_or(l, |(_, rest)| rest).to_string())
        .collect();
    (journal, commands)
}

#[test]
fn a_still_plan_runs_the_same_without_the_audio_code() {
    assert_eq!(run(&NoAudio), run(&LinkedAudio));
}

#[test]
fn the_unlinked_half_refuses_the_hosts_midi() {
    let mut r = boot(STILL, &NoAudio);
    assert!(r
        .midi_host(r#"{"ports":{"state":"granted"}}"#)
        .unwrap_err()
        .contains("no MIDI"));
    assert!(r
        .midi_agent(r#"{"op":"midi","input":"keys","data":[144,60,1]}"#)
        .is_none());
    r.end_sounds();
    // The agent's `state` shows no sound, transport or MIDI.
    let state = crate::agent::handle(&r, r#"{"op":"state"}"#);
    assert!(!state.contains("\"transport\""), "{state}");
}
