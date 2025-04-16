use crate::device_programmer::CREATE_NO_WINDOW;
use crate::utils::logger::Logger;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;

// Monitoring thresholds
const NORMAL_WRITE_THRESHOLD_MS: u32 = 10; // Threshold to consider a sector write "normal"
const MIN_NORMAL_WRITES_REQUIRED: usize = 5; // Minimum required normal writes before termination
const MONITOR_CHECK_INTERVAL_MS: u64 = 50; // Reduced from 100ms for faster response time

struct SectorWriteContext<'a> {
    logger: &'a Logger,
    normal_write_count: &'a Arc<AtomicUsize>,
    total_sector_count: &'a Arc<AtomicUsize>,
    auto_terminate: &'a Arc<AtomicBool>,
    terminated_early: &'a Arc<AtomicBool>,
    monitor_running: &'a Arc<AtomicBool>,
}

pub struct OperationMonitor {
    normal_write_count: Arc<AtomicUsize>,
    total_sector_count: Arc<AtomicUsize>,
    auto_terminate_enabled: Arc<AtomicBool>,
    terminated_early: Arc<AtomicBool>,
    monitor_running: Arc<AtomicBool>,
    logger: Logger,
}

impl OperationMonitor {
    pub fn new(logger: Logger) -> Self {
        Self {
            normal_write_count: Arc::new(AtomicUsize::new(0)),
            total_sector_count: Arc::new(AtomicUsize::new(0)),
            auto_terminate_enabled: Arc::new(AtomicBool::new(true)),
            terminated_early: Arc::new(AtomicBool::new(false)),
            monitor_running: Arc::new(AtomicBool::new(false)),
            logger,
        }
    }

    pub fn reset_counters(&self) {
        self.stop_monitor_thread();

        self.normal_write_count.store(0, Ordering::SeqCst);
        self.total_sector_count.store(0, Ordering::SeqCst);
        self.terminated_early.store(false, Ordering::SeqCst);
        self.monitor_running.store(false, Ordering::SeqCst);

        self.logger
            .debug("OperationMonitor: All counters and flags have been reset");
    }

    pub fn was_terminated_early(&self) -> bool {
        self.terminated_early.load(Ordering::SeqCst)
    }

    pub fn create_line_monitor(&self, logger: Logger) -> Box<dyn Fn(&str) + Send + Sync + 'static> {
        let normal_write_count = Arc::clone(&self.normal_write_count);
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
                normal_write_count: &normal_write_count,
                total_sector_count: &total_sector_count,
                auto_terminate: &auto_terminate,
                terminated_early: &terminated_early,
                monitor_running: &monitor_running,
            };

            Self::process_sector_writes(line, &context);
        })
    }

    fn start_monitor_thread(&self, logger: Logger) {
        let normal_write_count = Arc::clone(&self.normal_write_count);
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
                let mut prev_normal_count = 0;
                let mut prev_total_count = 0;
                let mut consecutive_checks_over_threshold = 0;
                let mut check_count = 0;

                while monitor_running.load(Ordering::SeqCst) {
                    let normal = normal_write_count.load(Ordering::SeqCst);
                    let total = total_sector_count.load(Ordering::SeqCst);

                    // Always log a status update every few checks
                    check_count += 1;
                    if check_count % 10 == 0
                        || normal != prev_normal_count
                        || total != prev_total_count
                    {
                        logger.debug(format!(
                            "[Monitor Thread] Check #{}: Normal writes: {}/{} (threshold: {})",
                            check_count, normal, total, MIN_NORMAL_WRITES_REQUIRED
                        ));
                        prev_normal_count = normal;
                        prev_total_count = total;
                    }

                    // Check if we need to terminate due to too few normal writes
                    if auto_terminate.load(Ordering::SeqCst) && normal < MIN_NORMAL_WRITES_REQUIRED
                    {
                        consecutive_checks_over_threshold += 1;

                        logger.debug(format!(
                            "Normal write count below minimum: {}/{} (consecutive detections: {}/2)",
                            normal, MIN_NORMAL_WRITES_REQUIRED, consecutive_checks_over_threshold
                        ));

                        // Add an extra check to ensure we've been consistently under threshold
                        if consecutive_checks_over_threshold >= 2 {
                            logger.warning(format!(
                                "Monitor thread detected too few normal sector writes: {}/{}. Terminating process early.",
                                normal, total
                            ));

                            // Use the same process name termination we added to terminate_process
                            for process_name in &["openocd.exe", "openocd-347.exe"] {
                                logger.debug(format!("Terminating {}", process_name));

                                // ... (use the termination code from terminate_process)
                            }

                            terminated_early.store(true, Ordering::SeqCst);
                            logger.error("Operation terminated early due to connection issues.");
                            break;
                        }
                    } else if consecutive_checks_over_threshold > 0 {
                        logger.info(format!(
                            "Normal write count {}/{} now above minimum, resetting consecutive counter", 
                            normal, MIN_NORMAL_WRITES_REQUIRED
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

    fn process_sector_writes(line: &str, ctx: &SectorWriteContext<'_>) {
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
    fn handle_sector_write(line: &str, ctx: &SectorWriteContext<'_>) {
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
                    let is_normal = write_time >= NORMAL_WRITE_THRESHOLD_MS;

                    if is_normal {
                        let previous = ctx.normal_write_count.fetch_add(1, Ordering::SeqCst);
                        ctx.logger.debug(format!(
                            "Normal write detected! Count increased from {} to {}",
                            previous,
                            previous + 1
                        ));
                    } else {
                        ctx.logger
                            .debug(format!("Quick write time: {}ms", write_time));
                    }

                    let normal = ctx.normal_write_count.load(Ordering::SeqCst);
                    let total = ctx.total_sector_count.load(Ordering::SeqCst);
                    ctx.logger.debug(format!(
                        "Current stats - Normal writes: {}/{} (threshold: {})",
                        normal, total, MIN_NORMAL_WRITES_REQUIRED
                    ));

                    // Check for termination
                    if ctx.auto_terminate.load(Ordering::SeqCst)
                        && normal < MIN_NORMAL_WRITES_REQUIRED
                    {
                        Self::terminate_process(ctx, normal, total);
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
    fn terminate_process(ctx: &SectorWriteContext<'_>, normal: usize, total: usize) {
        ctx.logger.debug(format!(
            "LINE MONITOR: Too few normal writes detected: {}/{}. Terminating OpenOCD process.",
            normal, total
        ));

        use std::os::windows::process::CommandExt;
        use std::process::Command;

        for process_name in &["openocd.exe", "openocd-347.exe"] {
            ctx.logger.debug(format!("Terminating {}", process_name));

            let result = Command::new("taskkill")
                .args(["/F", "/IM", process_name])
                .creation_flags(CREATE_NO_WINDOW)
                .output();

            match result {
                Ok(output) => {
                    let output_str = String::from_utf8_lossy(&output.stdout);
                    ctx.logger.debug(format!(
                        "Termination result for {}: {}",
                        process_name, output_str
                    ));

                    // If successful, set the terminated flag
                    if output.status.success() || output_str.contains("SUCCESS") {
                        ctx.terminated_early.store(true, Ordering::SeqCst);
                        ctx.logger
                            .error("Operation terminated early - too few normal writes detected.");

                        // Stop the monitor thread
                        ctx.monitor_running.store(false, Ordering::SeqCst);
                        ctx.logger
                            .debug("TERMINATION SUCCESSFUL - STOPPING FURTHER PROCESSING");

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
