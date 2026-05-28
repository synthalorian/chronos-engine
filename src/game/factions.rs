#[cfg(feature = "game")]
use std::collections::HashMap;

// ── FactionId ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FactionId {
    ChronosCompany,
    CityWatch,
    BlackMarket,
    MerchantGuild,
    Rebels,
    CorporateSec,
    Neutral,
}

impl FactionId {
    pub fn name(&self) -> &str {
        match self {
            FactionId::ChronosCompany => "Chronos Company",
            FactionId::CityWatch => "City Watch",
            FactionId::BlackMarket => "Black Market",
            FactionId::MerchantGuild => "Merchant Guild",
            FactionId::Rebels => "Rebels",
            FactionId::CorporateSec => "Corporate Security",
            FactionId::Neutral => "Neutral",
        }
    }

    pub fn all() -> [FactionId; 7] {
        [
            FactionId::ChronosCompany,
            FactionId::CityWatch,
            FactionId::BlackMarket,
            FactionId::MerchantGuild,
            FactionId::Rebels,
            FactionId::CorporateSec,
            FactionId::Neutral,
        ]
    }
}

// ── ReputationLevel ──

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReputationLevel {
    Hated,
    Hostile,
    Unfriendly,
    Neutral,
    Friendly,
    Honored,
    Hero,
}

impl ReputationLevel {
    pub fn from_value(value: i32) -> Self {
        match value {
            v if v <= -75 => ReputationLevel::Hated,
            v if v <= -50 => ReputationLevel::Hostile,
            v if v <= -25 => ReputationLevel::Unfriendly,
            v if v < 25 => ReputationLevel::Neutral,
            v if v < 50 => ReputationLevel::Friendly,
            v if v < 75 => ReputationLevel::Honored,
            _ => ReputationLevel::Hero,
        }
    }

    pub fn min_value(&self) -> i32 {
        match self {
            ReputationLevel::Hated => -100,
            ReputationLevel::Hostile => -74,
            ReputationLevel::Unfriendly => -49,
            ReputationLevel::Neutral => -24,
            ReputationLevel::Friendly => 25,
            ReputationLevel::Honored => 50,
            ReputationLevel::Hero => 75,
        }
    }

    pub fn max_value(&self) -> i32 {
        match self {
            ReputationLevel::Hated => -75,
            ReputationLevel::Hostile => -50,
            ReputationLevel::Unfriendly => -25,
            ReputationLevel::Neutral => 24,
            ReputationLevel::Friendly => 49,
            ReputationLevel::Honored => 74,
            ReputationLevel::Hero => 100,
        }
    }
}

// ── ReputationEntry ──

#[derive(Debug, Clone, PartialEq)]
pub struct ReputationEntry {
    pub faction: FactionId,
    pub value: i32,
    pub completed_jobs: u32,
}

impl ReputationEntry {
    pub fn new(faction: FactionId) -> Self {
        ReputationEntry {
            faction,
            value: 0,
            completed_jobs: 0,
        }
    }

    pub fn with_value(mut self, value: i32) -> Self {
        self.value = value.clamp(-100, 100);
        self
    }

    pub fn add(&mut self, delta: i32) -> i32 {
        self.value = (self.value + delta).clamp(-100, 100);
        self.value
    }

    pub fn level(&self) -> ReputationLevel {
        ReputationLevel::from_value(self.value)
    }
}

// ── ReputationTracker ──

#[derive(Debug, Clone, PartialEq)]
pub struct ReputationTracker {
    pub entries: HashMap<FactionId, ReputationEntry>,
}

impl ReputationTracker {
    pub fn new() -> Self {
        ReputationTracker {
            entries: HashMap::new(),
        }
    }

    pub fn get_or_create(&mut self, faction: FactionId) -> &mut ReputationEntry {
        self.entries
            .entry(faction)
            .or_insert_with(|| ReputationEntry::new(faction))
    }

    pub fn get_value(&self, faction: FactionId) -> i32 {
        self.entries.get(&faction).map(|e| e.value).unwrap_or(0)
    }

    pub fn get_level(&self, faction: FactionId) -> ReputationLevel {
        ReputationLevel::from_value(self.get_value(faction))
    }

    pub fn modify_reputation(&mut self, faction: FactionId, delta: i32) -> i32 {
        self.get_or_create(faction).add(delta)
    }

    pub fn has_reputation(&self, faction: FactionId, min_level: ReputationLevel) -> bool {
        let current = self.get_value(faction);
        current >= min_level.min_value()
    }

    pub fn record_job(&mut self, faction: FactionId) {
        let entry = self.get_or_create(faction);
        entry.completed_jobs += 1;
    }
}

// ── PricingModifier ──

#[derive(Debug, Clone, PartialEq)]
pub struct PricingModifier;

impl PricingModifier {
    pub fn buy_multiplier(level: ReputationLevel) -> f32 {
        match level {
            ReputationLevel::Hated => 2.0,
            ReputationLevel::Hostile => 1.75,
            ReputationLevel::Unfriendly => 1.5,
            ReputationLevel::Neutral => 1.0,
            ReputationLevel::Friendly => 0.85,
            ReputationLevel::Honored => 0.7,
            ReputationLevel::Hero => 0.5,
        }
    }

    pub fn sell_multiplier(level: ReputationLevel) -> f32 {
        match level {
            ReputationLevel::Hated => 0.5,
            ReputationLevel::Hostile => 0.6,
            ReputationLevel::Unfriendly => 0.75,
            ReputationLevel::Neutral => 1.0,
            ReputationLevel::Friendly => 1.25,
            ReputationLevel::Honored => 1.5,
            ReputationLevel::Hero => 2.0,
        }
    }

    pub fn apply_pricing(base_price: u32, level: ReputationLevel, is_buying: bool) -> u32 {
        let multiplier = if is_buying {
            Self::buy_multiplier(level)
        } else {
            Self::sell_multiplier(level)
        };
        (base_price as f32 * multiplier) as u32
    }
}

// ── AccessGate ──

#[derive(Debug, Clone, PartialEq)]
pub struct AccessGate {
    pub faction: FactionId,
    pub min_level: ReputationLevel,
    pub description: String,
}

impl AccessGate {
    pub fn new(
        faction: FactionId,
        min_level: ReputationLevel,
        description: impl Into<String>,
    ) -> Self {
        AccessGate {
            faction,
            min_level,
            description: description.into(),
        }
    }

    pub fn can_access(&self, tracker: &ReputationTracker) -> bool {
        tracker.has_reputation(self.faction, self.min_level)
    }

    pub fn black_market_entry() -> Self {
        AccessGate::new(
            FactionId::BlackMarket,
            ReputationLevel::Friendly,
            "Black Market back entrance",
        )
    }

    pub fn guild_armory() -> Self {
        AccessGate::new(
            FactionId::MerchantGuild,
            ReputationLevel::Honored,
            "Merchant Guild armory",
        )
    }

    pub fn rebel_hideout() -> Self {
        AccessGate::new(
            FactionId::Rebels,
            ReputationLevel::Friendly,
            "Rebel underground hideout",
        )
    }
}

// ── FactionSystem ──

#[derive(Debug, Clone, PartialEq)]
pub struct FactionSystem;

impl FactionSystem {
    pub fn new() -> Self {
        FactionSystem
    }

    pub fn apply_job_reward(
        tracker: &mut ReputationTracker,
        faction: FactionId,
        xp_multiplier: f32,
    ) {
        let gain = (5.0 * xp_multiplier) as i32;
        tracker.modify_reputation(faction, gain);
        tracker.record_job(faction);
    }

    pub fn apply_kill_penalty(tracker: &mut ReputationTracker, victim_faction: FactionId) {
        tracker.modify_reputation(victim_faction, -15);
        for enemy in Self::get_enemy_factions(victim_faction) {
            tracker.modify_reputation(enemy, 3);
        }
    }

    pub fn get_enemy_factions(faction: FactionId) -> Vec<FactionId> {
        match faction {
            FactionId::CityWatch => vec![FactionId::BlackMarket],
            FactionId::BlackMarket => vec![FactionId::CityWatch],
            FactionId::Rebels => vec![FactionId::CorporateSec],
            FactionId::CorporateSec => vec![FactionId::Rebels],
            _ => vec![],
        }
    }
}

// ── Tests ──

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reputation_entry_clamping() {
        let entry = ReputationEntry::new(FactionId::CityWatch).with_value(150);
        assert_eq!(entry.value, 100);

        let entry = ReputationEntry::new(FactionId::CityWatch).with_value(-200);
        assert_eq!(entry.value, -100);

        let mut entry = ReputationEntry::new(FactionId::CityWatch).with_value(95);
        let new_val = entry.add(20);
        assert_eq!(new_val, 100);

        let mut entry = ReputationEntry::new(FactionId::CityWatch).with_value(-90);
        let new_val = entry.add(-30);
        assert_eq!(new_val, -100);
    }

    #[test]
    fn reputation_levels_correct() {
        assert_eq!(ReputationLevel::from_value(-100), ReputationLevel::Hated);
        assert_eq!(ReputationLevel::from_value(-75), ReputationLevel::Hated);
        assert_eq!(ReputationLevel::from_value(-74), ReputationLevel::Hostile);
        assert_eq!(ReputationLevel::from_value(-50), ReputationLevel::Hostile);
        assert_eq!(
            ReputationLevel::from_value(-49),
            ReputationLevel::Unfriendly
        );
        assert_eq!(
            ReputationLevel::from_value(-25),
            ReputationLevel::Unfriendly
        );
        assert_eq!(ReputationLevel::from_value(-24), ReputationLevel::Neutral);
        assert_eq!(ReputationLevel::from_value(0), ReputationLevel::Neutral);
        assert_eq!(ReputationLevel::from_value(24), ReputationLevel::Neutral);
        assert_eq!(ReputationLevel::from_value(25), ReputationLevel::Friendly);
        assert_eq!(ReputationLevel::from_value(49), ReputationLevel::Friendly);
        assert_eq!(ReputationLevel::from_value(50), ReputationLevel::Honored);
        assert_eq!(ReputationLevel::from_value(74), ReputationLevel::Honored);
        assert_eq!(ReputationLevel::from_value(75), ReputationLevel::Hero);
        assert_eq!(ReputationLevel::from_value(100), ReputationLevel::Hero);
    }

    #[test]
    fn tracker_modify_reputation() {
        let mut tracker = ReputationTracker::new();

        let val = tracker.modify_reputation(FactionId::BlackMarket, 30);
        assert_eq!(val, 30);

        let val = tracker.modify_reputation(FactionId::BlackMarket, 25);
        assert_eq!(val, 55);

        let val = tracker.modify_reputation(FactionId::BlackMarket, -60);
        assert_eq!(val, -5);

        let val = tracker.modify_reputation(FactionId::BlackMarket, -200);
        assert_eq!(val, -100);
    }

    #[test]
    fn tracker_has_reputation() {
        let mut tracker = ReputationTracker::new();
        tracker.modify_reputation(FactionId::MerchantGuild, 55);

        assert!(tracker.has_reputation(FactionId::MerchantGuild, ReputationLevel::Friendly));
        assert!(tracker.has_reputation(FactionId::MerchantGuild, ReputationLevel::Honored));
        assert!(!tracker.has_reputation(FactionId::MerchantGuild, ReputationLevel::Hero));

        // Missing faction defaults to 0
        assert!(tracker.has_reputation(FactionId::Rebels, ReputationLevel::Neutral));
        assert!(!tracker.has_reputation(FactionId::Rebels, ReputationLevel::Friendly));
    }

    #[test]
    fn pricing_buy_sell() {
        assert_eq!(PricingModifier::buy_multiplier(ReputationLevel::Hated), 2.0);
        assert_eq!(
            PricingModifier::buy_multiplier(ReputationLevel::Hostile),
            1.75
        );
        assert_eq!(
            PricingModifier::buy_multiplier(ReputationLevel::Unfriendly),
            1.5
        );
        assert_eq!(
            PricingModifier::buy_multiplier(ReputationLevel::Neutral),
            1.0
        );
        assert_eq!(
            PricingModifier::buy_multiplier(ReputationLevel::Friendly),
            0.85
        );
        assert_eq!(
            PricingModifier::buy_multiplier(ReputationLevel::Honored),
            0.7
        );
        assert_eq!(PricingModifier::buy_multiplier(ReputationLevel::Hero), 0.5);

        assert_eq!(
            PricingModifier::sell_multiplier(ReputationLevel::Hated),
            0.5
        );
        assert_eq!(
            PricingModifier::sell_multiplier(ReputationLevel::Hostile),
            0.6
        );
        assert_eq!(
            PricingModifier::sell_multiplier(ReputationLevel::Unfriendly),
            0.75
        );
        assert_eq!(
            PricingModifier::sell_multiplier(ReputationLevel::Neutral),
            1.0
        );
        assert_eq!(
            PricingModifier::sell_multiplier(ReputationLevel::Friendly),
            1.25
        );
        assert_eq!(
            PricingModifier::sell_multiplier(ReputationLevel::Honored),
            1.5
        );
        assert_eq!(PricingModifier::sell_multiplier(ReputationLevel::Hero), 2.0);
    }

    #[test]
    fn pricing_apply() {
        assert_eq!(
            PricingModifier::apply_pricing(100, ReputationLevel::Neutral, true),
            100
        );
        assert_eq!(
            PricingModifier::apply_pricing(100, ReputationLevel::Hated, true),
            200
        );
        assert_eq!(
            PricingModifier::apply_pricing(100, ReputationLevel::Hero, true),
            50
        );
        assert_eq!(
            PricingModifier::apply_pricing(100, ReputationLevel::Neutral, false),
            100
        );
        assert_eq!(
            PricingModifier::apply_pricing(100, ReputationLevel::Hero, false),
            200
        );
        assert_eq!(
            PricingModifier::apply_pricing(100, ReputationLevel::Hated, false),
            50
        );
    }

    #[test]
    fn access_gate_grants() {
        let mut tracker = ReputationTracker::new();
        tracker.modify_reputation(FactionId::BlackMarket, 30);

        let gate = AccessGate::black_market_entry();
        assert!(gate.can_access(&tracker));
    }

    #[test]
    fn access_gate_denies() {
        let tracker = ReputationTracker::new();
        let gate = AccessGate::guild_armory();
        assert!(!gate.can_access(&tracker));

        let mut tracker = ReputationTracker::new();
        tracker.modify_reputation(FactionId::MerchantGuild, 30);
        assert!(!gate.can_access(&tracker));
    }

    #[test]
    fn faction_system_job_reward() {
        let mut tracker = ReputationTracker::new();

        FactionSystem::apply_job_reward(&mut tracker, FactionId::CityWatch, 1.0);
        assert_eq!(tracker.get_value(FactionId::CityWatch), 5);
        assert_eq!(
            tracker
                .entries
                .get(&FactionId::CityWatch)
                .unwrap()
                .completed_jobs,
            1
        );

        FactionSystem::apply_job_reward(&mut tracker, FactionId::CityWatch, 2.0);
        assert_eq!(tracker.get_value(FactionId::CityWatch), 15);
        assert_eq!(
            tracker
                .entries
                .get(&FactionId::CityWatch)
                .unwrap()
                .completed_jobs,
            2
        );
    }

    #[test]
    fn faction_system_kill_penalty() {
        let mut tracker = ReputationTracker::new();
        tracker.modify_reputation(FactionId::CityWatch, 50);

        FactionSystem::apply_kill_penalty(&mut tracker, FactionId::CityWatch);

        assert_eq!(tracker.get_value(FactionId::CityWatch), 35);
        assert_eq!(tracker.get_value(FactionId::BlackMarket), 3);
    }

    #[test]
    fn enemy_factions_correct() {
        assert_eq!(
            FactionSystem::get_enemy_factions(FactionId::CityWatch),
            vec![FactionId::BlackMarket]
        );
        assert_eq!(
            FactionSystem::get_enemy_factions(FactionId::BlackMarket),
            vec![FactionId::CityWatch]
        );
        assert_eq!(
            FactionSystem::get_enemy_factions(FactionId::Rebels),
            vec![FactionId::CorporateSec]
        );
        assert_eq!(
            FactionSystem::get_enemy_factions(FactionId::CorporateSec),
            vec![FactionId::Rebels]
        );
        assert!(FactionSystem::get_enemy_factions(FactionId::ChronosCompany).is_empty());
        assert!(FactionSystem::get_enemy_factions(FactionId::Neutral).is_empty());
        assert!(FactionSystem::get_enemy_factions(FactionId::MerchantGuild).is_empty());
    }
}
