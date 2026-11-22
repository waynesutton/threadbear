// The Swift helper that hears and speaks for Voice
// (desktop/gpui/native/voice_helper.swift). build.rs compiles it into OUT_DIR
// and it rides inside this binary, so the bundle and its update scripts don't
// change; the first time Voice is used it's written to
// ~/Library/Application Support/Threadbear/voice/gpui-helper-<hash>/, beside
// the voice engine, and started with pipes for its stdin and stdout. It's a
// child of this app, so macOS asks for the microphone in this app's name.
// When the app goes, its end of the pipe closes and the helper stops, and
// pocket-tts with it.
use serde_json::Value;
use std::io::{BufRead as _, Write as _};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};

#[cfg(target_os = "macos")]
static HELPER: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/threadbear-voice-helper"));
#[cfg(not(target_os = "macos"))]
static HELPER: &[u8] = b"";

/// This build has a helper to run: on a Mac, with swiftc when it was built.
pub(crate) fn built_in() -> bool {
    !HELPER.is_empty()
}

pub(crate) fn folder() -> PathBuf {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default();
    home.join("Library/Application Support/Threadbear/voice")
}

pub(crate) struct Helper {
    child: Child,
    stdin: ChildStdin,
}

impl Helper {
    /// Starts it, with what it says arriving on the channel a line at a time.
    pub(crate) fn start(
    ) -> Result<(Helper, futures::channel::mpsc::UnboundedReceiver<Value>), String> {
        if !built_in() {
            return Err(
                "This build of Threadbear has no Voice (it was built without the macOS 26 SDK)."
                    .to_owned(),
            );
        }
        let path = Self::write_out()?;
        let mut child = Command::new(&path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Voice's helper didn't start: {e}"))?;
        let stdin = child.stdin.take().ok_or("Voice's helper has no input")?;
        let stdout = child.stdout.take().ok_or("Voice's helper has no output")?;
        let (tx, rx) = futures::channel::mpsc::unbounded();
        std::thread::Builder::new()
            .name("voice-helper".into())
            .spawn(move || {
                for line in std::io::BufReader::new(stdout).lines() {
                    let Ok(line) = line else { break };
                    if let Ok(value) = serde_json::from_str::<Value>(&line) {
                        if tx.unbounded_send(value).is_err() {
                            break;
                        }
                    }
                }
                let _ = tx.unbounded_send(serde_json::json!({ "event": "gone" }));
            })
            .map_err(|e| e.to_string())?;
        Ok((Helper { child, stdin }, rx))
    }

    /// The helper on disk, named for its contents so a new build's goes
    /// beside the old rather than over one that's running.
    fn write_out() -> Result<PathBuf, String> {
        use std::hash::{Hash as _, Hasher as _};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        HELPER.hash(&mut hasher);
        let dir = folder().join(format!("gpui-helper-{:016x}", hasher.finish()));
        let path = dir.join("threadbear-voice-helper");
        if std::fs::metadata(&path).is_ok_and(|m| m.len() == HELPER.len() as u64) {
            return Ok(path);
        }
        // This user's only, and runnable by them alone.
        threadbear_core::private_file::write_with_mode(&path, HELPER, 0o700)
            .map_err(|e| format!("Couldn't write Voice's helper: {e}"))?;
        // Older builds' helpers: nothing runs them any more.
        if let Ok(entries) = std::fs::read_dir(folder()) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if name.starts_with("gpui-helper-") && entry.path() != dir {
                    let _ = std::fs::remove_dir_all(entry.path());
                }
            }
        }
        Ok(path)
    }

    /// One command. False once the helper has gone.
    pub(crate) fn send(&mut self, command: &Value) -> bool {
        let mut line = command.to_string();
        line.push('\n');
        self.stdin
            .write_all(line.as_bytes())
            .and_then(|_| self.stdin.flush())
            .is_ok()
    }
}

impl Drop for Helper {
    fn drop(&mut self) {
        // Closing its input is its cue to stop pocket-tts and go.
        let _ = self.child.try_wait();
    }
}
