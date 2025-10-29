use crate::device_programmer::TEMP_FIRMWARE_FILE;
use crate::utils::logger::Logger;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

pub struct FirmwareManager {
    firmware_files: Vec<PathBuf>,
    selected_index: Option<usize>,
    scan_count: usize,
    logger: Logger,
    cleanup_enabled: bool,
}

impl FirmwareManager {
    pub fn new() -> Self {
        Self {
            firmware_files: Vec::new(),
            selected_index: None,
            scan_count: 0,
            logger: Logger::new("FirmwareDiscovery"),
            cleanup_enabled: false,
        }
    }

    pub fn scan_firmware_files(&mut self) {
        if let Err(e) = fs::remove_file(TEMP_FIRMWARE_FILE) {
            self.logger.info(format!(
                "Note: Could not remove previous temp firmware file: {e}"
            ));
        }

        self.firmware_files.clear();

        let exe_path = env::current_exe().unwrap_or_else(|_| PathBuf::from("."));
        let exe_dir = exe_path.parent().unwrap_or_else(|| Path::new("."));

        let search_dirs = self.create_search_dirs(exe_dir);

        #[cfg(debug_assertions)]
        self.debug_print_search_dirs(&search_dirs);

        self.collect_firmware_files(&search_dirs);

        self.deduplicate_firmware_files();

        self.scan_count += 1;

        // Auto-select if only one file is found
        if self.firmware_files.len() == 1 {
            self.selected_index = Some(0);
        }

        #[cfg(debug_assertions)]
        self.debug_print_results();
    }

    /// Returns a slice of all found firmware files
    pub fn get_firmware_files(&self) -> &[PathBuf] {
        &self.firmware_files
    }

    /// Selects a firmware file by index and returns it if valid
    pub fn select_firmware(&mut self, index: usize) -> Option<PathBuf> {
        if index < self.firmware_files.len() {
            self.selected_index = Some(index);
            Some(self.firmware_files[index].clone())
        } else {
            None
        }
    }

    /// Returns the currently selected firmware file, if any
    pub fn get_selected_firmware(&self) -> Option<&PathBuf> {
        self.selected_index.map(|i| &self.firmware_files[i])
    }

    /// Returns the number of scans performed
    pub fn get_scan_count(&self) -> usize {
        self.scan_count
    }

    fn create_search_dirs(&self, exe_dir: &Path) -> Vec<PathBuf> {
        let mut search_dirs = vec![
            PathBuf::from("."),         // Current directory
            exe_dir.to_path_buf(),      // Executable directory
            PathBuf::from("resources"), // Resources directory for cargo run
            PathBuf::from("bin"),       // Possible bin directory
            PathBuf::from("firmware"),  // Possible firmware directory
            PathBuf::from("fw"),        // Possible openocd directory
        ];

        if cfg!(debug_assertions) {
            search_dirs.push(PathBuf::from("target/debug"));
            search_dirs.push(exe_dir.join("../"));
        }

        search_dirs
    }

    fn collect_firmware_files(&mut self, search_dirs: &[PathBuf]) {
        for dir in search_dirs {
            if let Ok(entries) = fs::read_dir(dir) {
                for entry in entries.filter_map(Result::ok) {
                    let path = entry.path();
                    if path.is_file() && path.extension().is_some_and(|ext| ext == "bin") {
                        #[cfg(debug_assertions)]
                        println!("Found firmware: {}", path.display());

                        self.firmware_files.push(path);
                    }
                }
            }
        }
    }

    fn deduplicate_firmware_files(&mut self) {
        // Sort by filename for consistent ordering and deduplication
        self.firmware_files.sort_by(|a, b| {
            a.file_name()
                .unwrap_or_default()
                .cmp(b.file_name().unwrap_or_default())
        });

        // Remove duplicates based on filename
        self.firmware_files.dedup_by(|a, b| {
            a.file_name().unwrap_or_default() == b.file_name().unwrap_or_default()
        });
    }

    #[cfg(debug_assertions)]
    fn debug_print_search_dirs(&self, search_dirs: &[PathBuf]) {
        println!("Searching for firmware in:");
        for dir in search_dirs {
            if let Ok(canonical) = fs::canonicalize(dir) {
                println!("  - {}", canonical.display());
            } else {
                println!("  - {}", dir.display());
            }
        }
    }

    #[cfg(debug_assertions)]
    fn debug_print_results(&self) {
        println!("Found {} unique firmware files", self.firmware_files.len());
        for file in &self.firmware_files {
            println!("  - {}", file.display());
        }
    }

    pub fn get_cleanup_enabled(&self) -> bool {
        self.cleanup_enabled
    }

    pub fn set_cleanup_enabled(&mut self, enabled: bool) {
        self.cleanup_enabled = enabled;
    }
}
