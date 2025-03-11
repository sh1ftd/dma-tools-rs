use crate::utils::logger::Logger;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

// Monitoring thresholds
const QUICK_WRITE_THRESHOLD_MS: u32 = 10; // Threshold to consider a sector write "quick"
const QUICK_WRITE_MAX_COUNT: usize = 35; // Maximum allowed quick writes before termination

pub struct OperationMonitor {
    quick_write_count: Arc<AtomicUsize>,
    total_sector_count: Arc<AtomicUsize>,
    auto_terminate_enabled: Arc<AtomicBool>,
    terminated_early: Arc<AtomicBool>,
}

impl OperationMonitor {
    pub fn new() -> Self {
        Self {
            quick_write_count: Arc::new(AtomicUsize::new(0)),
            total_sector_count: Arc::new(AtomicUsize::new(0)),
            auto_terminate_enabled: Arc::new(AtomicBool::new(true)),
            terminated_early: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn reset_counters(&self) {
        self.quick_write_count.store(0, Ordering::SeqCst);
        self.total_sector_count.store(0, Ordering::SeqCst);
        self.terminated_early.store(false, Ordering::SeqCst);
    }

    pub fn was_terminated_early(&self) -> bool {
        self.terminated_early.load(Ordering::SeqCst)
    }

    pub fn create_line_monitor(
        &self,
        logger: Logger,
        child_process: Arc<Mutex<Option<u32>>>,
    ) -> Box<dyn Fn(&str) + Send + 'static> {
        let quick_write_count = Arc::clone(&self.quick_write_count);
        let total_sector_count = Arc::clone(&self.total_sector_count);
        let auto_terminate = Arc::clone(&self.auto_terminate_enabled);
        let terminated_early = Arc::clone(&self.terminated_early);

        Box::new(move |line: &str| {
            Self::process_sector_writes(
                line,
                &logger,
                &quick_write_count,
                &total_sector_count,
                &auto_terminate,
                &terminated_early,
                &child_process,
            );
        })
    }

    fn process_sector_writes(
        line: &str,
        logger: &Logger,
        quick_write_count: &Arc<AtomicUsize>,
        total_sector_count: &Arc<AtomicUsize>,
        auto_terminate: &Arc<AtomicBool>,
        terminated_early: &Arc<AtomicBool>,
        child_process: &Arc<Mutex<Option<u32>>>,
    ) {
        // Look for lines like "[ERROR] Info : sector 25 took 1 ms"
        if line.contains("Info :") && line.contains("sector") && line.contains("took") {
            logger.info(format!("Detected sector write line: {}", line));

            // Extract the time value using a more robust method
            if let Some(time_str) = line
                .split("took")
                .nth(1)
                .and_then(|s| s.split_whitespace().next())
            {
                if let Ok(write_time) = time_str.parse::<u32>() {
                    logger.info(format!("Sector write time: {}ms", write_time));
                    total_sector_count.fetch_add(1, Ordering::SeqCst);

                    if write_time <= QUICK_WRITE_THRESHOLD_MS {
                        quick_write_count.fetch_add(1, Ordering::SeqCst);
                    }

                    let quick = quick_write_count.load(Ordering::SeqCst);
                    let total = total_sector_count.load(Ordering::SeqCst);
                    logger.info(format!("Quick writes: {}, Total sectors: {}", quick, total));

                    // Terminate if too many quick writes (indicates hardware issue)
                    if auto_terminate.load(Ordering::SeqCst) && quick > QUICK_WRITE_MAX_COUNT {
                        logger.warning("Detected too many quick sector writes (likely hardware issue). Terminating process early.");

                        if let Some(pid) = child_process.lock().unwrap().take() {
                            #[cfg(windows)]
                            {
                                use std::process::Command;
                                let _ = Command::new("taskkill")
                                    .args(["/F", "/PID", &pid.to_string()])
                                    .output();

                                logger.warning(format!("Terminating process with PID {}", pid));
                            }

                            #[cfg(not(windows))]
                            {
                                use std::process::Command;
                                let _ =
                                    Command::new("kill").arg("-9").arg(pid.to_string()).status();
                            }

                            terminated_early.store(true, Ordering::SeqCst);
                            logger.error("Operation terminated early due to connection issues.");
                        }
                    }
                }
            }
        }
    }

    #[allow(dead_code)]
    pub fn set_auto_terminate(&self, enabled: bool) {
        self.auto_terminate_enabled.store(enabled, Ordering::SeqCst);
    }
}
