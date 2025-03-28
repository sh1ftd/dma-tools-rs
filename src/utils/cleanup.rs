use crate::device_programmer::{CREATE_NO_WINDOW, DNA_OUTPUT_FILE, TEMP_FIRMWARE_FILE};
use crate::utils::logger::Logger;
use std::fs;
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::Command;

const OPENOCD_PROCESSES: [&str; 2] = ["openocd.exe", "openocd-347.exe"];

const CLEANUP_FILES: &[&str] = &[TEMP_FIRMWARE_FILE, DNA_OUTPUT_FILE];

pub fn perform_startup_cleanup(logger: &Logger) {
    logger.debug("Performing startup cleanup...");

    terminate_lingering_processes(logger);

    cleanup_temp_files(logger);

    logger.debug("Startup cleanup completed");
}

fn terminate_lingering_processes(logger: &Logger) {
    logger.debug("Checking for lingering processes...");

    for process_name in OPENOCD_PROCESSES.iter() {
        logger.debug(format!("Attempting to terminate {}", process_name));

        let result = Command::new("taskkill")
            .args(["/F", "/IM", process_name])
            .creation_flags(CREATE_NO_WINDOW) // Hide the window
            .output();

        match result {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if !stdout.contains("ERROR") {
                    logger.debug(format!("Terminated lingering {} processes", process_name));
                }
            }
            Err(e) => {
                logger.debug(format!(
                    "No {} processes to terminate or error: {}",
                    process_name, e
                ));
            }
        }
    }
}

fn cleanup_temp_files(logger: &Logger) {
    logger.debug("Cleaning up temporary files...");

    for file in CLEANUP_FILES {
        if Path::new(file).exists() {
            match fs::remove_file(file) {
                Ok(_) => logger.debug(format!("Removed temporary file: {}", file)),
                Err(e) => logger.debug(format!("Failed to remove {}: {}", file, e)),
            }
        }
    }
}
