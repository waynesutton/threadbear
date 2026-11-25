//! Starts BrowserBear's service when the Browser pane needs it and it isn't
//! running. Chrome's extension normally starts it, but Chrome can drop an
//! extension, and after a crash or a restart the socket file is left with
//! nothing behind it ("Connection refused", 25 Nov 2026). Runs only the
//! wrapper BrowserBear's installer writes in its own folder, and only one
//! nobody else can change. Blocking: for the pipe's thread, never the main one.
use std::io::ErrorKind;
use std::os::unix::fs::MetadataExt as _;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub(crate) const NOT_INSTALLED: &str =
    "BrowserBear isn't running, and this install can't be started from Threadbear. Run scripts/install.sh in BrowserBear's folder, then try again.";
pub(crate) const NOT_STARTED: &str = "BrowserBear isn't running. Threadbear couldn't start it.";

/// How long a started service has to open its socket.
const WAIT: Duration = Duration::from_secs(5);
/// A service that won't stay up isn't started over and over.
const AGAIN: Duration = Duration::from_secs(30);

pub(crate) fn wrapper(home: &Path) -> PathBuf {
    home.join("browserbear-service")
}

/// Ours, and nobody else can write it. `symlink_metadata`, so a link is refused.
fn safe(path: &Path, dir: bool) -> bool {
    let Ok(m) = std::fs::symlink_metadata(path) else {
        return false;
    };
    // SAFETY: geteuid has no arguments and can't fail.
    let me = unsafe { libc::geteuid() };
    let kind = if dir {
        m.is_dir()
    } else {
        m.is_file() && m.mode() & 0o100 != 0
    };
    kind && m.uid() == me && m.mode() & 0o022 == 0
}

pub(crate) fn can_start(home: &Path) -> bool {
    safe(home, true) && safe(&wrapper(home), false)
}

/// One for the app, so connections made close together start one service.
#[derive(Default)]
pub(crate) struct Starter(Mutex<Option<Instant>>);

impl Starter {
    /// The user asked: start it now, however recent the last try.
    pub(crate) fn forget(&self) {
        *self.0.lock().unwrap_or_else(|e| e.into_inner()) = None;
    }

    /// Connects, starting the service first if `start` and nothing answers.
    /// Nothing has been sent yet, so trying again can't do a thing twice.
    pub(crate) fn connect(
        &self,
        home: &Path,
        socket: &Path,
        start: bool,
        gone: impl Fn() -> bool,
    ) -> Result<UnixStream, String> {
        let down = match UnixStream::connect(socket) {
            Ok(s) => return Ok(s),
            Err(e) => e,
        };
        if !matches!(
            down.kind(),
            ErrorKind::ConnectionRefused | ErrorKind::NotFound
        ) {
            return Err(format!("Couldn't reach BrowserBear: {down}"));
        }
        if !can_start(home) {
            return Err(NOT_INSTALLED.into());
        }
        if !start {
            return Err(NOT_STARTED.into());
        }
        let began = {
            let mut last = self.0.lock().unwrap_or_else(|e| e.into_inner());
            match *last {
                Some(at) if at.elapsed() < AGAIN => at,
                _ => {
                    let at = Instant::now();
                    *last = Some(at);
                    // The wrapper leaves the service running by itself and returns.
                    let ran = Command::new(wrapper(home))
                        .stdin(Stdio::null())
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .spawn();
                    match ran {
                        Ok(mut child) => {
                            // Waited on elsewhere: a wrapper that hangs mustn't hold the pane.
                            let _ = std::thread::Builder::new()
                                .name("browserbear-start".into())
                                .spawn(move || child.wait());
                        }
                        Err(_) => return Err(NOT_STARTED.into()),
                    }
                    at
                }
            }
        };
        while began.elapsed() < WAIT && !gone() {
            if let Ok(s) = UnixStream::connect(socket) {
                return Ok(s);
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        Err(NOT_STARTED.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::{symlink, PermissionsExt as _};
    use std::os::unix::net::UnixListener;

    struct Home(PathBuf);
    impl Drop for Home {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    fn home(name: &str, script: Option<&str>, mode: u32) -> Home {
        let dir = PathBuf::from(format!("/tmp/tbs-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir(&dir).unwrap();
        chmod(&dir, 0o700);
        if let Some(s) = script {
            std::fs::write(wrapper(&dir), format!("#!/bin/sh\n{s}\n")).unwrap();
            chmod(&wrapper(&dir), mode);
        }
        Home(dir)
    }
    fn chmod(p: &Path, mode: u32) {
        std::fs::set_permissions(p, std::fs::Permissions::from_mode(mode)).unwrap();
    }
    fn stale(p: &Path) {
        drop(UnixListener::bind(p).unwrap());
        assert!(p.exists());
    }
    const COUNT: &str = "echo x >> \"$(dirname \"$0\")/ran\"";
    fn runs(h: &Home) -> usize {
        std::fs::read_to_string(h.0.join("ran")).map_or(0, |s| s.lines().count())
    }

    #[test]
    fn says_how_to_install_without_a_wrapper() {
        let h = home("none", None, 0);
        let sock = h.0.join("s");
        stale(&sock);
        let got = Starter::default().connect(&h.0, &sock, true, || false);
        assert_eq!(got.unwrap_err(), NOT_INSTALLED);
    }

    #[test]
    fn refuses_a_wrapper_others_could_change() {
        for mode in [0o775, 0o757, 0o644] {
            let h = home(&format!("mode{mode:o}"), Some(COUNT), mode);
            assert!(!can_start(&h.0), "{mode:o}");
        }
        let h = home("dir", Some(COUNT), 0o755);
        assert!(can_start(&h.0));
        chmod(&h.0, 0o777);
        assert!(!can_start(&h.0));
        let h = home("link", None, 0);
        symlink("/bin/sh", wrapper(&h.0)).unwrap();
        assert!(!can_start(&h.0));
    }

    #[test]
    fn doesnt_start_it_unasked() {
        let h = home("unasked", Some(COUNT), 0o755);
        let got = Starter::default().connect(&h.0, &h.0.join("s"), false, || false);
        assert_eq!(got.unwrap_err(), NOT_STARTED);
        assert_eq!(runs(&h), 0);
    }

    #[test]
    fn stale_and_missing_sockets_start_one_service_between_two_connects() {
        for left in [true, false] {
            let h = home(if left { "stale" } else { "missing" }, Some(COUNT), 0o755);
            let sock = h.0.join("s");
            if left {
                stale(&sock);
            }
            let starter = Starter::default();
            std::thread::scope(|s| {
                let a = s.spawn(|| starter.connect(&h.0, &sock, true, || false).is_ok());
                let b = s.spawn(|| starter.connect(&h.0, &sock, true, || false).is_ok());
                while runs(&h) == 0 {
                    std::thread::sleep(Duration::from_millis(10));
                }
                let _ = std::fs::remove_file(&sock);
                let _service = UnixListener::bind(&sock).unwrap();
                assert!(a.join().unwrap() && b.join().unwrap());
            });
            assert_eq!(runs(&h), 1);
        }
    }

    #[test]
    fn a_service_that_wont_start_is_said_and_not_tried_again_until_asked() {
        let h = home("fails", Some(COUNT), 0o755);
        let sock = h.0.join("s");
        let starter = Starter::default();
        for _ in 0..2 {
            let got = starter.connect(&h.0, &sock, true, || false);
            assert_eq!(got.unwrap_err(), NOT_STARTED);
        }
        assert_eq!(runs(&h), 1);
        starter.forget();
        assert!(starter.connect(&h.0, &sock, true, || false).is_err());
        assert_eq!(runs(&h), 2);
    }
}
