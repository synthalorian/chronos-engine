//! Inspector panel — Phase 7B.
//!
//! Displays and edits components attached to the currently selected entity.
//! Supports drag-value editing, component add/remove, and multi-selection info.
//!
//! # Component inspection strategy
//!
//! Since the ECS uses type-erased storage, we cannot iterate components
//! generically. Instead, the inspector probes each known component type
//! individually via [`World::has_component`] / [`World::get_component_mut`].

use super::{EditorPanel, EditorState};
use crate::component::{
    CircleRadius, Damage, Dead, Gravity, Grounded, Health, Position, RigidBody, Sprite, Transform,
    Velocity,
};
use crate::entity::Entity;

// ──────────────────────────────────────────────
// Known component registry
// ──────────────────────────────────────────────

/// All known component type names in canonical display order.
const KNOWN_COMPONENTS: &[&str] = &[
    "Position",
    "Velocity",
    "Health",
    "Transform",
    "Sprite",
    "RigidBody",
    "CircleRadius",
    "Gravity",
    "Damage",
    "Dead",
    "Grounded",
];

// ──────────────────────────────────────────────
// InspectorPanel
// ──────────────────────────────────────────────

/// The Inspector panel — displays component details for the selected entity.
///
/// When a single entity is selected, the inspector lists every component
/// attached to it with editable fields. Tag components ([`Dead`], [`Grounded`])
/// show a read-only label. An "Add Component" section at the bottom lets the
/// user attach new components from the known set.
pub struct InspectorPanel {
    /// Component section names that are currently expanded.
    expanded_components: Vec<String>,
}

impl InspectorPanel {
    /// Create a new inspector with all component sections expanded by default.
    pub fn new() -> Self {
        Self {
            expanded_components: KNOWN_COMPONENTS.iter().map(|s| s.to_string()).collect(),
        }
    }

    // ── Expansion helpers ──

    /// Check whether a component section is currently expanded.
    fn is_expanded(&self, name: &str) -> bool {
        self.expanded_components.iter().any(|s| s == name)
    }

    /// Toggle the expansion state of a component section.
    fn toggle_expanded(&mut self, name: &str) {
        if self.is_expanded(name) {
            self.expanded_components.retain(|s| s != name);
        } else {
            self.expanded_components.push(name.to_string());
        }
    }

    // ── Generic component inspector ──
    //
    // Renders a collapsible header for a known component type `T`. The `edit_ui`
    // closure receives `(&mut Ui, &mut T)` and draws the editable fields.
    // Removal is deferred until after the collapsing header closes so that the
    // mutable borrow on the component does not conflict with `remove_component`.

    fn show_component<T: Send + Sync + 'static>(
        &mut self,
        ui: &mut egui::Ui,
        state: &mut EditorState,
        entity: Entity,
        name: &str,
        edit_ui: impl FnOnce(&mut egui::Ui, &mut T),
    ) {
        if !state.world.has_component::<T>(entity) {
            return;
        }
        let expanded = self.is_expanded(name);
        let mut should_remove = false;
        egui::CollapsingHeader::new(name)
            .default_open(expanded)
            .show(ui, |ui| {
                if let Some(comp) = state.world.get_component_mut::<T>(entity) {
                    edit_ui(ui, comp);
                }
                ui.add_space(2.0);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    should_remove = ui.small_button("Remove").clicked();
                });
            });
        if should_remove {
            state.world.remove_component::<T>(entity);
        }
    }
}

impl Default for InspectorPanel {
    fn default() -> Self {
        Self::new()
    }
}

// ──────────────────────────────────────────────
// EditorPanel implementation
// ──────────────────────────────────────────────

impl EditorPanel for InspectorPanel {
    fn title(&self) -> &str {
        "Inspector"
    }

    fn show(&mut self, ui: &mut egui::Ui, state: &mut EditorState) {
        // ── No selection ──
        if state.selected_entities.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(24.0);
                ui.label("No entity selected");
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new("Select an entity in the Hierarchy panel")
                        .small()
                        .weak(),
                );
            });
            return;
        }

        // ── Multi-selection ──
        if state.selected_entities.len() > 1 {
            let n = state.selected_entities.len();
            ui.label(format!("{n} entities selected"));
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new("Multi-edit not yet supported")
                    .small()
                    .weak(),
            );
            return;
        }

        // ── Single entity ──
        let entity = state.selected_entities[0];
        if !state.world.entity_exists(entity) {
            ui.colored_label(egui::Color32::RED, "Entity no longer exists");
            state.clear_selection();
            return;
        }

        // Entity header.
        ui.heading(format!("Entity {}:{}", entity.index(), entity.generation()));
        ui.separator();
        ui.add_space(4.0);

        // ── Component inspectors ──

        self.show_component::<Position>(ui, state, entity, "Position", |ui, pos| {
            ui.horizontal(|ui| {
                ui.label("x");
                ui.add(
                    egui::DragValue::new(&mut pos.x)
                        .range(-1000.0..=1000.0)
                        .speed(0.1),
                );
                ui.label("y");
                ui.add(
                    egui::DragValue::new(&mut pos.y)
                        .range(-1000.0..=1000.0)
                        .speed(0.1),
                );
            });
        });

        self.show_component::<Velocity>(ui, state, entity, "Velocity", |ui, vel| {
            ui.horizontal(|ui| {
                ui.label("x");
                ui.add(
                    egui::DragValue::new(&mut vel.x)
                        .range(-100.0..=100.0)
                        .speed(0.1),
                );
                ui.label("y");
                ui.add(
                    egui::DragValue::new(&mut vel.y)
                        .range(-100.0..=100.0)
                        .speed(0.1),
                );
            });
        });

        self.show_component::<Health>(ui, state, entity, "Health", |ui, hp| {
            ui.horizontal(|ui| {
                ui.label("current");
                ui.add(egui::DragValue::new(&mut hp.current).range(0..=10000));
                ui.label("max");
                ui.add(egui::DragValue::new(&mut hp.max).range(1..=10000));
            });
            let pct = if hp.max > 0 {
                hp.current as f32 / hp.max as f32
            } else {
                0.0
            };
            ui.add(egui::ProgressBar::new(pct).text(format!("{:.0}%", pct * 100.0)));
        });

        self.show_component::<Transform>(ui, state, entity, "Transform", |ui, tr| {
            ui.horizontal(|ui| {
                ui.label("x");
                ui.add(
                    egui::DragValue::new(&mut tr.x)
                        .range(-1000.0..=1000.0)
                        .speed(0.1),
                );
                ui.label("y");
                ui.add(
                    egui::DragValue::new(&mut tr.y)
                        .range(-1000.0..=1000.0)
                        .speed(0.1),
                );
                ui.label("z");
                ui.add(
                    egui::DragValue::new(&mut tr.z)
                        .range(-1000.0..=1000.0)
                        .speed(0.1),
                );
            });
            ui.horizontal(|ui| {
                ui.label("rotation");
                ui.add(
                    egui::DragValue::new(&mut tr.rotation)
                        .range(-360.0..=360.0)
                        .speed(1.0),
                );
                ui.label("scale");
                ui.add(
                    egui::DragValue::new(&mut tr.scale)
                        .range(0.01..=100.0)
                        .speed(0.01),
                );
            });
        });

        self.show_component::<Sprite>(ui, state, entity, "Sprite", |ui, spr| {
            ui.horizontal(|ui| {
                ui.label("symbol");
                let mut sym = spr.symbol.to_string();
                if ui
                    .add(egui::TextEdit::singleline(&mut sym).desired_width(30.0))
                    .changed()
                {
                    spr.symbol = sym.chars().next().unwrap_or('?');
                }
                ui.label("layer");
                ui.add(egui::DragValue::new(&mut spr.layer).range(-10..=10));
            });
            ui.horizontal(|ui| {
                ui.label("R");
                ui.add(egui::DragValue::new(&mut spr.color.0).range(0..=255));
                ui.label("G");
                ui.add(egui::DragValue::new(&mut spr.color.1).range(0..=255));
                ui.label("B");
                ui.add(egui::DragValue::new(&mut spr.color.2).range(0..=255));
            });
            let color = egui::Color32::from_rgb(spr.color.0, spr.color.1, spr.color.2);
            ui.colored_label(color, format!("Preview: {}", color_to_hex(color)));
        });

        self.show_component::<RigidBody>(ui, state, entity, "RigidBody", |ui, rb| {
            ui.horizontal(|ui| {
                ui.label("mass");
                ui.add(
                    egui::DragValue::new(&mut rb.mass)
                        .range(0.0..=1000.0)
                        .speed(0.1),
                );
                ui.label("damping");
                ui.add(
                    egui::DragValue::new(&mut rb.damping)
                        .range(0.0..=1.0)
                        .speed(0.01),
                );
            });
            ui.horizontal(|ui| {
                ui.label("restitution");
                ui.add(
                    egui::DragValue::new(&mut rb.restitution)
                        .range(0.0..=1.0)
                        .speed(0.01),
                );
            });
            ui.label(if rb.is_static() {
                "Static body"
            } else {
                "Dynamic body"
            });
        });

        self.show_component::<CircleRadius>(ui, state, entity, "CircleRadius", |ui, cr| {
            ui.horizontal(|ui| {
                ui.label("radius");
                ui.add(
                    egui::DragValue::new(&mut cr.0)
                        .range(0.0..=100.0)
                        .speed(0.1),
                );
            });
        });

        self.show_component::<Gravity>(ui, state, entity, "Gravity", |ui, g| {
            ui.horizontal(|ui| {
                ui.label("x");
                ui.add(
                    egui::DragValue::new(&mut g.x)
                        .range(-100.0..=100.0)
                        .speed(0.1),
                );
                ui.label("y");
                ui.add(
                    egui::DragValue::new(&mut g.y)
                        .range(-100.0..=100.0)
                        .speed(0.1),
                );
            });
        });

        self.show_component::<Damage>(ui, state, entity, "Damage", |ui, dmg| {
            ui.horizontal(|ui| {
                ui.label("value");
                ui.add(egui::DragValue::new(&mut dmg.0).range(0..=10000));
            });
        });

        // ── Tag components (no editable fields) ──

        if state.world.has_component::<Dead>(entity) {
            ui.horizontal(|ui| {
                ui.colored_label(egui::Color32::RED, "● Entity is dead");
                if ui.small_button("Remove").clicked() {
                    state.world.remove_component::<Dead>(entity);
                }
            });
        }

        if state.world.has_component::<Grounded>(entity) {
            ui.horizontal(|ui| {
                ui.colored_label(egui::Color32::GREEN, "● On ground");
                if ui.small_button("Remove").clicked() {
                    state.world.remove_component::<Grounded>(entity);
                }
            });
        }

        // ── Add Component ──

        ui.add_space(8.0);
        ui.separator();

        ui.collapsing("Add Component", |ui| {
            add_component_button(ui, state, entity, "Position", || Position::new(0.0, 0.0));
            add_component_button(ui, state, entity, "Velocity", || Velocity::new(0.0, 0.0));
            add_component_button(ui, state, entity, "Health", || Health::new(100));
            add_component_button(ui, state, entity, "Transform", || {
                Transform::new(0.0, 0.0, 0.0)
            });
            add_component_button(ui, state, entity, "Sprite", || {
                Sprite::new('?', 255, 255, 255)
            });
            add_component_button(ui, state, entity, "RigidBody", || RigidBody::dynamic(1.0));
            add_component_button(ui, state, entity, "CircleRadius", || CircleRadius::new(1.0));
            add_component_button(ui, state, entity, "Gravity", || Gravity::down(9.81));
            add_component_button(ui, state, entity, "Damage", || Damage(1));
            add_component_button(ui, state, entity, "Dead", || Dead);
            add_component_button(ui, state, entity, "Grounded", || Grounded);
        });
    }
}

// ──────────────────────────────────────────────
// Helper functions
// ──────────────────────────────────────────────

/// Show an "Add {name}" button if the entity does not already have this component.
/// Uses a factory closure to construct the default value (avoids needing `Default`).
fn add_component_button<T: Send + Sync + 'static>(
    ui: &mut egui::Ui,
    state: &mut EditorState,
    entity: Entity,
    name: &str,
    factory: impl FnOnce() -> T,
) {
    if !state.world.has_component::<T>(entity) && ui.button(format!("+ {name}")).clicked() {
        state.world.add_component(entity, factory());
    }
}

/// Format an [`egui::Color32`] as a hex string like `#FF8000`.
fn color_to_hex(c: egui::Color32) -> String {
    format!("#{:02X}{:02X}{:02X}", c.r(), c.g(), c.b())
}

// ──────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── new() defaults ──

    #[test]
    fn new_defaults_all_expanded() {
        let panel = InspectorPanel::new();
        assert_eq!(
            panel.expanded_components.len(),
            KNOWN_COMPONENTS.len(),
            "all component types should start expanded"
        );
        for name in KNOWN_COMPONENTS {
            assert!(
                panel.is_expanded(name),
                "{name} should be expanded by default"
            );
        }
    }

    // ── title() ──

    #[test]
    fn title_returns_inspector() {
        let panel = InspectorPanel::new();
        assert_eq!(panel.title(), "Inspector");
    }

    // ── No-selection state ──

    #[test]
    fn no_selection_state_is_empty() {
        let state = EditorState::new();
        assert!(state.selected_entities.is_empty());
    }

    // ── Component display formatting ──

    #[test]
    fn color_to_hex_format() {
        let c = egui::Color32::from_rgb(255, 128, 0);
        assert_eq!(color_to_hex(c), "#FF8000");
    }

    #[test]
    fn color_to_hex_black() {
        assert_eq!(color_to_hex(egui::Color32::BLACK), "#000000");
    }

    #[test]
    fn color_to_hex_white() {
        assert_eq!(color_to_hex(egui::Color32::WHITE), "#FFFFFF");
    }

    // ── Expanded toggle ──

    #[test]
    fn toggle_expanded_flips_state() {
        let mut panel = InspectorPanel::new();
        assert!(panel.is_expanded("Position"));
        panel.toggle_expanded("Position");
        assert!(
            !panel.is_expanded("Position"),
            "should be collapsed after toggle"
        );
        panel.toggle_expanded("Position");
        assert!(
            panel.is_expanded("Position"),
            "should be expanded after second toggle"
        );
    }

    #[test]
    fn toggle_expanded_no_duplicates() {
        let mut panel = InspectorPanel::new();
        // Double-expand should not create duplicates.
        panel.toggle_expanded("Position"); // collapse
        panel.toggle_expanded("Position"); // expand
        let count = panel
            .expanded_components
            .iter()
            .filter(|s| *s == "Position")
            .count();
        assert_eq!(count, 1, "should not duplicate entries");
    }

    // ── KNOWN_COMPONENTS completeness ──

    #[test]
    fn known_components_covers_all_types() {
        let expected = [
            "Position",
            "Velocity",
            "Health",
            "Transform",
            "Sprite",
            "RigidBody",
            "CircleRadius",
            "Gravity",
            "Damage",
            "Dead",
            "Grounded",
        ];
        for name in &expected {
            assert!(KNOWN_COMPONENTS.contains(name), "missing {name}");
        }
        assert_eq!(KNOWN_COMPONENTS.len(), expected.len());
    }

    // ── Render no-panic tests ──

    #[test]
    fn show_with_no_selection_no_panic() {
        let mut panel = InspectorPanel::new();
        let mut state = EditorState::new();

        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                panel.show(ui, &mut state);
            });
        });
    }

    #[test]
    fn show_with_single_entity_no_panic() {
        let mut panel = InspectorPanel::new();
        let mut state = EditorState::new();

        let entity = state.world.create_entity();
        state.world.add_component(entity, Position::new(10.0, 20.0));
        state.world.add_component(entity, Health::new(100));
        state.select(entity);

        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                panel.show(ui, &mut state);
            });
        });
    }

    #[test]
    fn show_with_multi_selection_no_panic() {
        let mut panel = InspectorPanel::new();
        let mut state = EditorState::new();

        let e1 = state.world.create_entity();
        let e2 = state.world.create_entity();
        state.select_add(e1);
        state.select_add(e2);

        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                panel.show(ui, &mut state);
            });
        });
    }
}
