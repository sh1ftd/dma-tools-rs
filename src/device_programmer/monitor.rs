use crate::device_programmer::operation::{
    OpenOcdEvent, OperationTracker, ProgressSnapshot, SectorStats, connection_is_unstable,
    operation_progress_snapshot,
};
use crate::device_programmer::process::ProcessTerminator;
use crate::utils::logger::Logger;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const MONITOR_CHECK_INTERVAL_MS: u64 = 50;
const UNSTABLE_CONFIRMATION_OBSERVATIONS: u32 = 2;

fn unstable_connection_confirmed(stats: SectorStats, consecutive_observations: &mut u32) -> bool {
    if connection_is_unstable(stats) {
        *consecutive_observations = consecutive_observations.saturating_add(1);
    } else {
        *consecutive_observations = 0;
    }

    *consecutive_observations >= UNSTABLE_CONFIRMATION_OBSERVATIONS
}

pub struct OperationMonitor {
    tracker: OperationTracker,
    terminated_early: Arc<AtomicBool>,
    monitor_running: Arc<AtomicBool>,
    monitor_generation: Arc<AtomicU64>,
    line_processing: Arc<Mutex<()>>,
    logger: Logger,
}

impl OperationMonitor {
    pub fn new(logger: Logger) -> Self {
        Self {
            tracker: OperationTracker::default(),
            terminated_early: Arc::new(AtomicBool::new(false)),
            monitor_running: Arc::new(AtomicBool::new(false)),
            monitor_generation: Arc::new(AtomicU64::new(0)),
            line_processing: Arc::new(Mutex::new(())),
            logger,
        }
    }

    pub fn reset_counters(&self) {
        // A callback can otherwise pass its generation check immediately before
        // reset and then record a stale event into the freshly cleared tracker.
        let _line_guard = self.line_processing.lock().unwrap();
        self.stop_monitor_thread();
        self.tracker.reset();
        self.terminated_early.store(false, Ordering::SeqCst);
        self.monitor_running.store(false, Ordering::SeqCst);
        self.logger
            .debug("OperationMonitor: all progress and flags have been reset");
    }

    pub fn was_terminated_early(&self) -> bool {
        self.terminated_early.load(Ordering::SeqCst)
    }

    pub fn progress_snapshot(&self) -> ProgressSnapshot {
        operation_progress_snapshot(&self.tracker)
    }

    pub fn create_line_monitor(
        &self,
        logger: Logger,
        process_terminator: ProcessTerminator,
    ) -> Box<dyn Fn(&str) + Send + Sync + 'static> {
        let tracker = self.tracker.clone();
        let terminated_early = Arc::clone(&self.terminated_early);
        let monitor_generation = Arc::clone(&self.monitor_generation);
        let line_processing = Arc::clone(&self.line_processing);

        self.start_monitor_thread(logger.clone(), Arc::clone(&process_terminator));
        let generation = monitor_generation.load(Ordering::SeqCst);

        Box::new(move |line: &str| {
            // Stdout and stderr are consumed concurrently. Serialize the terminal
            // check with event recording so no buffered line can mutate progress
            // after an unstable connection has stopped the operation.
            let _line_guard = line_processing.lock().unwrap();
            if terminated_early.load(Ordering::SeqCst)
                || generation != monitor_generation.load(Ordering::SeqCst)
            {
                return;
            }

            logger.debug(format!("Monitor processing: {line}"));

            let Some(event) = tracker.record_line(line) else {
                return;
            };

            if !matches!(event, OpenOcdEvent::SectorWritten { .. }) {
                return;
            }

            let stats = tracker.sector_stats();
            logger.debug(format!(
                "Current sector stats: {}/{} normal writes",
                stats.normal, stats.total
            ));
        })
    }

    fn start_monitor_thread(&self, logger: Logger, process_terminator: ProcessTerminator) {
        let tracker = self.tracker.clone();
        let terminated_early = Arc::clone(&self.terminated_early);
        let monitor_running = Arc::clone(&self.monitor_running);
        let monitor_generation = Arc::clone(&self.monitor_generation);
        let line_processing = Arc::clone(&self.line_processing);

        if monitor_running.swap(true, Ordering::SeqCst) {
            return;
        }
        let generation = monitor_generation.load(Ordering::SeqCst);

        thread::spawn(move || {
            logger.info("Starting real-time sector write monitoring thread");
            let mut previous_stats = tracker.sector_stats();
            let mut check_count = 0_u64;
            let mut consecutive_unstable_observations = 0_u32;

            while monitor_running.load(Ordering::SeqCst)
                && generation == monitor_generation.load(Ordering::SeqCst)
            {
                // Serialize the observation and terminal decision with line recording.
                // A reset can invalidate this generation while the monitor is waiting
                // for the guard, so re-check both conditions after acquiring it.
                let _line_guard = line_processing.lock().unwrap();
                if !monitor_running.load(Ordering::SeqCst)
                    || generation != monitor_generation.load(Ordering::SeqCst)
                {
                    break;
                }

                let stats = tracker.sector_stats();
                check_count += 1;

                if check_count.is_multiple_of(10) || stats != previous_stats {
                    logger.debug(format!(
                        "[Monitor Thread] Check #{check_count}: {}/{} normal writes",
                        stats.normal, stats.total
                    ));
                    previous_stats = stats;
                }

                let previously_unstable = consecutive_unstable_observations > 0;
                let unstable_confirmed =
                    unstable_connection_confirmed(stats, &mut consecutive_unstable_observations);

                if consecutive_unstable_observations > 0 {
                    logger.debug(format!(
                        "Connection remains below the normal-write threshold: {}/{} (observation {}/{})",
                        stats.normal,
                        stats.total,
                        consecutive_unstable_observations,
                        UNSTABLE_CONFIRMATION_OBSERVATIONS
                    ));

                    if unstable_confirmed {
                        Self::terminate_process(
                            &logger,
                            &terminated_early,
                            &monitor_running,
                            &process_terminator,
                            stats.normal,
                            stats.total,
                        );
                        break;
                    }
                } else if previously_unstable {
                    logger.debug(format!(
                        "Connection recovered to {}/{} normal writes before termination",
                        stats.normal, stats.total
                    ));
                }

                drop(_line_guard);
                thread::sleep(Duration::from_millis(MONITOR_CHECK_INTERVAL_MS));
            }

            logger.info("Sector write monitoring thread has stopped");
        });
    }

    pub fn stop_monitor_thread(&self) {
        self.monitor_running.store(false, Ordering::SeqCst);
        self.monitor_generation.fetch_add(1, Ordering::SeqCst);
    }

    fn terminate_process(
        logger: &Logger,
        terminated_early: &AtomicBool,
        monitor_running: &AtomicBool,
        process_terminator: &ProcessTerminator,
        normal: usize,
        total: usize,
    ) {
        if terminated_early.swap(true, Ordering::SeqCst) {
            return;
        }

        logger.info(format!(
            "Line monitor: only {normal}/{total} normal writes — connection unstable. Restarting..."
        ));
        monitor_running.store(false, Ordering::SeqCst);
        match process_terminator() {
            Ok(()) => logger.info("OpenOCD stopped — will retry automatically."),
            Err(error) => {
                logger.error(format!("Failed to stop the owned OpenOCD process: {error}"));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::Duration;

    #[test]
    fn lines_after_early_termination_do_not_change_progress() {
        let monitor = OperationMonitor::new(Logger::new("OperationMonitorTest"));
        let callback =
            monitor.create_line_monitor(Logger::new("OperationMonitorTest"), Arc::new(|| Ok(())));
        monitor.terminated_early.store(true, Ordering::SeqCst);

        callback("Info : sector 12 took 25 ms");

        assert_eq!(monitor.progress_snapshot().sector_stats.total, 0);
        monitor.stop_monitor_thread();
    }

    #[test]
    fn callbacks_from_a_reset_operation_cannot_change_new_progress() {
        let monitor = OperationMonitor::new(Logger::new("OperationMonitorTest"));
        let stale_callback =
            monitor.create_line_monitor(Logger::new("OperationMonitorTest"), Arc::new(|| Ok(())));

        monitor.reset_counters();
        stale_callback("Info : sector 12 took 25 ms");

        assert_eq!(monitor.progress_snapshot().sector_stats.total, 0);
    }

    #[test]
    fn reset_waits_for_in_flight_line_processing() {
        let monitor = Arc::new(OperationMonitor::new(Logger::new("OperationMonitorTest")));
        let line_guard = monitor.line_processing.lock().unwrap();
        let reset_monitor = Arc::clone(&monitor);
        let (reset_done_tx, reset_done_rx) = mpsc::channel();

        let reset_thread = thread::spawn(move || {
            reset_monitor.reset_counters();
            reset_done_tx.send(()).unwrap();
        });

        assert!(
            reset_done_rx
                .recv_timeout(Duration::from_millis(50))
                .is_err()
        );
        drop(line_guard);
        reset_done_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("reset should finish after line processing releases the lock");
        reset_thread.join().unwrap();
    }

    #[test]
    fn unstable_connection_terminates_only_through_owned_process_callback() {
        use std::sync::atomic::AtomicUsize;

        let monitor = OperationMonitor::new(Logger::new("OperationMonitorTest"));
        let termination_calls = Arc::new(AtomicUsize::new(0));
        let calls_for_callback = Arc::clone(&termination_calls);
        let callback = monitor.create_line_monitor(
            Logger::new("OperationMonitorTest"),
            Arc::new(move || {
                calls_for_callback.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }),
        );

        for sector in 0..4 {
            callback(&format!("Info : sector {sector} took 10 ms"));
        }
        for sector in 4..10 {
            callback(&format!("Info : sector {sector} took 1 ms"));
        }

        let deadline = std::time::Instant::now() + Duration::from_secs(1);
        while !monitor.was_terminated_early() && std::time::Instant::now() < deadline {
            thread::sleep(Duration::from_millis(10));
        }

        assert!(monitor.was_terminated_early());
        assert_eq!(termination_calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            monitor.progress_snapshot().sector_stats,
            SectorStats {
                total: 10,
                normal: 4,
            }
        );
    }

    #[test]
    fn unstable_debounce_requires_consecutive_observations_and_resets_on_recovery() {
        let mut observations = 0;

        assert!(!unstable_connection_confirmed(
            SectorStats {
                total: 10,
                normal: 4,
            },
            &mut observations,
        ));
        assert_eq!(observations, 1);

        assert!(!unstable_connection_confirmed(
            SectorStats {
                total: 11,
                normal: 5,
            },
            &mut observations,
        ));
        assert_eq!(observations, 0);

        let unstable_stats = SectorStats {
            total: 10,
            normal: 4,
        };
        assert!(!unstable_connection_confirmed(
            unstable_stats,
            &mut observations,
        ));
        assert!(unstable_connection_confirmed(
            unstable_stats,
            &mut observations,
        ));
    }

    #[test]
    fn unstable_observation_is_cleared_when_the_next_sector_recovers() {
        use std::sync::atomic::AtomicUsize;

        let monitor = OperationMonitor::new(Logger::new("OperationMonitorTest"));
        let termination_calls = Arc::new(AtomicUsize::new(0));
        let calls_for_callback = Arc::clone(&termination_calls);
        let callback = monitor.create_line_monitor(
            Logger::new("OperationMonitorTest"),
            Arc::new(move || {
                calls_for_callback.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }),
        );

        for sector in 0..4 {
            callback(&format!("Info : sector {sector} took 10 ms"));
        }
        for sector in 4..10 {
            callback(&format!("Info : sector {sector} took 1 ms"));
        }
        callback("Info : sector 10 took 10 ms");

        thread::sleep(Duration::from_millis(
            MONITOR_CHECK_INTERVAL_MS * (UNSTABLE_CONFIRMATION_OBSERVATIONS as u64 + 1),
        ));

        assert!(!monitor.was_terminated_early());
        assert_eq!(termination_calls.load(Ordering::SeqCst), 0);
        assert_eq!(
            monitor.progress_snapshot().sector_stats,
            crate::device_programmer::operation::SectorStats {
                total: 11,
                normal: 5,
            }
        );
        monitor.stop_monitor_thread();
    }
}
