use std::{
    sync::LazyLock,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use gpui_util::ResultExt;
use windows::Win32::{
    Foundation::{HANDLE, HWND},
    Graphics::Dwm::{DWM_TIMING_INFO, DwmFlush, DwmGetCompositionTimingInfo},
    System::{
        LibraryLoader::{GetModuleHandleW, GetProcAddress},
        Performance::{QueryPerformanceCounter, QueryPerformanceFrequency},
    },
};
use windows::core::{s, w};

static QPC_TICKS_PER_SECOND: LazyLock<u64> = LazyLock::new(|| {
    let mut frequency = 0;
    // On systems that run Windows XP or later, the function will always succeed and
    // will thus never return zero.
    unsafe { QueryPerformanceFrequency(&mut frequency).unwrap() };
    frequency as u64
});

const VSYNC_INTERVAL_THRESHOLD: Duration = Duration::from_millis(1);
const DEFAULT_VSYNC_INTERVAL: Duration = Duration::from_micros(16_666); // ~60Hz

static FRAME_THREAD: std::sync::OnceLock<std::thread::Thread> = std::sync::OnceLock::new();

pub(crate) fn register_frame_thread() {
    let _ = FRAME_THREAD.set(std::thread::current());
}

pub(crate) fn wake_frame_thread() {
    if let Some(thread) = FRAME_THREAD.get() {
        thread.unpark();
    }
}

/// Per-window policy. Meter callbacks deliberately do not count as activity.
pub(crate) struct FramePacing {
    pub vsync: Option<bool>,
    pub active: bool,
    pub last_activity: Instant,
    last_input: Option<Instant>,
    pub last_tick: Instant,
}

impl FramePacing {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            vsync: None,
            active: false,
            last_activity: now,
            last_input: None,
            last_tick: now,
        }
    }

    pub fn interval(&self, now: Instant) -> Duration {
        let Some(vsync) = self.vsync else {
            return Duration::ZERO;
        };
        let recent_input = self
            .last_input
            .is_some_and(|input| now.duration_since(input) < Duration::from_secs(1));
        if (!self.active && !recent_input)
            || now.duration_since(self.last_activity) >= Duration::from_secs(1)
        {
            Duration::from_secs(1)
        } else if vsync {
            Duration::ZERO
        } else {
            Duration::from_nanos(16_666_667)
        }
    }

    /// Input wakes background windows too; background animations cannot extend that lease.
    pub fn note_activity(&mut self, now: Instant, is_input: bool) -> bool {
        let was_idle = self.interval(now) == Duration::from_secs(1);
        self.last_activity = now;
        if is_input {
            self.last_input = Some(now);
        }
        was_idle && self.interval(now) != Duration::from_secs(1)
    }

    pub fn remaining(&self, now: Instant) -> Duration {
        self.interval(now)
            .saturating_sub(now.duration_since(self.last_tick))
    }

    pub fn did_tick(&mut self, now: Instant) {
        let interval = self.interval(now);
        // Keep a fixed timer cadence without accumulating wake/Present overhead.
        // After a missed whole interval, rebase rather than issuing a catch-up burst.
        self.last_tick = if !interval.is_zero() && now.duration_since(self.last_tick) < interval * 2
        {
            self.last_tick + interval
        } else {
            now
        };
    }
}

/// QPC timestamp passed through `WM_GPUI_VSYNC`'s wparam so the handler can
/// measure post→dispatch latency (AGENDA frame diagnostics).
pub(crate) fn qpc_now() -> usize {
    let mut v = 0i64;
    let _ = unsafe { QueryPerformanceCounter(&mut v) };
    v as usize
}

pub(crate) fn qpc_elapsed_ms(then: usize) -> f32 {
    (qpc_now() as i64 - then as i64) as f32 * 1000.0 / *QPC_TICKS_PER_SECOND as f32
}

pub(crate) struct VSyncProvider {
    interval: Duration,
    f: Box<dyn Fn() -> bool>,
    compositor_clock: bool,
}

impl VSyncProvider {
    pub(crate) fn new() -> Self {
        let interval = get_dwm_interval()
            .context("Failed to get DWM interval")
            .log_err()
            .unwrap_or(DEFAULT_VSYNC_INTERVAL);
        // Resolve dynamically: this clock exists on Windows 11+, while GPUI
        // also supports Windows 10. DwmFlush waits for this app's queued work,
        // which makes it a less stable tick source under rendering load.
        // AGENDA_VSYNC_SOURCE=dwm keeps the old source available for A/B runs.
        type WaitForClock = unsafe extern "system" fn(u32, *const HANDLE, u32) -> u32;
        let wait_for_clock = unsafe {
            GetModuleHandleW(w!("dcomp.dll")).ok().and_then(|module| {
                GetProcAddress(module, s!("DCompositionWaitForCompositorClock")).map(|f| {
                    std::mem::transmute::<unsafe extern "system" fn() -> isize, WaitForClock>(f)
                })
            })
        };
        let use_clock = std::env::var("AGENDA_VSYNC_SOURCE").as_deref() != Ok("dwm");
        let compositor_clock = use_clock && wait_for_clock.is_some();
        let f: Box<dyn Fn() -> bool> = if let Some(wait) = wait_for_clock.filter(|_| use_clock) {
            Box::new(move || unsafe { wait(0, std::ptr::null(), 1000) == 0 })
        } else {
            Box::new(|| unsafe { DwmFlush().is_ok() })
        };
        if std::env::var_os("AGENDA_FRAME_LOG").is_some() {
            eprintln!(
                "[vsync] source={} refresh_hz={:.4} interval_ms={:.4}",
                if use_clock && wait_for_clock.is_some() {
                    "compositor-clock"
                } else {
                    "dwm"
                },
                1.0 / interval.as_secs_f64(),
                interval.as_secs_f64() * 1000.0
            );
        }
        Self {
            interval,
            f,
            compositor_clock,
        }
    }

    pub(crate) fn wait_for_vsync(&self) {
        let vsync_start = Instant::now();
        let wait_succeeded = (self.f)();
        let elapsed = vsync_start.elapsed();
        // The compositor clock reports occlusion as a failure; a short successful
        // wait is a real tick. DwmFlush can succeed immediately while occluded.
        // Do not add another refresh interval after a valid late clock wakeup.
        if !wait_succeeded || (!self.compositor_clock && elapsed < VSYNC_INTERVAL_THRESHOLD) {
            log::trace!("VSyncProvider::wait_for_vsync() took less time than expected");
            std::thread::sleep(self.interval.saturating_sub(elapsed));
        }
    }
}

fn get_dwm_interval() -> Result<Duration> {
    let mut timing_info = DWM_TIMING_INFO {
        cbSize: std::mem::size_of::<DWM_TIMING_INFO>() as u32,
        ..Default::default()
    };
    unsafe { DwmGetCompositionTimingInfo(HWND::default(), &mut timing_info) }?;
    let interval = retrieve_duration(timing_info.qpcRefreshPeriod, *QPC_TICKS_PER_SECOND);
    // Check for interval values that are impossibly low. A 29 microsecond
    // interval was seen (from a qpcRefreshPeriod of 60).
    if interval < VSYNC_INTERVAL_THRESHOLD {
        anyhow::ensure!(
            timing_info.rateRefresh.uiNumerator != 0,
            "Zero refresh rate"
        );
        Ok(retrieve_duration(
            timing_info.rateRefresh.uiDenominator as u64,
            timing_info.rateRefresh.uiNumerator as u64,
        ))
    } else {
        Ok(interval)
    }
}

#[inline]
fn retrieve_duration(counts: u64, ticks_per_second: u64) -> Duration {
    Duration::from_secs_f64(counts as f64 / ticks_per_second as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_pacing_power_states() {
        let mut pacing = FramePacing::new();
        let now = pacing.last_activity;
        assert_eq!(pacing.interval(now), Duration::ZERO); // Unconfigured GPUI window.
        pacing.vsync = Some(true);
        pacing.active = true;
        assert_eq!(pacing.interval(now), Duration::ZERO);
        assert_eq!(
            pacing.interval(now + Duration::from_secs(1)),
            Duration::from_secs(1)
        );
        pacing.last_activity = now + Duration::from_secs(2); // Input/animation wakes immediately.
        let now = pacing.last_activity;
        assert_eq!(pacing.interval(now), Duration::ZERO);
        pacing.vsync = Some(false);
        assert_eq!(pacing.interval(now), Duration::from_nanos(16_666_667));
        pacing.last_tick = now;
        assert!(!pacing.remaining(now + Duration::from_millis(10)).is_zero());
        assert!(pacing.remaining(now + Duration::from_millis(17)).is_zero());
        pacing.did_tick(now + Duration::from_millis(17));
        assert_eq!(pacing.last_tick, now + Duration::from_nanos(16_666_667));
        pacing.active = false;
        assert_eq!(pacing.interval(now), Duration::from_secs(1));
        assert!(!pacing.note_activity(now, false)); // Background animation stays capped.
        assert!(pacing.note_activity(now, true)); // Unfocused scrolling wakes immediately.
        assert_eq!(pacing.interval(now), Duration::from_nanos(16_666_667));
        pacing.vsync = Some(true);
        assert_eq!(pacing.interval(now), Duration::ZERO);
        assert!(!pacing.note_activity(now + Duration::from_millis(900), false));
        assert_eq!(
            pacing.interval(now + Duration::from_secs(1)),
            Duration::from_secs(1)
        );
        assert!(pacing.note_activity(now + Duration::from_secs(2), true));
        assert_eq!(
            pacing.interval(now + Duration::from_secs(2)),
            Duration::ZERO
        );
    }

    #[test]
    fn test_frame_pacing_refresh_ratio() {
        // The refresh-rate fallback is a ratio (1/165), not a MHz QPC clock.
        assert_eq!(retrieve_duration(1, 165), Duration::from_nanos(6_060_606));
        assert_eq!(
            retrieve_duration(60_606, 10_000_000),
            Duration::from_nanos(6_060_600)
        );
    }
}
