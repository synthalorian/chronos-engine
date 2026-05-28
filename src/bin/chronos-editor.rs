//! Chronos Engine Editor — Desktop application entry point.
//!
//! Thin wrapper around [`EditorApp`]. All logic lives in the editor module.
//!
//! Uses winit 0.30's [`ApplicationHandler`] trait for the event loop.

use chronos_engine::{EditorApp, EditorError};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::WindowId;

fn main() -> Result<(), EditorError> {
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
}

impl EditorAppHandler {
    fn new(event_loop: &EventLoop<()>) -> Result<Self, EditorError> {
        let app = EditorApp::new(event_loop)?;
        Ok(Self { app: Some(app) })
    }
}

impl ApplicationHandler for EditorAppHandler {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {
        // Window is already created in EditorApp::new(). Nothing to do here.
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
                app.handle_resize(*size);
            }
            WindowEvent::RedrawRequested => {
                if let Err(e) = app.render() {
                    eprintln!("Render error: {e}");
                }
            }
            _ => {
                app.handle_window_event(&event);
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        let Some(app) = self.app.as_mut() else { return };
        app.window().request_redraw();
    }
}
