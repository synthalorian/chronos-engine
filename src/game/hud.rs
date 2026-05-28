#[cfg(feature = "game")]
// ── Enums ──
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HudElement {
    HealthBar,
    ManaBar,
    XpBar,
    Minimap,
    GoldCounter,
    LevelIndicator,
    DayTimeDisplay,
    SquadPanel,
    Tooltip,
    TargetInfo,
    Notification,
}

impl HudElement {
    pub fn label(&self) -> &str {
        match self {
            HudElement::HealthBar => "Health",
            HudElement::ManaBar => "Mana",
            HudElement::XpBar => "Experience",
            HudElement::Minimap => "Minimap",
            HudElement::GoldCounter => "Gold",
            HudElement::LevelIndicator => "Level",
            HudElement::DayTimeDisplay => "Day/Time",
            HudElement::SquadPanel => "Squad",
            HudElement::Tooltip => "Tooltip",
            HudElement::TargetInfo => "Target",
            HudElement::Notification => "Notification",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HudAnchor {
    TopLeft,
    TopCenter,
    TopRight,
    CenterLeft,
    Center,
    CenterRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

// ── Bar Config ──

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HudBarConfig {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub current: f32,
    pub maximum: f32,
    pub bg_color: [f32; 4],
    pub fill_color: [f32; 4],
    pub border_color: [f32; 4],
    pub visible: bool,
}

impl HudBarConfig {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self {
            x,
            y,
            width: w,
            height: h,
            current: 0.0,
            maximum: 100.0,
            bg_color: [0.1, 0.1, 0.1, 0.8],
            fill_color: [0.0, 0.8, 0.0, 1.0],
            border_color: [0.3, 0.3, 0.3, 1.0],
            visible: true,
        }
    }

    pub fn with_colors(mut self, fill: [f32; 4], bg: [f32; 4], border: [f32; 4]) -> Self {
        self.fill_color = fill;
        self.bg_color = bg;
        self.border_color = border;
        self
    }

    pub fn with_values(mut self, current: f32, max: f32) -> Self {
        self.maximum = max;
        self.current = current.clamp(0.0, max);
        self
    }

    pub fn fill_fraction(&self) -> f32 {
        if self.maximum == 0.0 {
            0.0
        } else {
            self.current / self.maximum
        }
    }

    pub fn set_current(&mut self, val: f32) {
        self.current = val.clamp(0.0, self.maximum);
    }

    pub fn is_depleted(&self) -> bool {
        self.current == 0.0
    }
}

// ── Text Config ──

#[derive(Debug, Clone, PartialEq)]
pub struct HudTextConfig {
    pub x: f32,
    pub y: f32,
    pub text: String,
    pub font_size: f32,
    pub color: [f32; 4],
    pub visible: bool,
}

impl HudTextConfig {
    pub fn new(x: f32, y: f32, text: &str) -> Self {
        Self {
            x,
            y,
            text: text.to_string(),
            font_size: 16.0,
            color: [1.0, 1.0, 1.0, 1.0],
            visible: true,
        }
    }

    pub fn with_color(mut self, color: [f32; 4]) -> Self {
        self.color = color;
        self
    }

    pub fn with_font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    pub fn set_text(&mut self, text: &str) {
        self.text = text.to_string();
    }
}

// ── Tooltip ──

#[derive(Debug, Clone, PartialEq)]
pub struct Tooltip {
    pub title: String,
    pub body: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub visible: bool,
}

impl Tooltip {
    pub fn new(title: &str, body: &str) -> Self {
        Self {
            title: title.to_string(),
            body: body.to_string(),
            x: 0.0,
            y: 0.0,
            width: 200.0,
            visible: false,
        }
    }

    pub fn position_at(&mut self, x: f32, y: f32) {
        self.x = x;
        self.y = y;
    }

    pub fn show(&mut self) {
        self.visible = true;
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }
}

// ── Notification ──

#[derive(Debug, Clone, PartialEq)]
pub struct Notification {
    pub text: String,
    pub duration: f32,
    pub elapsed: f32,
    pub color: [f32; 4],
    pub position: HudAnchor,
}

impl Notification {
    pub fn new(text: &str, duration: f32) -> Self {
        Self {
            text: text.to_string(),
            duration,
            elapsed: 0.0,
            color: [1.0, 1.0, 1.0, 1.0],
            position: HudAnchor::TopCenter,
        }
    }

    pub fn with_color(mut self, color: [f32; 4]) -> Self {
        self.color = color;
        self
    }

    /// Decrement duration by delta. Returns true if still alive.
    pub fn tick(&mut self, delta: f32) -> bool {
        self.elapsed += delta;
        self.elapsed < self.duration
    }
}

// ── Squad Panel ──

#[derive(Debug, Clone, PartialEq)]
pub struct SquadMemberDisplay {
    pub name: String,
    pub level: u32,
    pub health_fraction: f32,
    pub mana_fraction: f32,
    pub selected: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SquadPanel {
    pub x: f32,
    pub y: f32,
    pub portrait_size: f32,
    pub members: Vec<SquadMemberDisplay>,
    pub visible: bool,
}

impl SquadPanel {
    pub fn new(x: f32, y: f32) -> Self {
        Self {
            x,
            y,
            portrait_size: 48.0,
            members: Vec::new(),
            visible: true,
        }
    }

    pub fn add_member(&mut self, display: SquadMemberDisplay) {
        self.members.push(display);
    }

    pub fn remove_member(&mut self, index: usize) {
        if index < self.members.len() {
            self.members.remove(index);
        }
    }

    pub fn update_member(&mut self, index: usize, display: SquadMemberDisplay) {
        if let Some(slot) = self.members.get_mut(index) {
            *slot = display;
        }
    }

    pub fn selected_members(&self) -> Vec<&SquadMemberDisplay> {
        self.members.iter().filter(|m| m.selected).collect()
    }
}

// ── HUD Overlay ──

pub struct HudOverlay {
    pub health_bar: HudBarConfig,
    pub mana_bar: HudBarConfig,
    pub xp_bar: HudBarConfig,
    pub gold_text: HudTextConfig,
    pub level_text: HudTextConfig,
    pub time_text: HudTextConfig,
    pub tooltip: Tooltip,
    pub notifications: Vec<Notification>,
    pub squad_panel: SquadPanel,
    pub visible: bool,
}

impl HudOverlay {
    pub fn new(screen_width: f32, screen_height: f32) -> Self {
        let margin = 12.0;
        let bar_w = 220.0;
        let bar_h = 18.0;
        let bar_gap = 24.0;

        let health_bar = HudBarConfig::new(margin, margin, bar_w, bar_h)
            .with_colors(
                [0.8, 0.2, 0.2, 1.0],
                [0.2, 0.05, 0.05, 0.8],
                [0.4, 0.1, 0.1, 1.0],
            )
            .with_values(100.0, 100.0);

        let mana_bar = HudBarConfig::new(margin, margin + bar_gap, bar_w, bar_h)
            .with_colors(
                [0.2, 0.4, 0.9, 1.0],
                [0.05, 0.1, 0.2, 0.8],
                [0.1, 0.15, 0.4, 1.0],
            )
            .with_values(50.0, 100.0);

        let xp_bar = HudBarConfig::new(margin, margin + bar_gap * 2.0, bar_w, bar_h)
            .with_colors(
                [0.9, 0.85, 0.1, 1.0],
                [0.2, 0.18, 0.02, 0.8],
                [0.4, 0.38, 0.05, 1.0],
            )
            .with_values(0.0, 200.0);

        let gold_text = HudTextConfig::new(margin, screen_height - margin - 16.0, "Gold: 0")
            .with_color([1.0, 0.85, 0.0, 1.0]);

        let level_text = HudTextConfig::new(margin + bar_w + 12.0, margin, "Lv 1")
            .with_font_size(14.0)
            .with_color([0.9, 0.9, 0.9, 1.0]);

        let time_text = HudTextConfig::new(screen_width - 160.0, margin, "Day 1 - 00:00")
            .with_color([0.7, 0.8, 1.0, 1.0]);

        let tooltip = Tooltip::new("", "");

        let squad_panel = SquadPanel::new(screen_width - 200.0, screen_height * 0.3);

        Self {
            health_bar,
            mana_bar,
            xp_bar,
            gold_text,
            level_text,
            time_text,
            tooltip,
            notifications: Vec::new(),
            squad_panel,
            visible: true,
        }
    }

    pub fn update(&mut self, delta: f32) {
        for n in &mut self.notifications {
            n.tick(delta);
        }
        self.notifications.retain(|n| n.elapsed < n.duration);
    }

    pub fn show_tooltip(&mut self, title: &str, body: &str, x: f32, y: f32) {
        self.tooltip = Tooltip::new(title, body);
        self.tooltip.position_at(x, y);
        self.tooltip.show();
    }

    pub fn hide_tooltip(&mut self) {
        self.tooltip.hide();
    }

    pub fn notify(&mut self, text: &str, duration: f32) {
        self.notifications.push(Notification::new(text, duration));
    }

    pub fn notify_with_color(&mut self, text: &str, duration: f32, color: [f32; 4]) {
        self.notifications
            .push(Notification::new(text, duration).with_color(color));
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    pub fn set_health(&mut self, current: f32, max: f32) {
        self.health_bar.maximum = max;
        self.health_bar.set_current(current);
    }

    pub fn set_mana(&mut self, current: f32, max: f32) {
        self.mana_bar.maximum = max;
        self.mana_bar.set_current(current);
    }

    pub fn set_xp(&mut self, current: f32, max: f32) {
        self.xp_bar.maximum = max;
        self.xp_bar.set_current(current);
    }

    pub fn set_gold(&mut self, gold: u32) {
        self.gold_text.set_text(&format!("Gold: {}", gold));
    }

    pub fn set_level(&mut self, level: u32) {
        self.level_text.set_text(&format!("Lv {}", level));
    }

    pub fn set_time_display(&mut self, day: u32, hour: f32) {
        let total_minutes = (hour * 60.0) as u32;
        let h = total_minutes / 60;
        let m = total_minutes % 60;
        self.time_text
            .set_text(&format!("Day {} - {:02}:{:02}", day, h, m));
    }

    pub fn notification_count(&self) -> usize {
        self.notifications.len()
    }
}

// ── Tests ──

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hud_bar_fill_fraction() {
        let bar = HudBarConfig::new(0.0, 0.0, 100.0, 10.0).with_values(75.0, 100.0);
        assert!((bar.fill_fraction() - 0.75).abs() < f32::EPSILON);
    }

    #[test]
    fn hud_bar_depleted() {
        let bar = HudBarConfig::new(0.0, 0.0, 100.0, 10.0).with_values(0.0, 100.0);
        assert!(bar.is_depleted());
    }

    #[test]
    fn hud_bar_set_current_clamps() {
        let mut bar = HudBarConfig::new(0.0, 0.0, 100.0, 10.0).with_values(50.0, 100.0);
        bar.set_current(150.0);
        assert_eq!(bar.current, 100.0);
        bar.set_current(-10.0);
        assert_eq!(bar.current, 0.0);
    }

    #[test]
    fn hud_text_creation() {
        let txt = HudTextConfig::new(10.0, 20.0, "hello");
        assert_eq!(txt.x, 10.0);
        assert_eq!(txt.y, 20.0);
        assert_eq!(txt.text, "hello");
        assert_eq!(txt.font_size, 16.0);
        assert!(txt.visible);
    }

    #[test]
    fn tooltip_show_hide() {
        let mut tip = Tooltip::new("Title", "Body text");
        assert!(!tip.visible);
        tip.show();
        assert!(tip.visible);
        tip.hide();
        assert!(!tip.visible);
    }

    #[test]
    fn notification_tick_expires() {
        let mut n = Notification::new("msg", 2.0);
        assert!(n.tick(1.0)); // elapsed=1.0, still alive
        assert!(!n.tick(1.0)); // elapsed=2.0 >= duration, expired
    }

    #[test]
    fn notification_tick_alive() {
        let mut n = Notification::new("msg", 5.0);
        assert!(n.tick(1.0));
        assert!(n.tick(2.0));
        assert!(n.elapsed < n.duration);
    }

    #[test]
    fn squad_panel_members() {
        let mut panel = SquadPanel::new(0.0, 0.0);
        let m1 = SquadMemberDisplay {
            name: "Alice".into(),
            level: 5,
            health_fraction: 1.0,
            mana_fraction: 0.8,
            selected: true,
        };
        let m2 = SquadMemberDisplay {
            name: "Bob".into(),
            level: 3,
            health_fraction: 0.5,
            mana_fraction: 0.2,
            selected: false,
        };
        panel.add_member(m1.clone());
        panel.add_member(m2.clone());
        assert_eq!(panel.members.len(), 2);

        panel.remove_member(0);
        assert_eq!(panel.members.len(), 1);
        assert_eq!(panel.members[0].name, "Bob");

        panel.update_member(
            0,
            SquadMemberDisplay {
                name: "Bob".into(),
                level: 4,
                health_fraction: 1.0,
                mana_fraction: 1.0,
                selected: true,
            },
        );
        assert_eq!(panel.members[0].level, 4);
    }

    #[test]
    fn squad_panel_selected() {
        let mut panel = SquadPanel::new(0.0, 0.0);
        panel.add_member(SquadMemberDisplay {
            name: "A".into(),
            level: 1,
            health_fraction: 1.0,
            mana_fraction: 1.0,
            selected: true,
        });
        panel.add_member(SquadMemberDisplay {
            name: "B".into(),
            level: 1,
            health_fraction: 1.0,
            mana_fraction: 1.0,
            selected: false,
        });
        panel.add_member(SquadMemberDisplay {
            name: "C".into(),
            level: 2,
            health_fraction: 1.0,
            mana_fraction: 1.0,
            selected: true,
        });
        let sel = panel.selected_members();
        assert_eq!(sel.len(), 2);
        assert_eq!(sel[0].name, "A");
        assert_eq!(sel[1].name, "C");
    }

    #[test]
    fn hud_overlay_creation() {
        let hud = HudOverlay::new(1920.0, 1080.0);
        assert!(hud.visible);
        assert!(hud.health_bar.visible);
        assert!(hud.notifications.is_empty());
        assert!(!hud.tooltip.visible);
        assert!(hud.squad_panel.members.is_empty());
    }

    #[test]
    fn hud_overlay_set_health() {
        let mut hud = HudOverlay::new(1920.0, 1080.0);
        hud.set_health(60.0, 120.0);
        assert_eq!(hud.health_bar.maximum, 120.0);
        assert_eq!(hud.health_bar.current, 60.0);
    }

    #[test]
    fn hud_overlay_notify() {
        let mut hud = HudOverlay::new(1920.0, 1080.0);
        hud.notify("Hello", 3.0);
        assert_eq!(hud.notification_count(), 1);
        hud.update(5.0);
        assert_eq!(hud.notification_count(), 0);
    }

    #[test]
    fn hud_overlay_toggle() {
        let mut hud = HudOverlay::new(1920.0, 1080.0);
        assert!(hud.visible);
        hud.toggle();
        assert!(!hud.visible);
        hud.toggle();
        assert!(hud.visible);
    }

    #[test]
    fn hud_bar_fill_fraction_zero_max() {
        let bar = HudBarConfig::new(0.0, 0.0, 100.0, 10.0).with_values(0.0, 0.0);
        assert_eq!(bar.fill_fraction(), 0.0);
    }

    #[test]
    fn hud_element_labels() {
        assert_eq!(HudElement::HealthBar.label(), "Health");
        assert_eq!(HudElement::ManaBar.label(), "Mana");
        assert_eq!(HudElement::Notification.label(), "Notification");
    }

    #[test]
    fn tooltip_position() {
        let mut tip = Tooltip::new("T", "B");
        tip.position_at(42.0, 99.0);
        assert_eq!(tip.x, 42.0);
        assert_eq!(tip.y, 99.0);
    }

    #[test]
    fn hud_overlay_set_time_display() {
        let mut hud = HudOverlay::new(1920.0, 1080.0);
        hud.set_time_display(3, 14.5);
        assert_eq!(hud.time_text.text, "Day 3 - 14:30");
    }

    #[test]
    fn hud_overlay_set_gold_and_level() {
        let mut hud = HudOverlay::new(1920.0, 1080.0);
        hud.set_gold(9999);
        assert_eq!(hud.gold_text.text, "Gold: 9999");
        hud.set_level(42);
        assert_eq!(hud.level_text.text, "Lv 42");
    }

    #[test]
    fn notification_with_color() {
        let n = Notification::new("msg", 1.0).with_color([1.0, 0.0, 0.0, 1.0]);
        assert_eq!(n.color, [1.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn hud_text_set_text() {
        let mut txt = HudTextConfig::new(0.0, 0.0, "old");
        txt.set_text("new");
        assert_eq!(txt.text, "new");
    }

    #[test]
    fn hud_overlay_notify_with_color() {
        let mut hud = HudOverlay::new(1920.0, 1080.0);
        hud.notify_with_color("Alert!", 2.0, [1.0, 0.0, 0.0, 1.0]);
        assert_eq!(hud.notification_count(), 1);
        assert_eq!(hud.notifications[0].color, [1.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn squad_panel_remove_out_of_bounds() {
        let mut panel = SquadPanel::new(0.0, 0.0);
        panel.add_member(SquadMemberDisplay {
            name: "A".into(),
            level: 1,
            health_fraction: 1.0,
            mana_fraction: 1.0,
            selected: false,
        });
        panel.remove_member(5);
        assert_eq!(panel.members.len(), 1);
    }

    #[test]
    fn squad_panel_update_out_of_bounds() {
        let mut panel = SquadPanel::new(0.0, 0.0);
        panel.update_member(
            0,
            SquadMemberDisplay {
                name: "X".into(),
                level: 1,
                health_fraction: 1.0,
                mana_fraction: 1.0,
                selected: false,
            },
        );
        assert!(panel.members.is_empty());
    }
}
