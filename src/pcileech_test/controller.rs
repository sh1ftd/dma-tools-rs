use super::PcileechTestState;
use super::runner::{self, CancellationToken, RunOutcome};
use std::sync::{Arc, Mutex};
use std::thread;

type TestRunner = dyn Fn(CancellationToken) -> RunOutcome + Send + Sync;

#[derive(Debug)]
struct ActiveRun {
    generation: u64,
    cancellation: CancellationToken,
}

#[derive(Debug, Default)]
struct ControllerState {
    generation: u64,
    test_state: PcileechTestState,
    active_run: Option<ActiveRun>,
    requested_generation: Option<u64>,
    restart_blocked: Option<String>,
    back_requested: bool,
}

struct PendingLaunch {
    generation: u64,
    cancellation: CancellationToken,
}

pub struct PcileechTestController {
    state: Arc<Mutex<ControllerState>>,
    runner: Arc<TestRunner>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PcileechTestSnapshot {
    pub state: PcileechTestState,
    pub back_pending: bool,
    pub can_go_back: bool,
}

impl PcileechTestController {
    pub fn new() -> Self {
        Self::with_runner(Arc::new(|cancellation| {
            runner::run_pcileech_test(&cancellation)
        }))
    }

    fn with_runner(runner: Arc<TestRunner>) -> Self {
        Self {
            state: Arc::new(Mutex::new(ControllerState::default())),
            runner,
        }
    }

    pub fn ensure_started(&self) {
        let launch = {
            let mut state = self.state.lock().unwrap();
            if state.back_requested {
                return;
            }

            if state.test_state != PcileechTestState::Idle {
                return;
            }

            if let Some(error) = &state.restart_blocked {
                state.test_state = PcileechTestState::Failed(error.clone());
                return;
            }

            state.generation = state.generation.wrapping_add(1);
            state.test_state = PcileechTestState::Running;
            state.requested_generation = Some(state.generation);

            if let Some(active_run) = &state.active_run {
                active_run.cancellation.cancel();
            }

            prepare_launch(&mut state)
        };

        if let Some(launch) = launch {
            launch_run(Arc::clone(&self.state), Arc::clone(&self.runner), launch);
        }
    }

    pub fn retry(&self) {
        let launch = {
            let mut state = self.state.lock().unwrap();
            if state.back_requested {
                return;
            }

            if let Some(error) = &state.restart_blocked {
                state.test_state = PcileechTestState::Failed(error.clone());
                return;
            }

            state.generation = state.generation.wrapping_add(1);
            state.test_state = PcileechTestState::Running;
            state.requested_generation = Some(state.generation);

            if let Some(active_run) = &state.active_run {
                active_run.cancellation.cancel();
            }

            prepare_launch(&mut state)
        };

        if let Some(launch) = launch {
            launch_run(Arc::clone(&self.state), Arc::clone(&self.runner), launch);
        }
    }

    pub fn request_back(&self) {
        let mut state = self.state.lock().unwrap();
        if !state.back_requested {
            state.generation = state.generation.wrapping_add(1);
            state.requested_generation = None;
            state.back_requested = true;
        }

        if let Some(active_run) = &state.active_run {
            active_run.cancellation.cancel();
        }
    }

    pub fn acknowledge_back_if_ready(&self) -> bool {
        let mut state = self.state.lock().unwrap();
        if !back_ready(&state) {
            return false;
        }

        state.back_requested = false;
        state.test_state = PcileechTestState::Idle;
        true
    }

    pub fn snapshot(&self) -> PcileechTestSnapshot {
        let state = self.state.lock().unwrap();
        PcileechTestSnapshot {
            state: state.test_state.clone(),
            back_pending: state.back_requested,
            can_go_back: back_ready(&state),
        }
    }
}

impl Default for PcileechTestController {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for PcileechTestController {
    fn drop(&mut self) {
        if let Ok(mut state) = self.state.lock() {
            state.requested_generation = None;
            if let Some(active_run) = &state.active_run {
                active_run.cancellation.cancel();
            }
        }
    }
}

fn prepare_launch(state: &mut ControllerState) -> Option<PendingLaunch> {
    if state.active_run.is_some() || state.back_requested {
        return None;
    }

    let generation = state.requested_generation.take()?;
    let cancellation = CancellationToken::default();
    state.active_run = Some(ActiveRun {
        generation,
        cancellation: cancellation.clone(),
    });

    Some(PendingLaunch {
        generation,
        cancellation,
    })
}

fn back_ready(state: &ControllerState) -> bool {
    state.back_requested && state.active_run.is_none() && state.restart_blocked.is_none()
}

fn launch_run(
    shared_state: Arc<Mutex<ControllerState>>,
    runner: Arc<TestRunner>,
    launch: PendingLaunch,
) {
    thread::spawn(move || {
        let outcome = runner(launch.cancellation);
        let next_launch = {
            let mut state = shared_state.lock().unwrap();
            let is_active_generation = state
                .active_run
                .as_ref()
                .is_some_and(|active| active.generation == launch.generation);

            if !is_active_generation {
                return;
            }
            state.active_run = None;

            if outcome.safe_to_restart {
                if state.generation == launch.generation {
                    state.test_state = outcome.state;
                }
            } else {
                let error = restart_blocked_message(&outcome.state);
                state.restart_blocked = Some(error.clone());
                state.requested_generation = None;

                if state.generation == launch.generation
                    || state.test_state != PcileechTestState::Idle
                {
                    state.test_state = PcileechTestState::Failed(error);
                }
            }

            prepare_launch(&mut state)
        };

        if let Some(next_launch) = next_launch {
            launch_run(shared_state, runner, next_launch);
        }
    });
}

fn restart_blocked_message(state: &PcileechTestState) -> String {
    let detail = match state {
        PcileechTestState::Failed(error) => error.as_str(),
        _ => "The previous PCILeech test process may still be running",
    };
    format!("{detail}. Restart is blocked to avoid running concurrent hardware tests")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::mpsc;
    use std::time::Duration;

    #[derive(Debug, PartialEq, Eq)]
    enum RunnerEvent {
        Started(usize),
        Cancelled(usize),
    }

    #[test]
    fn back_from_a_completed_run_is_immediately_ready() {
        let controller = PcileechTestController::new();
        {
            let mut state = controller.state.lock().unwrap();
            state.test_state = PcileechTestState::Success("ok".into());
        }

        controller.request_back();

        assert!(controller.snapshot().can_go_back);
        assert!(controller.acknowledge_back_if_ready());
        assert_eq!(controller.snapshot().state, PcileechTestState::Idle);
    }

    #[test]
    fn back_is_available_after_a_safe_reader_bookkeeping_failure() {
        let runner = Arc::new(|_cancellation: CancellationToken| {
            RunOutcome::safe(PcileechTestState::Failed(
                "Test tool output reader panicked".into(),
            ))
        }) as Arc<TestRunner>;
        let controller = PcileechTestController::with_runner(runner);

        controller.ensure_started();
        let deadline = std::time::Instant::now() + Duration::from_secs(1);
        while controller.snapshot().state == PcileechTestState::Running
            && std::time::Instant::now() < deadline
        {
            thread::yield_now();
        }

        assert!(
            matches!(controller.snapshot().state, PcileechTestState::Failed(error) if error.contains("reader panicked"))
        );
        controller.request_back();
        assert!(controller.snapshot().can_go_back);
        assert!(controller.acknowledge_back_if_ready());
    }

    #[test]
    fn retry_cancels_before_starting_replacement() {
        let active_workers = Arc::new(AtomicUsize::new(0));
        let maximum_workers = Arc::new(AtomicUsize::new(0));
        let next_run = Arc::new(AtomicUsize::new(0));
        let allow_replacement_to_finish = Arc::new(AtomicBool::new(false));
        let (event_sender, event_receiver) = mpsc::channel();

        let runner = {
            let active_workers = Arc::clone(&active_workers);
            let maximum_workers = Arc::clone(&maximum_workers);
            let next_run = Arc::clone(&next_run);
            let allow_replacement_to_finish = Arc::clone(&allow_replacement_to_finish);
            Arc::new(move |cancellation: CancellationToken| {
                let run = next_run.fetch_add(1, Ordering::SeqCst) + 1;
                let active = active_workers.fetch_add(1, Ordering::SeqCst) + 1;
                maximum_workers.fetch_max(active, Ordering::SeqCst);
                event_sender.send(RunnerEvent::Started(run)).unwrap();

                while !cancellation.is_cancelled() {
                    thread::yield_now();
                }

                active_workers.fetch_sub(1, Ordering::SeqCst);
                event_sender.send(RunnerEvent::Cancelled(run)).unwrap();
                if run == 2 {
                    // Keep the replacement registered as active until the test
                    // has observed the pending-back state. Without this gate,
                    // the worker can validly finish between `request_back` and
                    // the snapshot, making the timing assertion flaky.
                    while !allow_replacement_to_finish.load(Ordering::SeqCst) {
                        thread::yield_now();
                    }
                }
                RunOutcome::safe(PcileechTestState::Failed("cancelled".into()))
            }) as Arc<TestRunner>
        };

        let controller = PcileechTestController::with_runner(runner);
        controller.ensure_started();
        assert_eq!(
            event_receiver.recv_timeout(Duration::from_secs(1)).unwrap(),
            RunnerEvent::Started(1)
        );

        controller.retry();
        assert_eq!(
            event_receiver.recv_timeout(Duration::from_secs(1)).unwrap(),
            RunnerEvent::Cancelled(1)
        );
        assert_eq!(
            event_receiver.recv_timeout(Duration::from_secs(1)).unwrap(),
            RunnerEvent::Started(2)
        );
        assert_eq!(maximum_workers.load(Ordering::SeqCst), 1);

        controller.request_back();
        assert!(controller.snapshot().back_pending);
        assert!(!controller.snapshot().can_go_back);
        assert_eq!(
            event_receiver.recv_timeout(Duration::from_secs(1)).unwrap(),
            RunnerEvent::Cancelled(2)
        );
        assert!(!controller.snapshot().can_go_back);
        allow_replacement_to_finish.store(true, Ordering::SeqCst);
        let deadline = std::time::Instant::now() + Duration::from_secs(1);
        while !controller.snapshot().can_go_back && std::time::Instant::now() < deadline {
            thread::yield_now();
        }
        assert!(controller.acknowledge_back_if_ready());
        assert_eq!(controller.snapshot().state, PcileechTestState::Idle);
        assert_eq!(active_workers.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn unsafe_termination_blocks_a_queued_retry() {
        let starts = Arc::new(AtomicUsize::new(0));
        let (started_sender, started_receiver) = mpsc::channel();
        let runner = {
            let starts = Arc::clone(&starts);
            Arc::new(move |cancellation: CancellationToken| {
                starts.fetch_add(1, Ordering::SeqCst);
                started_sender.send(()).unwrap();
                while !cancellation.is_cancelled() {
                    thread::yield_now();
                }
                RunOutcome::unsafe_to_restart(PcileechTestState::Failed(
                    "Failed to terminate test tool".into(),
                ))
            }) as Arc<TestRunner>
        };

        let controller = PcileechTestController::with_runner(runner);
        controller.ensure_started();
        started_receiver
            .recv_timeout(Duration::from_secs(1))
            .unwrap();
        controller.retry();

        let deadline = std::time::Instant::now() + Duration::from_secs(1);
        while controller.snapshot().state == PcileechTestState::Running
            && std::time::Instant::now() < deadline
        {
            thread::yield_now();
        }

        assert_eq!(starts.load(Ordering::SeqCst), 1);
        assert!(
            matches!(controller.snapshot().state, PcileechTestState::Failed(error) if error.contains("Restart is blocked"))
        );
    }
}
