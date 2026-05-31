//! Frame Profiler — performance instrumentation for the Chronos Engine.
//!
//! Tracks per-system execution times, frame duration, entity counts,
//! and component memory estimates. Useful for the dev overlay and
//! for identifying performance bottlenecks.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// A single timing sample for a named region (e.g., a system).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimingSample {
    pub name: &'static str,
    pub elapsed_us: u64,
    pub tick: u64,
}

/// Statistics for a single profiler counter over a rolling window.
#[derive(Debug, Clone, PartialEq)]
pub struct CounterStats {
    pub name: &'static str,
    pub min: u64,
    pub max: u64,
    pub avg: u64,
    pub last: u64,
    pub history: VecDeque<u64>,
}

impl CounterStats {
    fn new(name: &'static str, window_size: usize) -> Self {
        CounterStats {
            name,
            min: 0,
            max: 0,
            avg: 0,
            last: 0,
            history: VecDeque::with_capacity(window_size),
        }
    }

    fn push(&mut self, value: u64) {
        self.last = value;
        if self.history.is_empty() {
            self.min = value;
            self.max = value;
        } else {
            self.min = self.min.min(value);
            self.max = self.max.max(value);
        }
        if self.history.len() == self.history.capacity() {
            self.history.pop_front();
        }
        self.history.push_back(value);
        let sum: u64 = self.history.iter().sum();
        self.avg = sum / self.history.len() as u64;
    }
}

/// Frame profiler that records timing and counters across the engine.
///
/// Use [`FrameProfiler::begin_frame`] at the start of each frame,
/// [`FrameProfiler::region`] to time named blocks, and
/// [`FrameProfiler::end_frame`] to collect statistics.
#[derive(Debug)]
pub struct FrameProfiler {
    frame_start: Option<Instant>,
    region_start: Option<Instant>,
    region_name: Option<&'static str>,
    pub current_tick: u64,

    /// Per-region timing stats (key = region name).
    pub region_stats: Vec<(String, CounterStats)>,

    /// Overall frame time stats.
    pub frame_stats: CounterStats,

    /// Entity count tracking.
    pub entity_stats: CounterStats,

    /// Component count tracking.
    pub component_stats: CounterStats,

    /// Rolling window size for averages.
    window_size: usize,

    /// Samples collected since the last flush.
    samples: Vec<TimingSample>,
}

impl FrameProfiler {
    /// Create a profiler with the given rolling-window size.
    pub fn new(window_size: usize) -> Self {
        FrameProfiler {
            frame_start: None,
            region_start: None,
            region_name: None,
            current_tick: 0,
            region_stats: Vec::new(),
            frame_stats: CounterStats::new("frame_time", window_size),
            entity_stats: CounterStats::new("entity_count", window_size),
            component_stats: CounterStats::new("component_count", window_size),
            window_size,
            samples: Vec::new(),
        }
    }

    /// Begin timing a new frame.
    pub fn begin_frame(&mut self, tick: u64) {
        self.current_tick = tick;
        self.frame_start = Some(Instant::now());
        self.samples.clear();
    }

    /// Start timing a named region (system, render pass, etc.).
    ///
    /// Only one region may be active at a time. Call [`end_region`](Self::end_region)
    /// before beginning another.
    pub fn begin_region(&mut self, name: &'static str) {
        self.region_start = Some(Instant::now());
        self.region_name = Some(name);
    }

    /// End the currently-active region and record its elapsed time.
    pub fn end_region(&mut self) {
        if let (Some(start), Some(name)) = (self.region_start, self.region_name) {
            let elapsed = start.elapsed().as_micros() as u64;
            self.samples.push(TimingSample {
                name,
                elapsed_us: elapsed,
                tick: self.current_tick,
            });

            // Update stats for this region
            if let Some((_, stats)) = self.region_stats.iter_mut().find(|(n, _)| n == name) {
                stats.push(elapsed);
            } else {
                let mut stats = CounterStats::new(name, self.window_size);
                stats.push(elapsed);
                self.region_stats.push((name.to_string(), stats));
            }

            self.region_start = None;
            self.region_name = None;
        }
    }

    /// Convenience: time a closure under a named region.
    pub fn region<F, R>(&mut self, name: &'static str, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        self.begin_region(name);
        let result = f();
        self.end_region();
        result
    }

    /// End the current frame and record aggregate stats.
    ///
    /// `entity_count` and `component_count` are provided by the caller
    /// (typically from the World).
    pub fn end_frame(&mut self, entity_count: usize, component_count: usize) {
        if let Some(start) = self.frame_start {
            let elapsed = start.elapsed().as_micros() as u64;
            self.frame_stats.push(elapsed);
        }
        self.entity_stats.push(entity_count as u64);
        self.component_stats.push(component_count as u64);
        self.frame_start = None;
    }

    /// Reset all collected statistics.
    pub fn reset(&mut self) {
        self.region_stats.clear();
        self.frame_stats = CounterStats::new("frame_time", self.window_size);
        self.entity_stats = CounterStats::new("entity_count", self.window_size);
        self.component_stats = CounterStats::new("component_count", self.window_size);
        self.samples.clear();
    }

    /// Estimate total memory used by tracked components (very rough).
    pub fn estimated_memory_kb(&self) -> u64 {
        let entities = self.entity_stats.last;
        let components = self.component_stats.last;
        // Rough heuristic: 64 bytes per entity + 32 bytes per component
        (entities * 64 + components * 32) / 1024
    }

    /// Build a human-readable report of the last frame.
    pub fn last_frame_report(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!(
            "Frame {}: {:.2} ms | Entities: {} | Components: {} | Mem: ~{} KB",
            self.current_tick,
            self.frame_stats.last as f64 / 1000.0,
            self.entity_stats.last,
            self.component_stats.last,
            self.estimated_memory_kb()
        ));
        for (name, stats) in &self.region_stats {
            lines.push(format!(
                "  {:20} {:8.3} ms  (avg {:8.3}  min {:8.3}  max {:8.3})",
                name,
                stats.last as f64 / 1000.0,
                stats.avg as f64 / 1000.0,
                stats.min as f64 / 1000.0,
                stats.max as f64 / 1000.0
            ));
        }
        lines.join("\n")
    }
}

impl Default for FrameProfiler {
    fn default() -> Self {
        Self::new(60)
    }
}

// ──────────────────────────────────────────────────────────────
// GPU Timer Stub (ready for future wgpu integration)
// ──────────────────────────────────────────────────────────────

/// Placeholder for GPU-side timing. When the `render` feature is
/// active this can be extended to use wgpu timestamp queries.
#[derive(Debug, Clone)]
pub struct GpuTimer {
    pub label: String,
    pub elapsed_us: u64,
}

impl GpuTimer {
    pub fn new(label: impl Into<String>) -> Self {
        GpuTimer {
            label: label.into(),
            elapsed_us: 0,
        }
    }

    /// Stub: returns zero until real GPU timing is wired in.
    pub fn elapsed(&self) -> Duration {
        Duration::from_micros(self.elapsed_us)
    }
}

// ──────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    // Test 1: Basic frame timing.
    #[test]
    fn profiler_frame_timing() {
        let mut p = FrameProfiler::new(10);
        p.begin_frame(1);
        thread::sleep(Duration::from_millis(5));
        p.end_frame(10, 20);
        assert!(p.frame_stats.last >= 4000); // at least 4 ms in microseconds
    }

    // Test 2: Region timing.
    #[test]
    fn profiler_region_timing() {
        let mut p = FrameProfiler::new(10);
        p.begin_frame(1);
        p.begin_region("physics");
        thread::sleep(Duration::from_millis(2));
        p.end_region();
        p.end_frame(5, 10);

        assert_eq!(p.samples.len(), 1);
        assert_eq!(p.samples[0].name, "physics");
        assert!(p.samples[0].elapsed_us >= 1500);
    }

    // Test 3: Region closure helper.
    #[test]
    fn profiler_region_closure() {
        let mut p = FrameProfiler::new(10);
        p.begin_frame(1);
        let result = p.region("math", || {
            let mut sum = 0;
            for i in 0..1000 {
                sum += i;
            }
            sum
        });
        assert_eq!(result, 499500);
        p.end_frame(1, 1);
        assert!(p.region_stats.iter().any(|(n, _)| n == "math"));
    }

    // Test 4: Rolling window size on CounterStats.
    #[test]
    fn profiler_rolling_window() {
        let mut p = FrameProfiler::new(3);
        for tick in 1..=5 {
            p.begin_frame(tick);
            p.end_frame(tick as usize, tick as usize);
        }
        // Window size is 3 — history should never exceed 3 entries
        assert_eq!(p.frame_stats.history.len(), 3);
        // Entity/component stats also use the same window
        assert_eq!(p.entity_stats.history.len(), 3);
    }

    // Test 5: Reset clears everything.
    #[test]
    fn profiler_reset() {
        let mut p = FrameProfiler::new(10);
        p.begin_frame(1);
        p.begin_region("test");
        p.end_region();
        p.end_frame(1, 1);
        p.reset();
        assert!(p.region_stats.is_empty());
        assert_eq!(p.frame_stats.last, 0);
    }

    // Test 6: Estimated memory.
    #[test]
    fn profiler_memory_estimate() {
        let mut p = FrameProfiler::new(10);
        p.begin_frame(1);
        p.end_frame(100, 200);
        let mem = p.estimated_memory_kb();
        // (100*64 + 200*32) / 1024 = 12695 / 1024 = 12
        assert_eq!(mem, 12);
    }

    // Test 7: Report formatting.
    #[test]
    fn profiler_report_format() {
        let mut p = FrameProfiler::new(10);
        p.begin_frame(42);
        p.begin_region("render");
        p.end_region();
        p.end_frame(10, 20);
        let report = p.last_frame_report();
        assert!(report.contains("Frame 42"));
        assert!(report.contains("render"));
    }

    // Test 8: CounterStats min/max/avg.
    #[test]
    fn counter_stats() {
        let mut s = CounterStats::new("test", 5);
        s.push(10);
        s.push(20);
        s.push(30);
        assert_eq!(s.min, 10);
        assert_eq!(s.max, 30);
        assert_eq!(s.avg, 20);
    }

    // Test 9: GpuTimer stub.
    #[test]
    fn gpu_timer_stub() {
        let t = GpuTimer::new("shadow_pass");
        assert_eq!(t.label, "shadow_pass");
        assert_eq!(t.elapsed(), Duration::ZERO);
    }

    // Test 10: Default profiler.
    #[test]
    fn profiler_default() {
        let p = FrameProfiler::default();
        assert_eq!(p.window_size, 60);
    }
}
