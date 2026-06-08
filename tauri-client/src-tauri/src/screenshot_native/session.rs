use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

static SCREENSHOT_SESSION_COUNTER: AtomicU64 = AtomicU64::new(1);
static SCREENSHOT_RUN_GENERATION: AtomicU64 = AtomicU64::new(1);
const SESSION_ID_PREFIX: &str = "ss";

#[cfg(test)]
const FIRST_SESSION_ID: u64 = 1;
#[cfg(test)]
const FIRST_RUN_GENERATION: u64 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScreenshotRunGeneration(u64);

impl fmt::Display for ScreenshotRunGeneration {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl ScreenshotRunGeneration {
    #[cfg(test)]
    fn first() -> Self {
        Self(FIRST_RUN_GENERATION)
    }

    pub fn value(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenshotGenerationState {
    Current,
    Stale,
}

impl ScreenshotGenerationState {
    pub fn is_stale(self) -> bool {
        matches!(self, Self::Stale)
    }
}

pub fn next_screenshot_session_id() -> String {
    let session_id = SCREENSHOT_SESSION_COUNTER.fetch_add(1, Ordering::Relaxed);

    format!("{SESSION_ID_PREFIX}-{session_id}")
}

pub fn begin_run_generation() -> ScreenshotRunGeneration {
    advance_run_generation()
}

pub fn advance_run_generation() -> ScreenshotRunGeneration {
    let generation = SCREENSHOT_RUN_GENERATION.fetch_add(1, Ordering::SeqCst) + 1;

    ScreenshotRunGeneration(generation)
}

pub fn generation_state(generation: ScreenshotRunGeneration) -> ScreenshotGenerationState {
    if SCREENSHOT_RUN_GENERATION.load(Ordering::SeqCst) == generation.value() {
        ScreenshotGenerationState::Current
    } else {
        ScreenshotGenerationState::Stale
    }
}

pub fn is_stale_generation(generation: ScreenshotRunGeneration) -> bool {
    generation_state(generation).is_stale()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static SESSION_TEST_LOCK: Mutex<()> = Mutex::new(());

    fn reset_session_contract_for_test() {
        SCREENSHOT_SESSION_COUNTER.store(FIRST_SESSION_ID, Ordering::SeqCst);
        SCREENSHOT_RUN_GENERATION.store(FIRST_RUN_GENERATION, Ordering::SeqCst);
    }

    #[test]
    fn session_ids_are_monotonic_and_prefixed() {
        let _guard = SESSION_TEST_LOCK
            .lock()
            .expect("session test lock poisoned");
        reset_session_contract_for_test();

        assert_eq!(next_screenshot_session_id(), "ss-1");
        assert_eq!(next_screenshot_session_id(), "ss-2");
        assert_eq!(next_screenshot_session_id(), "ss-3");
    }

    #[test]
    fn begin_and_advance_share_generation_contract() {
        let _guard = SESSION_TEST_LOCK
            .lock()
            .expect("session test lock poisoned");
        reset_session_contract_for_test();

        let run_generation = begin_run_generation();
        assert_eq!(run_generation.value(), 2);
        assert_eq!(
            generation_state(run_generation),
            ScreenshotGenerationState::Current
        );
        assert!(!is_stale_generation(run_generation));
        assert_eq!(run_generation.to_string(), "2");

        let next_generation = advance_run_generation();
        assert_eq!(next_generation.value(), 3);
        assert_eq!(
            generation_state(run_generation),
            ScreenshotGenerationState::Stale
        );
        assert!(is_stale_generation(run_generation));
        assert_eq!(
            generation_state(next_generation),
            ScreenshotGenerationState::Current
        );
    }

    #[test]
    fn initial_generation_is_stale_after_run_begins() {
        let _guard = SESSION_TEST_LOCK
            .lock()
            .expect("session test lock poisoned");
        reset_session_contract_for_test();

        let initial_generation = ScreenshotRunGeneration::first();
        assert_eq!(
            generation_state(initial_generation),
            ScreenshotGenerationState::Current
        );

        let run_generation = begin_run_generation();
        assert_eq!(
            generation_state(initial_generation),
            ScreenshotGenerationState::Stale
        );
        assert_eq!(
            generation_state(run_generation),
            ScreenshotGenerationState::Current
        );
    }
}
