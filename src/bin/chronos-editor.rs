//! Chronos Engine Editor — Desktop application entry point.
//!
//! Thin wrapper around [`EditorApp`]. All logic lives in the editor module.
//!
//! Uses winit 0.30's [`ApplicationHandler`] trait for the event loop.

use chronos_engine::{EditorApp, EditorError};
use std::path::PathBuf;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::WindowId;

/// Install a panic hook that logs to stderr and a persistent crash log file.
fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let msg = format!("[PANIC] {info}\n");
        eprint!("{msg}");
        // Try to write to a crash log in the user's config dir.
        if let Some(config_dir) = chronos_engine::platform::config_dir() {
            let _ = std::fs::create_dir_all(&config_dir);
            let crash_path = config_dir.join("crash.log");
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs().to_string())
                .unwrap_or_else(|_| "unknown".into());
            let _ = std::fs::write(&crash_path, format!("{} {}\n", timestamp, msg));
        }
        default_hook(info);
    }));
}

/// Ensure the working directory is sensible when launched from a shortcut.
/// If the current dir is not writable or looks like a system dir, switch to
/// the user's home directory.
fn normalize_working_directory() {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(PathBuf::from));

    let cwd = std::env::current_dir().ok();
    let home = std::env::var_os("HOME").map(PathBuf::from);

    // Detect if we're in a "bad" launch directory (common when launched from
    // app launcher / walker on Linux). Use exact ancestor checks rather than
    // string prefix matching so /usr/share does not match /usr/local.
    let bad_paths: &[PathBuf] = &[
        PathBuf::from("/"),
        PathBuf::from("/usr"),
        PathBuf::from("/usr/bin"),
        PathBuf::from("/usr/local/bin"),
        PathBuf::from("/usr/local"),
        PathBuf::from("/bin"),
    ];
    let in_bad_dir = cwd
        .as_ref()
        .map(|p| {
            bad_paths
                .iter()
                .any(|bad| p == bad || p.ancestors().any(|a| a == bad))
        })
        .unwrap_or(true);

    let target = if in_bad_dir {
        home.as_ref().or(exe_dir.as_ref())
    } else {
        cwd.as_ref().or(home.as_ref())
    };

    if let Some(dir) = target {
        eprintln!(
            "[chronos-editor] normalized working directory to: {}",
            dir.display()
        );
        let _ = std::env::set_current_dir(dir);
    }
}

fn main() -> Result<(), EditorError> {
    install_panic_hook();
    normalize_working_directory();

    let event_loop = EventLoop::new().map_err(|e| EditorError::Other(e.to_string()))?;
    let mut app = EditorAppHandler::new(&event_loop)?;

    event_loop
        .run_app(&mut app)
        .map_err(|e| EditorError::Other(e.to_string()))?;

    Ok(())
}

/// winit 0.30 [`ApplicationHandler`] that delegates to [`EditorApp`].
struct EditorAppHandler {
    app: Option<EditorApp>,
    suspended: bool,
}

impl EditorAppHandler {
    fn new(event_loop: &EventLoop<()>) -> Result<Self, EditorError> {
        let app = EditorApp::new(event_loop)?;
        Ok(Self {
            app: Some(app),
            suspended: false,
        })
    }
}

impl ApplicationHandler for EditorAppHandler {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {
        self.suspended = false;
        // Recover the wgpu surface after suspend/resume, then request a redraw.
        if let Some(app) = self.app.as_mut() {
            if let Err(e) = app.recover_surface() {
                eprintln!("[chronos-editor] surface recovery failed: {e}");
            }
            app.window().request_redraw();
        }
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        self.suspended = true;
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(app) = self.app.as_mut() else { return };

        match &event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                if size.width > 0 && size.height > 0 {
                    app.handle_resize(*size);
                }
            }
            WindowEvent::RedrawRequested => {
                if !self.suspended {
                    if let Err(e) = app.render() {
                        eprintln!("Render error: {e}");
                        // On fatal render errors, try one recovery render
                        // before giving up for this frame.
                        if let Err(e2) = app.render() {
                            eprintln!("Render recovery failed: {e2}");
                        }
                    }
                }
            }
            _ => {
                app.handle_window_event(&event);
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let Some(app) = self.app.as_mut() else { return };

        // If the editor state signals quit (e.g. from Ctrl+Q shortcut),
        // exit cleanly instead of hanging.
        if app.should_quit() {
            event_loop.exit();
            return;
        }

        app.window().request_redraw();
    }
}
