use crate::utils::logger::Logger;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;

// Monitoring thresholds
const QUICK_WRITE_THRESHOLD_MS: u32 = 10; // Threshold to consider a sector write "quick"
const QUICK_WRITE_MAX_COUNT: usize = 5; // Maximum allowed quick writes before termination
const MONITOR_CHECK_INTERVAL_MS: u64 = 50; // Reduced from 100ms for faster response time

struct SectorWriteContext<'a> {
    logger: &'a Logger,
    quick_write_count: &'a Arc<AtomicUsize>,
    total_sector_count: &'a Arc<AtomicUsize>,
    auto_terminate: &'a Arc<AtomicBool>,
    terminated_early: &'a Arc<AtomicBool>,
    monitor_running: &'a Arc<AtomicBool>,
}

pub struct OperationMonitor {
    quick_write_count: Arc<AtomicUsize>,
    total_sector_count: Arc<AtomicUsize>,
    auto_terminate_enabled: Arc<AtomicBool>,
    terminated_early: Arc<AtomicBool>,
    monitor_running: Arc<AtomicBool>,
}

impl OperationMonitor {
    pub fn new() -> Self {
        Self {
            quick_write_count: Arc::new(AtomicUsize::new(0)),
            total_sector_count: Arc::new(AtomicUsize::new(0)),
            auto_terminate_enabled: Arc::new(AtomicBool::new(true)),
            terminated_early: Arc::new(AtomicBool::new(false)),
            monitor_running: Arc::new(AtomicBool::new(false)),
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

    pub fn create_line_monitor(&self, logger: Logger) -> Box<dyn Fn(&str) + Send + Sync + 'static> {
        let quick_write_count = Arc::clone(&self.quick_write_count);
        let total_sector_count = Arc::clone(&self.total_sector_count);
        let auto_terminate = Arc::clone(&self.auto_terminate_enabled);
        let terminated_early = Arc::clone(&self.terminated_early);
        let monitor_running = Arc::clone(&self.monitor_running);

        // Start a dedicated monitoring thread
        self.start_monitor_thread(logger.clone());

        Box::new(move |line: &str| {
            // Only log detailed info in debug mode
            logger.debug(format!("Monitor processing: {}", line));

            // Create the context with all required references
            let context = SectorWriteContext {
                logger: &logger,
                quick_write_count: &quick_write_count,
                total_sector_count: &total_sector_count,
                auto_terminate: &auto_terminate,
                terminated_early: &terminated_early,
                monitor_running: &monitor_running,
            };

            Self::process_sector_writes(line, &context);
        })
    }

    fn start_monitor_thread(&self, logger: Logger) {
        let quick_write_count = Arc::clone(&self.quick_write_count);
        let total_sector_count = Arc::clone(&self.total_sector_count);
        let auto_terminate = Arc::clone(&self.auto_terminate_enabled);
        let terminated_early = Arc::clone(&self.terminated_early);
        let monitor_running = Arc::clone(&self.monitor_running);

        // Only start the thread if it's not already running
        if !monitor_running.load(Ordering::SeqCst) {
            monitor_running.store(true, Ordering::SeqCst);

            thread::spawn(move || {
                logger.info("Starting real-time sector write monitoring thread".to_string());

                // Track the previous count to detect increments
                let mut prev_quick_count = 0;
                let mut prev_total_count = 0;
                let mut consecutive_checks_over_threshold = 0;
                let mut check_count = 0;

                while monitor_running.load(Ordering::SeqCst) {
                    let quick = quick_write_count.load(Ordering::SeqCst);
                    let total = total_sector_count.load(Ordering::SeqCst);

                    // Always log a status update every few checks
                    check_count += 1;
                    if check_count % 10 == 0
                        || quick != prev_quick_count
                        || total != prev_total_count
                    {
                        logger.debug(format!(
                            "[Monitor Thread] Check #{}: Quick writes: {}/{} (threshold: {})",
                            check_count, quick, total, QUICK_WRITE_MAX_COUNT
                        ));
                        prev_quick_count = quick;
                        prev_total_count = total;
                    }

                    // Check if we need to terminate due to too many quick writes
                    if auto_terminate.load(Ordering::SeqCst) && quick > QUICK_WRITE_MAX_COUNT {
                        consecutive_checks_over_threshold += 1;

                        logger.warning(format!(
                            "Quick write threshold exceeded: {}/{} (consecutive detections: {}/2)",
                            quick, QUICK_WRITE_MAX_COUNT, consecutive_checks_over_threshold
                        ));

                        // Add an extra check to ensure we've been consistently over threshold
                        if consecutive_checks_over_threshold >= 2 {
                            logger.warning(format!(
                                "Monitor thread detected too many quick sector writes: {}/{}. Terminating process early.",
                                quick, total
                            ));

                            // Use the same process name termination we added to terminate_process
                            for process_name in &["openocd.exe", "openocd-347.exe"] {
                                logger.warning(format!("Terminating {}", process_name));

                                // ... (use the termination code from terminate_process)
                            }

                            terminated_early.store(true, Ordering::SeqCst);
                            logger.error("Operation terminated early due to connection issues.");
                            break;
                        }
                    } else if consecutive_checks_over_threshold > 0 {
                        logger.info(format!(
                            "Quick write count {}/{} now below threshold, resetting consecutive counter", 
                            quick, QUICK_WRITE_MAX_COUNT
                        ));
                        consecutive_checks_over_threshold = 0;
                    }

                    // Sleep before checking again
                    thread::sleep(Duration::from_millis(MONITOR_CHECK_INTERVAL_MS));
                }

                logger.info("Sector write monitoring thread has stopped".to_string());
            });
        }
    }

    pub fn stop_monitor_thread(&self) {
        self.monitor_running.store(false, Ordering::SeqCst);
    }

    fn process_sector_writes(line: &str, ctx: &SectorWriteContext) {
        // First check if we're already terminated
        if ctx.terminated_early.load(Ordering::SeqCst) {
            ctx.logger
                .debug("Skipping sector write - process already terminated");
            return;
        }

        // Add a visible debug print for every line
        ctx.logger.debug(format!("Processing line: {}", line));

        // Original format attempt
        if line.contains("Info :") && line.contains("sector") && line.contains("took") {
            ctx.logger.debug("Matched pattern: Info : sector took");
            Self::handle_sector_write(line, ctx);
        }
        // Check for [ERROR] prefix format
        else if line.contains("[ERROR]")
            && line.contains("Info :")
            && line.contains("sector")
            && line.contains("took")
        {
            ctx.logger
                .debug("Matched pattern: [ERROR] Info : sector took");
            let line_without_prefix = line.split("[ERROR]").nth(1).unwrap_or(line).trim();
            Self::handle_sector_write(line_without_prefix, ctx);
        }
        // Last resort - just look for the essential parts
        else if line.contains("sector") && line.contains("took") && line.contains("ms") {
            ctx.logger.debug("Matched pattern: sector took ms");
            Self::handle_sector_write(line, ctx);
        }
    }

    // Separate function to handle a sector write line once detected
    fn handle_sector_write(line: &str, ctx: &SectorWriteContext) {
        // Force increment total count
        let prev_total = ctx.total_sector_count.fetch_add(1, Ordering::SeqCst);
        ctx.logger.debug(format!(
            "Total sector count: {} -> {}",
            prev_total,
            prev_total + 1
        ));

        // Extract the time value more reliably - all debug level
        ctx.logger.debug(format!("Extracting time from: {}", line));

        if let Some(time_part) = line.split("took").nth(1) {
            ctx.logger.debug(format!("After 'took': '{}'", time_part));

            if let Some(time_str) = time_part.split_whitespace().next() {
                ctx.logger
                    .debug(format!("Extracted time value: '{}'", time_str));

                if let Ok(write_time) = time_str.parse::<u32>() {
                    let is_quick = write_time <= QUICK_WRITE_THRESHOLD_MS;

                    // Only log quick writes as warnings
                    if is_quick {
                        let previous = ctx.quick_write_count.fetch_add(1, Ordering::SeqCst);
                        ctx.logger.warning(format!(
                            "Quick write detected! Count increased from {} to {}",
                            previous,
                            previous + 1
                        ));
                    } else {
                        ctx.logger
                            .debug(format!("Normal write time: {}ms", write_time));
                    }

                    // Keep the stats as warnings
                    let quick = ctx.quick_write_count.load(Ordering::SeqCst);
                    let total = ctx.total_sector_count.load(Ordering::SeqCst);
                    ctx.logger.debug(format!(
                        "Current stats - Quick writes: {}/{} (threshold: {})",
                        quick, total, QUICK_WRITE_MAX_COUNT
                    ));

                    // Check for termination
                    if ctx.auto_terminate.load(Ordering::SeqCst) && quick > QUICK_WRITE_MAX_COUNT {
                        Self::terminate_process(ctx, quick, total);
                    }
                } else {
                    ctx.logger
                        .debug(format!("Failed to parse '{}' as u32", time_str));
                }
            }
        } else {
            ctx.logger.debug("Failed to split on 'took'");
        }
    }

    // Split termination logic into a function
    fn terminate_process(ctx: &SectorWriteContext, quick: usize, total: usize) {
        ctx.logger.warning(format!(
            "LINE MONITOR: Too many quick writes detected: {}/{}. Terminating OpenOCD process.",
            quick, total
        ));

        use std::os::windows::process::CommandExt;
        use std::process::Command;

        for process_name in &["openocd.exe", "openocd-347.exe"] {
            ctx.logger.warning(format!("Terminating {}", process_name));

            let result = Command::new("taskkill")
                .args(["/F", "/IM", process_name])
                .creation_flags(crate::device_programmer::CREATE_NO_WINDOW)
                .output();

            match result {
                Ok(output) => {
                    let output_str = String::from_utf8_lossy(&output.stdout);
                    ctx.logger.warning(format!(
                        "Termination result for {}: {}",
                        process_name, output_str
                    ));

                    // If successful, set the terminated flag
                    if output.status.success() || output_str.contains("SUCCESS") {
                        ctx.terminated_early.store(true, Ordering::SeqCst);
                        ctx.logger
                            .error("Operation terminated early - too many quick writes detected.");

                        // Stop the monitor thread
                        ctx.monitor_running.store(false, Ordering::SeqCst);
                        ctx.logger
                            .warning("TERMINATION SUCCESSFUL - STOPPING FURTHER PROCESSING");

                        // No need to continue trying other process names
                        break;
                    }
                }
                Err(e) => {
                    ctx.logger
                        .error(format!("Failed to terminate {}: {}", process_name, e));
                }
            }
        }
    }
}
