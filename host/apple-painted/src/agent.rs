//! The native host's JSON-lines protocol, serviced by the window thread.
use super::window::Event;
use serde_json::Value;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::time::Duration;
use winit::event_loop::EventLoopProxy;

pub struct Agent {
    path: Option<PathBuf>,
}
impl Agent {
    pub fn start(proxy: EventLoopProxy<Event>, ready: String) -> Result<Self, String> {
        let path = std::env::var_os("EXACT_AGENT_SOCKET").map(PathBuf::from);
        let listener = path
            .as_ref()
            .map(UnixListener::bind)
            .transpose()
            .map_err(|e| e.to_string())?;
        std::thread::Builder::new()
            .name("painted-agent".into())
            .spawn(move || {
                let result = if let Some(listener) = listener {
                    listener.accept().and_then(|(socket, _)| {
                        let writer = socket.try_clone()?;
                        serve(BufReader::new(socket), writer, &proxy, &ready)
                    })
                } else {
                    serve(
                        BufReader::new(std::io::stdin()),
                        std::io::stdout(),
                        &proxy,
                        &ready,
                    )
                };
                if let Err(error) = result {
                    eprintln!("painted agent: {error}");
                }
                let _ = proxy.send_event(Event::Closed);
            })
            .map_err(|e| e.to_string())?;
        Ok(Self { path })
    }
}
impl Drop for Agent {
    fn drop(&mut self) {
        if let Some(path) = &self.path {
            let _ = std::fs::remove_file(path);
        }
    }
}
fn serve(
    mut reader: impl BufRead,
    mut writer: impl Write,
    proxy: &EventLoopProxy<Event>,
    ready: &str,
) -> std::io::Result<()> {
    writeln!(writer, "{ready}")?;
    writer.flush()?;
    let mut line = String::new();
    while reader.read_line(&mut line)? != 0 {
        let (reply, receive) = std::sync::mpsc::sync_channel(1);
        if proxy
            .send_event(Event::Request(std::mem::take(&mut line), reply))
            .is_err()
        {
            break;
        }
        let Ok(answer) = receive.recv() else {
            break;
        };
        write_reply(&mut writer, &answer, std::thread::sleep)?;
    }
    Ok(())
}

fn write_reply(
    writer: &mut impl Write,
    answer: &Value,
    wait: impl FnOnce(Duration),
) -> std::io::Result<()> {
    // The window can keep turning while the contact is held.
    if let Some(ms) = answer["waitMs"].as_f64().filter(|&ms| ms > 0.) {
        wait(Duration::from_secs_f64(ms / 1000.));
    }
    writeln!(writer, "{answer}")?;
    writer.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_live_drive_waits_for_the_contact_hold_before_replying() {
        let mut output = Vec::new();
        let answer = serde_json::json!({"phase":"hold","waitMs":250});
        let mut waited = None;
        write_reply(&mut output, &answer, |duration| waited = Some(duration)).unwrap();
        assert_eq!(waited, Some(Duration::from_millis(250)));
        assert_eq!(output, format!("{answer}\n").as_bytes());
    }
}
