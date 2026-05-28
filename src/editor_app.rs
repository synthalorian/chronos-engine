//! Chronos Engine Editor — Desktop application core (Phase 7A + 7B).
//!
//! Provides [`EditorApp`] which manages a winit window, wgpu rendering surface,
//! and egui immediate-mode UI with all editor panels.
//! All editor code is gated behind the `editor` feature.

use std::num::NonZeroU32;
use std::sync::Arc;

use egui::{ViewportId, Visuals};
use egui_wgpu::WgpuConfiguration;
use winit::dpi::PhysicalSize;
use winit::event::WindowEvent;
use winit::event_loop::EventLoop;
use winit::window::Window;

use crate::editor_panels::{
    AssetBrowserPanel, ConsolePanel, EditorPanel, EditorState, HierarchyPanel, InspectorPanel,
    MenuBarPanel, ToolbarPanel, ViewportPanel, WelcomeScreen,
};
use crate::editor_workspace::{
    GizmoSystem, GridRenderer, SettingsDialog, ShortcutAction, ShortcutMap, UndoStack,
    ViewportSelector,
};

// ──────────────────────────────────────────────
// EditorError
// ──────────────────────────────────────────────

/// Errors that can occur during editor operation.
#[derive(Debug)]
pub enum EditorError {
    /// Failed to create the editor window.
    WindowCreation(String),
    /// Failed to create the wgpu surface.
    SurfaceCreation(String),
    /// No compatible GPU adapter found.
    NoAdapter,
    /// Failed to request a wgpu device.
    DeviceRequest(String),
    /// Failed to acquire the next surface texture for rendering.
    SurfaceTexture(String),
    /// Other / miscellaneous error.
    Other(String),
}

impl std::fmt::Display for EditorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EditorError::WindowCreation(msg) => {
                write!(f, "Failed to create editor window: {msg}")
            }
            EditorError::SurfaceCreation(msg) => {
                write!(f, "Failed to create wgpu surface: {msg}")
            }
            EditorError::NoAdapter => write!(f, "No compatible GPU adapter found"),
            EditorError::DeviceRequest(msg) => {
                write!(f, "Failed to request wgpu device: {msg}")
            }
            EditorError::SurfaceTexture(msg) => {
                write!(f, "Failed to acquire surface texture: {msg}")
            }
            EditorError::Other(msg) => write!(f, "Editor error: {msg}"),
        }
    }
}

impl std::error::Error for EditorError {}

// ──────────────────────────────────────────────
// EditorApp
// ──────────────────────────────────────────────

/// Core editor application — owns the window, wgpu surface, and egui state.
///
/// Typical usage:
/// ```ignore
/// let event_loop = EventLoop::new().unwrap();
/// let mut app = EditorApp::new(&event_loop)?;
///
/// event_loop.run(move |event, elwt| {
///     match event {
///         Event::AboutToWait => app.window().request_redraw(),
///         Event::WindowEvent { event, .. } => match &event {
///             WindowEvent::CloseRequested => elwt.exit(),
///             WindowEvent::Resized(size) => app.handle_resize(*size),
///             _ => { app.handle_window_event(&event); }
///         },
///         Event::RedrawRequested(_) => { let _ = app.render(); }
///         _ => {}
///     }
/// })?;
/// ```
pub struct EditorApp {
    window: Arc<Window>,

    egui_ctx: egui::Context,
    egui_winit: egui_winit::State,
    egui_painter: egui_wgpu::winit::Painter,
    viewport_id: ViewportId,

    // ── Editor state & panels (Phase 7B) ──
    state: EditorState,
    menu_bar: MenuBarPanel,
    toolbar: ToolbarPanel,
    viewport: ViewportPanel,
    hierarchy: HierarchyPanel,
    inspector: InspectorPanel,
    asset_browser: AssetBrowserPanel,
    console: ConsolePanel,

    undo_stack: UndoStack,
    grid_renderer: GridRenderer,
    gizmo_system: GizmoSystem,
    viewport_selector: ViewportSelector,
    shortcut_map: ShortcutMap,
    settings_dialog: SettingsDialog,
    welcome: WelcomeScreen,
}

impl EditorApp {
    /// Default editor window width in logical pixels.
    pub const DEFAULT_WIDTH: u32 = 1280;

    /// Default editor window height in logical pixels.
    pub const DEFAULT_HEIGHT: u32 = 720;

    /// Window title.
    pub const WINDOW_TITLE: &str = "Chronos Engine Editor";

    /// Clear color for the background (dark charcoal).
    pub const CLEAR_COLOR: [f32; 4] = [0.067, 0.067, 0.067, 1.0];

    /// Create the editor: open window, init wgpu via egui-wgpu painter, init egui state.
    ///
    /// Uses `pollster::block_on` for async wgpu init (same pattern as `render.rs`).
    #[allow(deprecated)] // EventLoop::create_window — matches existing render.rs pattern
    pub fn new(event_loop: &EventLoop<()>) -> Result<Self, EditorError> {
        // ── Create window ──
        let window_attrs = Window::default_attributes()
            .with_title(Self::WINDOW_TITLE)
            .with_inner_size(PhysicalSize::new(
                Self::DEFAULT_WIDTH,
                Self::DEFAULT_HEIGHT,
            ));

        let window = Arc::new(
            event_loop
                .create_window(window_attrs)
                .map_err(|e| EditorError::WindowCreation(e.to_string()))?,
        );

        // ── egui context ──
        let egui_ctx = egui::Context::default();
        egui_ctx.set_visuals(Visuals::dark());

        let viewport_id = ViewportId::ROOT;

        // ── egui-wgpu painter (manages wgpu instance, adapter, device, queue, surface) ──
        let wgpu_config = WgpuConfiguration {
            present_mode: wgpu::PresentMode::AutoVsync,
            desired_maximum_frame_latency: Some(2),
            ..Default::default()
        };

        let mut egui_painter = egui_wgpu::winit::Painter::new(
            egui_ctx.clone(),
            wgpu_config,
            1,                      // msaa_samples
            None,                   // depth_format
            false,                  // support_transparent_backbuffer
            false,                  // dithering
        );

        // Async: create surface & initialize render state for the window.
        pollster::block_on(egui_painter.set_window(viewport_id, Some(window.clone())))
            .map_err(|e| EditorError::SurfaceCreation(e.to_string()))?;

        // ── egui-winit state (input bridge) ──
        let max_texture_side = egui_painter.max_texture_side();
        let scale_factor = Some(window.scale_factor() as f32);
        let egui_winit = egui_winit::State::new(
            egui_ctx.clone(),
            viewport_id,
            &window,            // implements HasDisplayHandle
            scale_factor,
            None,               // theme — let egui decide
            max_texture_side,
        );

        Ok(Self {
            window,
            egui_ctx,
            egui_winit,
            egui_painter,
            viewport_id,
            state: EditorState::new(),
            menu_bar: MenuBarPanel::new(),
            toolbar: ToolbarPanel::new(),
            viewport: { let mut v = ViewportPanel::new(); v.grid_visible = false; v },
            hierarchy: HierarchyPanel::new(),
            inspector: InspectorPanel::new(),
            asset_browser: AssetBrowserPanel::new(),
            console: ConsolePanel::new(),
            undo_stack: UndoStack::new(),
            grid_renderer: GridRenderer::new(),
            gizmo_system: GizmoSystem::new(),
            viewport_selector: ViewportSelector::new(),
            shortcut_map: ShortcutMap::new(),
            settings_dialog: SettingsDialog::new(),
            welcome: WelcomeScreen::new(),
        })
    }

    /// Handle a winit `WindowEvent`. Feeds input to egui_winit.
    ///
    /// Returns `true` if egui requests a repaint as a result of this event.
    pub fn handle_window_event(&mut self, event: &WindowEvent) -> bool {
        let response = self.egui_winit.on_window_event(&self.window, event);
        response.repaint
    }

    /// Handle window resize: reconfigure the wgpu surface and notify the egui painter.
    ///
    /// Called when the window is resized by the user or the WM.
    pub fn handle_resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            let width = NonZeroU32::new(new_size.width).unwrap_or(NonZeroU32::MIN);
            let height = NonZeroU32::new(new_size.height).unwrap_or(NonZeroU32::MIN);
            self.egui_painter
                .on_window_resized(self.viewport_id, width, height);
        }
    }

    /// Render one frame: run egui UI with all panels, tessellate, paint via wgpu.
    pub fn render(&mut self) -> Result<(), EditorError> {
        let raw_input = self.egui_winit.take_egui_input(&self.window);

        let ctx = self.egui_ctx.clone();
        let state = &mut self.state;
        let menu_bar = &mut self.menu_bar;
        let toolbar = &mut self.toolbar;
        let viewport = &mut self.viewport;
        let hierarchy = &mut self.hierarchy;
        let inspector = &mut self.inspector;
        let asset_browser = &mut self.asset_browser;
        let console = &mut self.console;
        let undo_stack = &mut self.undo_stack;
        let grid_renderer = &mut self.grid_renderer;
        let gizmo_system = &mut self.gizmo_system;
        let viewport_selector = &mut self.viewport_selector;
        let shortcut_map = &mut self.shortcut_map;
        let settings_dialog = &mut self.settings_dialog;
        let welcome = &mut self.welcome;

        let full_output = ctx.run(raw_input, |ctx| {
            ctx.input(|i| {
                for event in &i.events {
                    if let egui::Event::Key { key, pressed: true, modifiers, .. } = event {
                        if let Some(action) = shortcut_map.find_action(*key, modifiers.ctrl, modifiers.shift, modifiers.alt) {
                            match action {
                                ShortcutAction::Undo => {
                                    if let Some(desc) = undo_stack.undo(&mut state.world) {
                                        state.log(crate::editor_panels::ConsoleLogLevel::Info, format!("Undo: {desc}"));
                                    }
                                }
                                ShortcutAction::Redo => {
                                    if let Some(desc) = undo_stack.redo(&mut state.world) {
                                        state.log(crate::editor_panels::ConsoleLogLevel::Info, format!("Redo: {desc}"));
                                    }
                                }
                                ShortcutAction::GizmoTranslate => state.gizmo_mode = crate::editor_panels::GizmoMode::Translate,
                                ShortcutAction::GizmoRotate => state.gizmo_mode = crate::editor_panels::GizmoMode::Rotate,
                                ShortcutAction::GizmoScale => state.gizmo_mode = crate::editor_panels::GizmoMode::Scale,
                                ShortcutAction::ToggleSnap => state.snap_enabled = !state.snap_enabled,
                                ShortcutAction::ToggleGrid => grid_renderer.visible = !grid_renderer.visible,
                                ShortcutAction::Delete => {
                                    if !state.selected_entities.is_empty() {
                                        state.log(crate::editor_panels::ConsoleLogLevel::Info,
                                            format!("Delete: {} entities", state.selected_entities.len()));
                                        state.selected_entities.clear();
                                    }
                                }
                                ShortcutAction::Deselect => state.clear_selection(),
                                ShortcutAction::PlayStop => {
                                    state.play_mode = match state.play_mode {
                                        crate::editor_panels::PlayMode::Stopped => crate::editor_panels::PlayMode::Playing,
                                        _ => crate::editor_panels::PlayMode::Stopped,
                                    };
                                }
                                ShortcutAction::Quit => state.should_quit = true,
                                ShortcutAction::NewProject => {
                                    state.project_manager.show_new_wizard = true;
                                    state.log(crate::editor_panels::ConsoleLogLevel::Info, "New project wizard");
                                }
                                ShortcutAction::OpenProject => {
                                    state.project_manager.show_open_dialog = true;
                                    state.log(crate::editor_panels::ConsoleLogLevel::Info, "Open project dialog");
                                }
                                ShortcutAction::Save => {
                                    if state.project_manager.is_loaded() {
                                        match state.project_manager.save_current() {
                                            Ok(()) => state.log(crate::editor_panels::ConsoleLogLevel::Info, "Project saved"),
                                            Err(e) => state.log(crate::editor_panels::ConsoleLogLevel::Error, format!("Save failed: {e}")),
                                        }
                                    } else {
                                        state.log(crate::editor_panels::ConsoleLogLevel::Warn, "No project loaded to save");
                                    }
                                }
                                ShortcutAction::SaveAs => {
                                    state.log(crate::editor_panels::ConsoleLogLevel::Info, "Save As dialog");
                                }
                                _ => {}
                            }
                        }
                    }
                }
            });

            // ── Menu bar (top) ──
            egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
                menu_bar.show(ui, state);
            });

            // ── Toolbar (below menu bar) ──
            egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
                toolbar.show(ui, state);
            });

            settings_dialog.visible = state.show_settings;
            settings_dialog.show(ctx);
            state.show_settings = settings_dialog.visible;

            // ── Hierarchy (left) ──
            egui::SidePanel::left("hierarchy")
                .min_width(180.0)
                .default_width(220.0)
                .show(ctx, |ui| {
                    ui.heading("Hierarchy");
                    ui.separator();
                    hierarchy.show(ui, state);
                });

            // ── Inspector (right) ──
            egui::SidePanel::right("inspector")
                .min_width(200.0)
                .default_width(280.0)
                .show(ctx, |ui| {
                    ui.heading("Inspector");
                    ui.separator();
                    inspector.show(ui, state);
                });

            // ── Bottom panel: Console + Asset Browser ──
            egui::TopBottomPanel::bottom("bottom_panel")
                .min_height(120.0)
                .default_height(200.0)
                .resizable(true)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut state.show_console_tab, true, "Console");
                        ui.selectable_value(&mut state.show_console_tab, false, "Asset Browser");
                    });
                    ui.separator();
                    if state.show_console_tab {
                        console.show(ui, state);
                    } else {
                        asset_browser.show(ui, state);
                    }
                });

            // ── Central area: Welcome screen (no project) or Viewport (project loaded) ──
            if !state.project_manager.is_loaded() {
                egui::CentralPanel::default().show(ctx, |ui| {
                    welcome.show(ui, state);
                });
            } else {
                egui::CentralPanel::default().show(ctx, |ui| {
                    viewport.show(ui, state);

                    if let Some(vp_rect) = viewport.last_viewport_rect {
                        let painter = ui.painter_at(vp_rect);
                        let center = vp_rect.center();

                        let cam_offset = [viewport.camera_target[0], viewport.camera_target[2]];
                        let zoom = 1.0 / viewport.camera_distance.max(0.1);

                        grid_renderer.render(&painter, cam_offset, zoom, vp_rect);

                        gizmo_system.mode = state.gizmo_mode;
                        if !state.selected_entities.is_empty() {
                            gizmo_system.render(&painter, center);
                            if let Some(_gizmo_result) = gizmo_system.handle_input(ui, center) {
                                // Gizmo drag detected — delta available for entity transform
                            }
                        }

                        viewport_selector.render_box_select(&painter);
                    }
                });
            }
        });

        self.egui_winit
            .handle_platform_output(&self.window, full_output.platform_output);

        let title = if self.state.project_manager.is_loaded() {
            format!("{} — {}", self.state.project_manager.project_name(), Self::WINDOW_TITLE)
        } else {
            Self::WINDOW_TITLE.to_string()
        };
        self.window.set_title(&title);

        let shapes = full_output.shapes;
        let textures_delta = full_output.textures_delta;
        let pixels_per_point = full_output.pixels_per_point;

        let clipped_primitives = self.egui_ctx.tessellate(shapes, pixels_per_point);

        self.egui_painter.paint_and_update_textures(
            self.viewport_id,
            pixels_per_point,
            Self::CLEAR_COLOR,
            &clipped_primitives,
            &textures_delta,
            vec![],
        );

        if self.state.should_quit {
            // Caller should check this and exit the event loop.
        }

        Ok(())
    }

    /// Access the underlying window (for `request_redraw` etc.).
    pub fn window(&self) -> &Window {
        &self.window
    }

    /// Access the egui context.
    pub fn egui_ctx(&self) -> &egui::Context {
        &self.egui_ctx
    }
}

// ──────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── EditorError display tests ──

    #[test]
    fn editor_error_window_creation_display() {
        let err = EditorError::WindowCreation("bad handle".into());
        let msg = format!("{err}");
        assert!(msg.contains("Failed to create editor window"));
        assert!(msg.contains("bad handle"));
    }

    #[test]
    fn editor_error_surface_creation_display() {
        let err = EditorError::SurfaceCreation("no surface".into());
        let msg = format!("{err}");
        assert!(msg.contains("Failed to create wgpu surface"));
        assert!(msg.contains("no surface"));
    }

    #[test]
    fn editor_error_no_adapter_display() {
        let err = EditorError::NoAdapter;
        let msg = format!("{err}");
        assert!(msg.contains("No compatible GPU adapter"));
    }

    #[test]
    fn editor_error_device_request_display() {
        let err = EditorError::DeviceRequest("timeout".into());
        let msg = format!("{err}");
        assert!(msg.contains("Failed to request wgpu device"));
        assert!(msg.contains("timeout"));
    }

    #[test]
    fn editor_error_surface_texture_display() {
        let err = EditorError::SurfaceTexture("lost".into());
        let msg = format!("{err}");
        assert!(msg.contains("Failed to acquire surface texture"));
        assert!(msg.contains("lost"));
    }

    #[test]
    fn editor_error_other_display() {
        let err = EditorError::Other("something went wrong".into());
        let msg = format!("{err}");
        assert!(msg.contains("Editor error"));
        assert!(msg.contains("something went wrong"));
    }

    // ── Structural / constant tests ──

    #[test]
    fn editor_defaults() {
        assert_eq!(EditorApp::DEFAULT_WIDTH, 1280);
        assert_eq!(EditorApp::DEFAULT_HEIGHT, 720);
        assert_eq!(EditorApp::WINDOW_TITLE, "Chronos Engine Editor");
    }

    #[test]
    fn clear_color_is_dark() {
        let [r, g, b, a] = EditorApp::CLEAR_COLOR;
        assert!(r < 0.1, "red channel should be dark");
        assert!(g < 0.1, "green channel should be dark");
        assert!(b < 0.1, "blue channel should be dark");
        assert!((a - 1.0).abs() < f32::EPSILON, "alpha should be 1.0");
    }

    // ── Error chain test ──

    #[test]
    fn editor_error_implements_std_error() {
        let err = EditorError::Other("test".into());
        let _boxed: Box<dyn std::error::Error> = Box::new(err);
    }

    #[test]
    fn editor_error_debug_format() {
        let err = EditorError::NoAdapter;
        let debug = format!("{err:?}");
        assert!(debug.contains("NoAdapter"));
    }
}
