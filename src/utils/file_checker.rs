use std::env;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant};

pub const SUCCESS_TRANSITION_DELAY: u64 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileCheckResult {
    pub missing_files: Vec<String>,
    pub error_count: usize,
}

/// Status of the file checking process
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckStatus {
    NotStarted,
    Checking(String), // Currently checking this file
    Complete(FileCheckResult),
    Success(Instant),  // When successful check completed
    ReadyToTransition, // After countdown timer completes
}

pub struct FileChecker {
    pub status: Arc<Mutex<CheckStatus>>,
}

impl FileChecker {
    pub fn new() -> Self {
        Self {
            status: Arc::new(Mutex::new(CheckStatus::NotStarted)),
        }
    }

    pub fn get_status(&self) -> CheckStatus {
        self.status.lock().unwrap().clone()
    }

    pub fn get_status_mut(&mut self) -> MutexGuard<'_, CheckStatus> {
        self.status.lock().unwrap()
    }

    /// Starts file checking in a background thread to keep UI responsive
    pub fn start_check(&self) {
        let status = Arc::clone(&self.status);

        thread::spawn(move || {
            log_execution_context();

            let result = perform_file_check(&status);

            *status.lock().unwrap() = CheckStatus::Complete(result);
        });
    }
}

/// Logs information about execution context for debugging purposes
fn log_execution_context() {
    if let Ok(dir) = env::current_dir() {
        println!("Working directory: {}", dir.display());
    }

    if let Ok(exe) = env::current_exe() {
        println!("Executable path: {}", exe.display());
    }
}

fn perform_file_check(status: &Arc<Mutex<CheckStatus>>) -> FileCheckResult {
    let mut missing_files = Vec::new();

    let required_files = [
        // Executables
        "OpenOCD/openocd-347.exe",
        "OpenOCD/openocd.exe",
        // Libraries
        "OpenOCD/cygwin1.dll",
        "OpenOCD/cygusb-1.0.dll",
        "OpenOCD/libftdi1.dll",
        "OpenOCD/libgcc_s_sjlj-1.dll",
        "OpenOCD/libhidapi-0.dll",
        "OpenOCD/libusb-1.0.dll",
        "OpenOCD/libwinpthread-1.dll",
        // Configuration
        "OpenOCD/libftdi1-config",
        // Bitstream files
        "OpenOCD/bit/bscan_spi_xc7a35t.bit",
        "OpenOCD/bit/bscan_spi_xc7a75t.bit",
        "OpenOCD/bit/bscan_spi_xc7a100t.bit",
        // Flash configuration
        "OpenOCD/flash/xc7a35T.cfg",
        "OpenOCD/flash/xc7a75T.cfg",
        "OpenOCD/flash/xc7a100T.cfg",
        "OpenOCD/flash/xc7a35T_rs232.cfg",
        "OpenOCD/flash/xc7a75T_rs232.cfg",
        "OpenOCD/flash/xc7a100T_rs232.cfg",
        // DNA configuration
        "OpenOCD/DNA/init_347.cfg",
        "OpenOCD/DNA/init_232_35t.cfg",
        "OpenOCD/DNA/init_232_75t.cfg",
        "OpenOCD/DNA/init_232_100t.cfg",
    ];

    // Check each file with a small delay to show progress
    for file in &required_files {
        // Update status to show what we're currently checking
        *status.lock().unwrap() = CheckStatus::Checking(file.to_string());

        // Allow UI to update and show progress
        thread::sleep(Duration::from_millis(15));

        // Check if file exists in any of the possible locations
        if !check_file_exists(file) {
            missing_files.push(file.to_string());
        }
    }

    FileCheckResult {
        error_count: missing_files.len(),
        missing_files,
    }
}

/// Helper function to check multiple possible locations for a file
fn check_file_exists(file_path: &str) -> bool {
    let mut base_paths = vec![PathBuf::from(".")];

    #[cfg(debug_assertions)]
    {
        base_paths.push(PathBuf::from("target/debug"));
        base_paths.push(PathBuf::from(".."));

        println!("Checking for file: {file_path}");
        for path in &base_paths {
            let full_path = path.join(file_path);
            println!("Looking in: {}", full_path.display());
        }
    }

    #[cfg(not(debug_assertions))]
    {
        if let Ok(exe) = env::current_exe() {
            if let Some(dir) = exe.parent() {
                base_paths.push(dir.to_path_buf());
            }
        }
    }

    for base_path in &base_paths {
        let full_path = base_path.join(file_path);
        if full_path.exists() {
            #[cfg(debug_assertions)]
            println!("✓ Found: {}", full_path.display());
            return true;
        }
    }

    #[cfg(debug_assertions)]
    println!("✗ Not found: {file_path}");

    false
}
