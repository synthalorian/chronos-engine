//! WASM / Web entry point for the Chronos Engine.
//!
//! Provides the browser-facing `run_chronos_web()` function that
//! initializes the engine on a `<canvas>` element, starts the winit
//! event loop, and bridges browser events (keyboard, mouse, touch,
//! resize) into the engine's input system.
//!
//! # Building
//!
//! ```bash
//! cargo build --target wasm32-unknown-unknown --features web
//! wasm-pack build --target web --features web
//! ```
//!
//! # Usage
//!
//! ```html
//! <canvas id="chronos-canvas"></canvas>
//! <script type="module">
//!   import init, { run_chronos_web } from './chronos_engine.js';
//!   await init();
//!   run_chronos_web('chronos-canvas');
//! </script>
//! ```

use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::prelude::*;
use winit::platform::web::WindowExtWebSys;

use crate::editor_app::EditorApp;

/// Called from JavaScript to start the engine on a given canvas element.
///
/// `canvas_id` — the `id` attribute of the `<canvas>` element.
///
/// This function sets up the winit event loop on the canvas, builds the
/// editor app, and enters the event loop. It never returns (the event loop
/// runs forever).
#[wasm_bindgen]
pub fn run_chronos_web(canvas_id: &str) {
    // Install panic hook that logs to console and fires a custom DOM event
    console_error_panic_hook::set_once();

    // Initialize logger (maps log::info! etc. to console.log)
    #[cfg(debug_assertions)]
    console_log::init_with_level(log::Level::Debug).ok();
    #[cfg(not(debug_assertions))]
    console_log::init_with_level(log::Level::Info).ok();

    // Set up the canvas
    let canvas = match web_sys::window()
        .and_then(|w| w.document())
        .and_then(|doc| doc.get_element_by_id(canvas_id))
        .and_then(|el| el.dyn_into::<web_sys::HtmlCanvasElement>().ok())
    {
        Some(c) => c,
        None => {
            fire_error(&format!("Canvas element '#{}' not found", canvas_id));
            return;
        }
    };

    // If the canvas has no explicit size, default to 800×600
    if canvas.width() == 0 || canvas.height() == 0 {
        canvas.set_width(800);
        canvas.set_height(600);
    }

    // Build the winit event loop (WASM-compatible)
    let event_loop = match winit::event_loop::EventLoop::new() {
        Ok(el) => el,
        Err(e) => {
            fire_error(&format!("Failed to create event loop: {}", e));
            return;
        }
    };

    // Build the EditorApp (renderer, world, etc.)
    // The window is created inside EditorApp::new() — on WASM the surface
    // init is deferred (egui_painter is None).
    let app = match EditorApp::new(&event_loop) {
        Ok(a) => a,
        Err(e) => {
            fire_error(&format!("Engine initialization failed: {}", e));
            return;
        }
    };

    // Attach the canvas to the EditorApp's window (required for WASM rendering)
    app.window().set_canvas(Some(canvas.clone().into()));

    // Set canvas size to match CSS size for HiDPI
    let scale_factor = app.window().scale_factor();
    if let Some(rect) = canvas.get_bounding_client_rect() {
        let width = (rect.width() * scale_factor) as u32;
        let height = (rect.height() * scale_factor) as u32;
        if width > 0 && height > 0 {
            let _ = app.window().request_inner_size(winit::dpi::LogicalSize::new(
                width as f64 / scale_factor,
                height as f64 / scale_factor,
            ));
        }
    }

    // Wrap app in Rc<RefCell<>> so the winit event loop handler and the
    // deferred async surface init can both access it.
    let app_rc = Rc::new(RefCell::new(app));

    // Enter the winit event loop with our ApplicationHandler.
    let result = event_loop.run_app(&mut EditorAppHandler(app_rc));
    if let Err(e) = result {
        fire_error(&format!("Event loop exited with error: {}", e));
    }
}

/// Thin wrapper that delegates winit events to EditorApp.
///
/// Uses `Rc<RefCell<EditorApp>>` so the handler can share access with
/// deferred async operations (e.g. `wasm_bindgen_futures::spawn_local`
/// for surface initialization).
struct EditorAppHandler(Rc<RefCell<EditorApp>>);

impl winit::application::ApplicationHandler for EditorAppHandler {
    fn resumed(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        // On WASM, the wgpu surface must be initialized asynchronously.
        // Spawn a future that creates the egui-wgpu painter and attaches
        // the rendering surface to the window.
        //
        // `Rc<RefCell<>>` is safe on WASM (single-threaded) and allows
        // the handler and the spawned future to share the EditorApp.
        let app_rc = self.0.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let mut app = app_rc.borrow_mut();
            match app.init_surface_wasm().await {
                Ok(()) => {
                    log::info!("WASM surface initialized");
                    app.window().request_redraw();
                }
                Err(e) => {
                    // Surface init failure is fatal — the canvas will stay
                    // blank. Fire a DOM event so the web page's error
                    // display activates.
                    fire_error(&format!("WASM surface init failed: {}", e));
                }
            }
        });
    }

    fn suspended(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {}

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let mut app = self.0.borrow_mut();
        match &event {
            winit::event::WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            winit::event::WindowEvent::Resized(size) => {
                if size.width > 0 && size.height > 0 {
                    app.handle_resize(*size);
                }
            }
            winit::event::WindowEvent::RedrawRequested => {
                if let Err(e) = app.render() {
                    log::error!("Render error: {}", e);
                }
            }
            _ => {
                app.handle_window_event(&event);
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let app = self.0.borrow();
        if app.should_quit() {
            event_loop.exit();
            return;
        }
        app.window().request_redraw();
    }
}

/// Fire a custom DOM error event so the JavaScript layer can display it.
fn fire_error(msg: &str) {
    log::error!("{}", msg);
    if let Some(window) = web_sys::window() {
        let detail = js_sys::Object::new();
        js_sys::Reflect::set(&detail, &"message".into(), &msg.into()).ok();
        let event_init = web_sys::CustomEventInit::new();
        event_init.set_detail(&detail);
        let event = web_sys::CustomEvent::new_with_event_init_dict("chronos-error", &event_init);
        if let Ok(evt) = event {
            let _ = window.dispatch_event(&evt);
        }
    }
}
