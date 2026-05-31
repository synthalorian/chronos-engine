#[cfg(feature = "game")]
use super::components::{LootDrop, MercenaryStats};

// ──────────────────────────────────────────────
// Enums
// ──────────────────────────────────────────────

/// The category of work a mercenary contract covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobType {
    Bounty,
    Escort,
    Fetch,
    ClearArea,
    Defend,
    Assassinate,
}

impl JobType {
    /// Total number of variants — used for deterministic cycling.
    const COUNT: usize = 6;

    /// Human-readable label used in job descriptions.
    pub fn label(&self) -> &'static str {
        match self {
            JobType::Bounty => "Bounty",
            JobType::Escort => "Escort",
            JobType::Fetch => "Fetch",
            JobType::ClearArea => "Clear Area",
            JobType::Defend => "Defend",
            JobType::Assassinate => "Assassinate",
        }
    }

    /// Deterministic selection by index modulo.
    pub fn from_index(i: u32) -> Self {
        match i % Self::COUNT as u32 {
            0 => JobType::Bounty,
            1 => JobType::Escort,
            2 => JobType::Fetch,
            3 => JobType::ClearArea,
            4 => JobType::Defend,
            _ => JobType::Assassinate,
        }
    }
}

/// How tough the contract is — scales rewards, enemies, and time limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobDifficulty {
    Trivial,
    Easy,
    Medium,
    Hard,
    Legendary,
}

impl JobDifficulty {
    /// XP reward multiplier per difficulty tier.
    pub fn xp_multiplier(&self) -> f32 {
        match self {
            JobDifficulty::Trivial => 0.5,
            JobDifficulty::Easy => 0.8,
            JobDifficulty::Medium => 1.0,
            JobDifficulty::Hard => 1.5,
            JobDifficulty::Legendary => 2.5,
        }
    }

    /// Gold reward multiplier per difficulty tier.
    pub fn gold_multiplier(&self) -> f32 {
        match self {
            JobDifficulty::Trivial => 0.3,
            JobDifficulty::Easy => 0.6,
            JobDifficulty::Medium => 1.0,
            JobDifficulty::Hard => 2.0,
            JobDifficulty::Legendary => 4.0,
        }
    }

    /// (min, max) party level appropriate for this difficulty.
    pub fn level_range(&self) -> (u32, u32) {
        match self {
            JobDifficulty::Trivial => (1, 3),
            JobDifficulty::Easy => (3, 6),
            JobDifficulty::Medium => (6, 12),
            JobDifficulty::Hard => (12, 20),
            JobDifficulty::Legendary => (20, 30),
        }
    }

    /// Automatically select a difficulty appropriate for the given level.
    pub fn from_level(level: u32) -> JobDifficulty {
        if level <= 3 {
            JobDifficulty::Trivial
        } else if level <= 6 {
            JobDifficulty::Easy
        } else if level <= 12 {
            JobDifficulty::Medium
        } else if level <= 20 {
            JobDifficulty::Hard
        } else {
            JobDifficulty::Legendary
        }
    }

    /// Number of enemies spawned at this difficulty.
    fn enemy_count(&self) -> u32 {
        match self {
            JobDifficulty::Trivial => 1,
            JobDifficulty::Easy => 2,
            JobDifficulty::Medium => 4,
            JobDifficulty::Hard => 6,
            JobDifficulty::Legendary => 10,
        }
    }

    /// Time limit in seconds, or `None` for no restriction.
    fn time_limit(&self) -> Option<f32> {
        match self {
            JobDifficulty::Trivial => None,
            JobDifficulty::Easy => Some(300.0),
            JobDifficulty::Medium => Some(180.0),
            JobDifficulty::Hard => Some(120.0),
            JobDifficulty::Legendary => Some(60.0),
        }
    }

    /// Human-readable label.
    fn label(&self) -> &'static str {
        match self {
            JobDifficulty::Trivial => "Trivial",
            JobDifficulty::Easy => "Easy",
            JobDifficulty::Medium => "Medium",
            JobDifficulty::Hard => "Hard",
            JobDifficulty::Legendary => "Legendary",
        }
    }
}

/// Where a contract currently sits in its lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobStatus {
    Available,
    InProgress { accepted_by: u32 },
    Completed,
    Failed,
    Expired,
}

// ──────────────────────────────────────────────
// JobContract
// ──────────────────────────────────────────────

/// A single job contract on the board.
#[derive(Debug, Clone)]
pub struct JobContract {
    pub id: u32,
    pub title: String,
    pub description: String,
    pub job_type: JobType,
    pub difficulty: JobDifficulty,
    pub status: JobStatus,
    pub target_location: [f32; 3],
    pub reward_gold: u32,
    pub reward_xp: u32,
    pub reward_items: Vec<String>,
    pub time_limit: Option<f32>,
    pub required_level: u32,
    pub enemy_count: u32,
    pub client_name: String,
}

impl JobContract {
    /// Create a minimal contract — use builder methods to flesh it out.
    pub fn new(id: u32) -> Self {
        JobContract {
            id,
            title: String::new(),
            description: String::new(),
            job_type: JobType::Bounty,
            difficulty: JobDifficulty::Trivial,
            status: JobStatus::Available,
            target_location: [0.0, 0.0, 0.0],
            reward_gold: 0,
            reward_xp: 0,
            reward_items: Vec::new(),
            time_limit: None,
            required_level: 1,
            enemy_count: 1,
            client_name: String::new(),
        }
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    pub fn with_job_type(mut self, job_type: JobType) -> Self {
        self.job_type = job_type;
        self
    }

    pub fn with_difficulty(mut self, difficulty: JobDifficulty) -> Self {
        self.difficulty = difficulty;
        self
    }

    pub fn with_target_location(mut self, loc: [f32; 3]) -> Self {
        self.target_location = loc;
        self
    }

    pub fn with_reward_gold(mut self, gold: u32) -> Self {
        self.reward_gold = gold;
        self
    }

    pub fn with_reward_xp(mut self, xp: u32) -> Self {
        self.reward_xp = xp;
        self
    }

    pub fn with_reward_items(mut self, items: Vec<String>) -> Self {
        self.reward_items = items;
        self
    }

    pub fn with_time_limit(mut self, limit: Option<f32>) -> Self {
        self.time_limit = limit;
        self
    }

    pub fn with_required_level(mut self, level: u32) -> Self {
        self.required_level = level;
        self
    }

    pub fn with_enemy_count(mut self, count: u32) -> Self {
        self.enemy_count = count;
        self
    }

    pub fn with_client_name(mut self, name: impl Into<String>) -> Self {
        self.client_name = name.into();
        self
    }

    /// Whether the contract can be picked up.
    pub fn is_available(&self) -> bool {
        matches!(self.status, JobStatus::Available)
    }

    /// Whether the contract has already been fulfilled.
    pub fn is_complete(&self) -> bool {
        matches!(self.status, JobStatus::Completed)
    }

    /// Check whether the given mercenary meets the level requirement.
    pub fn can_accept(&self, stats: &MercenaryStats) -> bool {
        stats.level >= self.required_level && self.is_available()
    }

    /// Mark the contract as accepted by the given entity index.
    pub fn accept(&mut self, accepted_by: u32) {
        if self.is_available() {
            self.status = JobStatus::InProgress { accepted_by };
        }
    }

    /// Mark the contract as successfully completed.
    pub fn complete(&mut self) {
        self.status = JobStatus::Completed;
    }

    /// Mark the contract as failed.
    pub fn fail(&mut self) {
        self.status = JobStatus::Failed;
    }
}

// ──────────────────────────────────────────────
// AcceptResult
// ──────────────────────────────────────────────

/// Outcome of trying to accept a job from the board.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcceptResult {
    Accepted,
    NotFound,
    AlreadyActive,
    LevelTooLow,
    BoardFull,
}

// ──────────────────────────────────────────────
// JobGenerator
// ──────────────────────────────────────────────

/// Procedural job factory — deterministically generates varied contracts.
pub struct JobGenerator {
    next_id: u32,
}

const CLIENT_NAMES: &[&str] = &[
    "Aldric", "Mira", "Theron", "Selene", "Gareth", "Lyra", "Dorian", "Freya",
];

const BASE_GOLD: u32 = 50;
const BASE_XP: u32 = 100;

impl Default for JobGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl JobGenerator {
    pub fn new() -> Self {
        JobGenerator { next_id: 1 }
    }

    /// Generate a single job for the given difficulty tier.
    pub fn generate_job(&mut self, difficulty: JobDifficulty) -> JobContract {
        let id = self.next_id;
        self.next_id += 1;

        let job_type = JobType::from_index(id);
        let enemy_count = difficulty.enemy_count();
        let time_limit = difficulty.time_limit();
        let (min_level, _) = difficulty.level_range();

        let title = format!("{} Contract #{}", job_type.label(), id);
        let description = format!(
            "A {} {} job. {} enemies await.",
            difficulty.label(),
            job_type.label(),
            enemy_count,
        );

        let reward_gold = (BASE_GOLD as f32 * difficulty.gold_multiplier()) as u32;
        let reward_xp = (BASE_XP as f32 * difficulty.xp_multiplier()) as u32;

        // Deterministic pseudo-random location via id-based hash
        let loc_x = Self::det_offset(id, 0) * 50.0;
        let loc_y = 0.0;
        let loc_z = Self::det_offset(id, 1) * 50.0;

        let client = CLIENT_NAMES[(id as usize) % CLIENT_NAMES.len()].to_string();

        // Reward items for harder difficulties
        let reward_items = Self::reward_items_for(id, difficulty);

        JobContract::new(id)
            .with_title(title)
            .with_description(description)
            .with_job_type(job_type)
            .with_difficulty(difficulty)
            .with_target_location([loc_x, loc_y, loc_z])
            .with_reward_gold(reward_gold)
            .with_reward_xp(reward_xp)
            .with_reward_items(reward_items)
            .with_time_limit(time_limit)
            .with_required_level(min_level)
            .with_enemy_count(enemy_count)
            .with_client_name(client)
    }

    /// Generate multiple jobs at once.
    pub fn generate_jobs(&mut self, count: usize, difficulty: JobDifficulty) -> Vec<JobContract> {
        (0..count).map(|_| self.generate_job(difficulty)).collect()
    }

    /// Generate a job whose difficulty matches the given player level.
    pub fn generate_for_level(&mut self, player_level: u32) -> JobContract {
        let difficulty = JobDifficulty::from_level(player_level);
        self.generate_job(difficulty)
    }

    /// Deterministic offset in -1..1 range using id and seed.
    fn det_offset(id: u32, seed: u32) -> f32 {
        let v = (id.wrapping_mul(17).wrapping_add(seed).wrapping_mul(31)) as f32;
        (v.sin() * 43_758.547).fract() * 2.0 - 1.0
    }

    /// Produce reward items based on difficulty.
    fn reward_items_for(id: u32, difficulty: JobDifficulty) -> Vec<String> {
        let mut items = Vec::new();
        match difficulty {
            JobDifficulty::Trivial => {}
            JobDifficulty::Easy => {
                items.push("Health Potion".to_string());
            }
            JobDifficulty::Medium => {
                items.push("Health Potion".to_string());
                if id.is_multiple_of(2) {
                    items.push("Mana Potion".to_string());
                }
            }
            JobDifficulty::Hard => {
                items.push("Health Potion".to_string());
                items.push("Mana Potion".to_string());
                items.push("Iron Ore".to_string());
            }
            JobDifficulty::Legendary => {
                items.push("Health Potion".to_string());
                items.push("Mana Potion".to_string());
                items.push("Dragon Scale".to_string());
                items.push("Enchanted Gem".to_string());
            }
        }
        items
    }
}

// ──────────────────────────────────────────────
// JobBoard
// ──────────────────────────────────────────────

/// Manages the lifecycle of available, active, and completed contracts.
#[derive(Debug, Clone)]
pub struct JobBoard {
    pub available: Vec<JobContract>,
    pub active: Vec<JobContract>,
    pub completed: Vec<JobContract>,
    pub max_active: usize,
}

impl JobBoard {
    pub fn new(max_active: usize) -> Self {
        JobBoard {
            available: Vec::new(),
            active: Vec::new(),
            completed: Vec::new(),
            max_active,
        }
    }

    /// Post a new contract to the available pool.
    pub fn post_job(&mut self, job: JobContract) {
        self.available.push(job);
    }

    /// Accept a contract by ID. Moves it from available to active if eligible.
    pub fn accept_job(&mut self, job_id: u32, party_level: u32) -> AcceptResult {
        // Check if already active
        if self.active.iter().any(|j| j.id == job_id) {
            return AcceptResult::AlreadyActive;
        }

        // Check if board is full
        if self.active.len() >= self.max_active {
            return AcceptResult::BoardFull;
        }

        // Find in available
        let idx = match self.available.iter().position(|j| j.id == job_id) {
            Some(i) => i,
            None => return AcceptResult::NotFound,
        };

        // Level gate
        if party_level < self.available[idx].required_level {
            return AcceptResult::LevelTooLow;
        }

        let mut job = self.available.remove(idx);
        job.accept(party_level);
        self.active.push(job);
        AcceptResult::Accepted
    }

    /// Complete an active contract. Returns the reward as a [`LootDrop`].
    pub fn complete_job(&mut self, job_id: u32) -> Option<LootDrop> {
        let idx = self.active.iter().position(|j| j.id == job_id)?;
        let mut job = self.active.remove(idx);
        job.complete();
        let drop = LootDrop::new(job.reward_gold).with_items(job.reward_items.clone());
        self.completed.push(job);
        Some(drop)
    }

    /// Fail an active contract — removes it without reward.
    pub fn fail_job(&mut self, job_id: u32) {
        if let Some(idx) = self.active.iter().position(|j| j.id == job_id) {
            let mut job = self.active.remove(idx);
            job.fail();
        }
    }

    /// Remove expired available jobs, then generate fresh ones for the given level.
    pub fn refresh(&mut self, generator: &mut JobGenerator, count: usize, level: u32) {
        self.available
            .retain(|j| !matches!(j.status, JobStatus::Expired));
        let new_jobs = generator.generate_jobs(count, JobDifficulty::from_level(level));
        for job in new_jobs {
            self.post_job(job);
        }
    }

    /// Slice of contracts that can be accepted.
    pub fn available_jobs(&self) -> &[JobContract] {
        &self.available
    }

    /// Slice of contracts currently in progress.
    pub fn active_jobs(&self) -> &[JobContract] {
        &self.active
    }

    /// How many contracts have been fulfilled.
    pub fn completed_count(&self) -> usize {
        self.completed.len()
    }

    /// Look up a contract by ID across all lists.
    pub fn find_job(&self, id: u32) -> Option<&JobContract> {
        self.available
            .iter()
            .chain(self.active.iter())
            .chain(self.completed.iter())
            .find(|j| j.id == id)
    }
}

// ──────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── JobDifficulty::from_level ───────────────

    #[test]
    fn difficulty_from_level() {
        assert_eq!(JobDifficulty::from_level(1), JobDifficulty::Trivial);
        assert_eq!(JobDifficulty::from_level(3), JobDifficulty::Trivial);
        assert_eq!(JobDifficulty::from_level(4), JobDifficulty::Easy);
        assert_eq!(JobDifficulty::from_level(6), JobDifficulty::Easy);
        assert_eq!(JobDifficulty::from_level(7), JobDifficulty::Medium);
        assert_eq!(JobDifficulty::from_level(12), JobDifficulty::Medium);
        assert_eq!(JobDifficulty::from_level(15), JobDifficulty::Hard);
        assert_eq!(JobDifficulty::from_level(20), JobDifficulty::Hard);
        assert_eq!(JobDifficulty::from_level(25), JobDifficulty::Legendary);
        assert_eq!(JobDifficulty::from_level(30), JobDifficulty::Legendary);
    }

    // ── JobContract creation + builder ──────────

    #[test]
    fn contract_creation_and_builder() {
        let contract = JobContract::new(42)
            .with_title("Test Contract")
            .with_description("A test.")
            .with_job_type(JobType::Bounty)
            .with_difficulty(JobDifficulty::Hard)
            .with_target_location([10.0, 0.0, -5.0])
            .with_reward_gold(200)
            .with_reward_xp(300)
            .with_reward_items(vec!["Sword".to_string()])
            .with_time_limit(Some(120.0))
            .with_required_level(12)
            .with_enemy_count(6)
            .with_client_name("Mira");

        assert_eq!(contract.id, 42);
        assert_eq!(contract.title, "Test Contract");
        assert_eq!(contract.description, "A test.");
        assert_eq!(contract.job_type, JobType::Bounty);
        assert_eq!(contract.difficulty, JobDifficulty::Hard);
        assert_eq!(contract.target_location, [10.0, 0.0, -5.0]);
        assert_eq!(contract.reward_gold, 200);
        assert_eq!(contract.reward_xp, 300);
        assert_eq!(contract.reward_items, vec!["Sword".to_string()]);
        assert_eq!(contract.time_limit, Some(120.0));
        assert_eq!(contract.required_level, 12);
        assert_eq!(contract.enemy_count, 6);
        assert_eq!(contract.client_name, "Mira");
        assert!(contract.is_available());
        assert!(!contract.is_complete());
    }

    // ── can_accept level gate ───────────────────

    #[test]
    fn can_accept_checks_level() {
        let contract = JobContract::new(1).with_required_level(5);
        let low_level = MercenaryStats::new("Rookie"); // level 1
        let high_level = MercenaryStats::new("Veteran").with_stats(20, 20, 20, 20);

        // Manually set levels via struct fields
        let mut high = high_level;
        high.level = 10;

        assert!(!contract.can_accept(&low_level));
        assert!(contract.can_accept(&high));
    }

    // ── Status transitions: Available → InProgress → Completed ──

    #[test]
    fn status_transition_available_to_completed() {
        let mut contract = JobContract::new(1);
        assert!(contract.is_available());

        contract.accept(7);
        assert_eq!(contract.status, JobStatus::InProgress { accepted_by: 7 });
        assert!(!contract.is_available());

        contract.complete();
        assert!(contract.is_complete());
        assert_eq!(contract.status, JobStatus::Completed);
    }

    // ── Failure transition ──────────────────────

    #[test]
    fn status_transition_fail() {
        let mut contract = JobContract::new(1);
        contract.accept(3);
        contract.fail();
        assert_eq!(contract.status, JobStatus::Failed);
    }

    // ── JobGenerator produces valid jobs ────────

    #[test]
    fn generator_produces_valid_jobs() {
        let mut gen = JobGenerator::new();
        let job = gen.generate_job(JobDifficulty::Medium);

        assert_eq!(job.id, 1);
        assert!(!job.title.is_empty());
        assert!(!job.description.is_empty());
        assert!(job.is_available());
        assert_eq!(job.enemy_count, 4);
        assert_eq!(job.time_limit, Some(180.0));
        assert_eq!(job.required_level, 6); // min of Medium range
        assert!(!job.client_name.is_empty());
    }

    // ── JobGenerator scales rewards by difficulty ──

    #[test]
    fn generator_scales_rewards_by_difficulty() {
        let mut gen = JobGenerator::new();

        let trivial = gen.generate_job(JobDifficulty::Trivial);
        let legendary = gen.generate_job(JobDifficulty::Legendary);

        // Trivial: gold = 50 * 0.3 = 15, xp = 100 * 0.5 = 50
        assert_eq!(trivial.reward_gold, 15);
        assert_eq!(trivial.reward_xp, 50);

        // Legendary: gold = 50 * 4.0 = 200, xp = 100 * 2.5 = 250
        assert_eq!(legendary.reward_gold, 200);
        assert_eq!(legendary.reward_xp, 250);

        // Legendary should have more items
        assert!(legendary.reward_items.len() > trivial.reward_items.len());
    }

    // ── JobBoard accept + complete cycle ────────

    #[test]
    fn board_accept_and_complete_cycle() {
        let mut board = JobBoard::new(5);
        let mut gen = JobGenerator::new();

        let job = gen.generate_job(JobDifficulty::Easy);
        let job_id = job.id;
        board.post_job(job);

        assert_eq!(board.available_jobs().len(), 1);

        let result = board.accept_job(job_id, 10); // level 10 >> Easy min level 3
        assert_eq!(result, AcceptResult::Accepted);
        assert_eq!(board.available_jobs().len(), 0);
        assert_eq!(board.active_jobs().len(), 1);

        let loot = board.complete_job(job_id).expect("should have loot");
        assert_eq!(board.active_jobs().len(), 0);
        assert_eq!(board.completed_count(), 1);
        assert_eq!(loot.gold, 30); // 50 * 0.6 = 30
        assert!(!loot.items.is_empty());
    }

    // ── JobBoard rejects low-level acceptance ───

    #[test]
    fn board_rejects_low_level() {
        let mut board = JobBoard::new(5);
        let mut gen = JobGenerator::new();

        let job = gen.generate_job(JobDifficulty::Hard); // required_level = 12
        let job_id = job.id;
        board.post_job(job);

        let result = board.accept_job(job_id, 5); // level 5 < 12
        assert_eq!(result, AcceptResult::LevelTooLow);
        assert_eq!(board.available_jobs().len(), 1); // still available
    }

    // ── JobBoard max active limit ───────────────

    #[test]
    fn board_max_active_limit() {
        let mut board = JobBoard::new(2);
        let mut gen = JobGenerator::new();

        // Post and accept 2 jobs
        for _ in 0..2 {
            let job = gen.generate_job(JobDifficulty::Trivial); // required_level = 1
            let jid = job.id;
            board.post_job(job);
            let r = board.accept_job(jid, 1);
            assert_eq!(r, AcceptResult::Accepted);
        }

        // Third should be rejected
        let job = gen.generate_job(JobDifficulty::Trivial);
        let jid = job.id;
        board.post_job(job);
        let r = board.accept_job(jid, 1);
        assert_eq!(r, AcceptResult::BoardFull);
    }

    // ── JobBoard refresh generates new jobs ─────

    #[test]
    fn board_refresh_generates_new_jobs() {
        let mut board = JobBoard::new(5);
        let mut gen = JobGenerator::new();

        // Initial state: empty
        assert_eq!(board.available_jobs().len(), 0);

        board.refresh(&mut gen, 3, 7); // level 7 → Medium

        assert_eq!(board.available_jobs().len(), 3);
        for job in board.available_jobs() {
            assert_eq!(job.difficulty, JobDifficulty::Medium);
        }
    }

    // ── JobBoard find_job across lists ──────────

    #[test]
    fn board_find_job_across_lists() {
        let mut board = JobBoard::new(5);
        let mut gen = JobGenerator::new();

        let a = gen.generate_job(JobDifficulty::Easy);
        let b = gen.generate_job(JobDifficulty::Medium);
        let c = gen.generate_job(JobDifficulty::Hard);
        let id_a = a.id;
        let id_b = b.id;
        let id_c = c.id;

        board.post_job(a);
        board.post_job(b);
        board.post_job(c);

        // Accept b
        board.accept_job(id_b, 20);

        // Complete c
        board.accept_job(id_c, 20);
        board.complete_job(id_c);

        // a is in available
        assert!(board.find_job(id_a).is_some());
        // b is in active
        assert!(board.find_job(id_b).is_some());
        // c is in completed
        assert!(board.find_job(id_c).is_some());
        // non-existent
        assert!(board.find_job(9999).is_none());
    }

    // ── JobType cycling ─────────────────────────

    #[test]
    fn job_type_cycles_deterministically() {
        assert_eq!(JobType::from_index(0), JobType::Bounty);
        assert_eq!(JobType::from_index(1), JobType::Escort);
        assert_eq!(JobType::from_index(2), JobType::Fetch);
        assert_eq!(JobType::from_index(3), JobType::ClearArea);
        assert_eq!(JobType::from_index(4), JobType::Defend);
        assert_eq!(JobType::from_index(5), JobType::Assassinate);
        assert_eq!(JobType::from_index(6), JobType::Bounty); // wraps
    }

    // ── Difficulty multiplier consistency ───────

    #[test]
    fn difficulty_multipliers_are_monotonic() {
        let tiers = [
            JobDifficulty::Trivial,
            JobDifficulty::Easy,
            JobDifficulty::Medium,
            JobDifficulty::Hard,
            JobDifficulty::Legendary,
        ];

        for window in tiers.windows(2) {
            assert!(
                window[1].xp_multiplier() > window[0].xp_multiplier(),
                "XP multiplier should increase: {:?} vs {:?}",
                window[0],
                window[1],
            );
            assert!(
                window[1].gold_multiplier() > window[0].gold_multiplier(),
                "Gold multiplier should increase: {:?} vs {:?}",
                window[0],
                window[1],
            );
            let (lo_min, lo_max) = window[0].level_range();
            let (hi_min, hi_max) = window[1].level_range();
            assert!(hi_min >= lo_min, "Level range should not decrease");
            assert!(hi_max > lo_max, "Level range max should increase");
        }
    }

    // ── Accept on non-available job is a no-op ──

    #[test]
    fn accept_on_non_available_is_noop() {
        let mut job = JobContract::new(1);
        job.accept(5); // Available → InProgress
        assert_eq!(job.status, JobStatus::InProgress { accepted_by: 5 });

        // Accepting again should be a no-op (already InProgress)
        job.accept(9);
        assert_eq!(job.status, JobStatus::InProgress { accepted_by: 5 });
    }

    // ── fail_job on non-existent id is safe ─────

    #[test]
    fn fail_nonexistent_job_is_safe() {
        let mut board = JobBoard::new(5);
        board.fail_job(9999); // should not panic
        assert_eq!(board.active_jobs().len(), 0);
    }
}
