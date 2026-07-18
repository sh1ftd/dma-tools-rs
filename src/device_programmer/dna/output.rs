use super::parser::{DnaParseError, parse_dna_output};
use crate::device_programmer::DnaInfo;
use crate::utils::logger::Logger;
use std::fmt;
use std::fs;
use std::io::{self, Read};
use std::path::Path;
use std::thread;
use std::time::{Duration, SystemTime};

pub const MIN_VALID_DNA_FILE_SIZE: u64 = 10;
const MAX_VALID_DNA_FILE_SIZE: u64 = 1024 * 1024;
// FAT-family and some network filesystems expose coarse modification timestamps. Cleanup is
// already strict, so this tolerance only prevents a freshly recreated file from being rejected
// because its timestamp was rounded down by the filesystem.
const OUTPUT_TIMESTAMP_TOLERANCE: Duration = Duration::from_secs(2);

/// Identifies the point after which output belongs to the current DNA operation.
///
/// The output path is fixed by the bundled OpenOCD configuration, so a successful
/// cleanup plus this timestamp is the strongest provenance check available without
/// changing those commands.
#[derive(Debug, Clone, Copy)]
pub struct OutputGeneration {
    prepared_at: SystemTime,
}

#[derive(Debug)]
pub enum OutputWaitError {
    Unavailable,
    Stale,
    Read(io::Error),
    Malformed(DnaParseError),
}

impl fmt::Display for OutputWaitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unavailable => {
                formatter.write_str("DNA output file is unavailable or incomplete")
            }
            Self::Stale => formatter.write_str("DNA output file predates the current operation"),
            Self::Read(error) => write!(formatter, "failed to read DNA output: {error}"),
            Self::Malformed(error) => write!(formatter, "invalid DNA output: {error}"),
        }
    }
}

/// Removes prior output and creates a freshness boundary for one DNA operation.
///
/// Any cleanup error is returned to the caller. Continuing after such an error
/// could make a previous device's DNA look like the current result.
pub fn prepare_output_file(path: &Path, logger: &Logger) -> io::Result<OutputGeneration> {
    match fs::remove_file(path) {
        Ok(()) => logger.debug("Successfully removed previous DNA output file"),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            logger.debug("No previous DNA output file found (which is expected)");
        }
        Err(error) => {
            logger.error(format!(
                "Could not remove previous DNA output file: {error}"
            ));
            return Err(error);
        }
    }

    Ok(OutputGeneration {
        prepared_at: SystemTime::now(),
    })
}

/// Waits for output from the current operation and parses it atomically.
///
/// Incomplete, temporarily unreadable, and malformed files are all retried. This
/// covers the period where OpenOCD has created the file but has not finished
/// flushing its final DNA line.
pub fn wait_for_parsed_output(
    path: &Path,
    generation: OutputGeneration,
    attempts: usize,
    retry_wait: Duration,
    logger: &Logger,
) -> Result<DnaInfo, OutputWaitError> {
    let mut last_error = OutputWaitError::Unavailable;

    for attempt in 1..=attempts {
        match inspect_and_parse(path, generation) {
            Ok(info) => {
                logger.debug(format!("Found valid DNA output at {}", path.display()));
                return Ok(info);
            }
            Err(error) => {
                logger.debug(format!(
                    "DNA output attempt {attempt}/{attempts} was not ready: {error}"
                ));
                last_error = error;
            }
        }

        if attempt < attempts {
            thread::sleep(retry_wait);
        }
    }

    Err(last_error)
}

fn inspect_and_parse(
    path: &Path,
    generation: OutputGeneration,
) -> Result<DnaInfo, OutputWaitError> {
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Err(OutputWaitError::Unavailable);
        }
        Err(error) => return Err(OutputWaitError::Read(error)),
    };

    if !metadata.is_file() || metadata.len() < MIN_VALID_DNA_FILE_SIZE {
        return Err(OutputWaitError::Unavailable);
    }
    if metadata.len() > MAX_VALID_DNA_FILE_SIZE {
        return Err(oversized_output_error(metadata.len()));
    }

    let modified = metadata.modified().map_err(OutputWaitError::Read)?;
    if modified
        .checked_add(OUTPUT_TIMESTAMP_TOLERANCE)
        .is_some_and(|latest_possible_write| latest_possible_write < generation.prepared_at)
    {
        return Err(OutputWaitError::Stale);
    }

    // Bound the read as well as the metadata check so a file replaced or grown
    // between those operations still cannot trigger an unbounded allocation.
    let file = fs::File::open(path).map_err(OutputWaitError::Read)?;
    let mut contents = String::with_capacity(metadata.len() as usize);
    let bytes_read = file
        .take(MAX_VALID_DNA_FILE_SIZE + 1)
        .read_to_string(&mut contents)
        .map_err(OutputWaitError::Read)?;
    if bytes_read as u64 > MAX_VALID_DNA_FILE_SIZE {
        return Err(oversized_output_error(bytes_read as u64));
    }
    parse_dna_output(&contents).map_err(OutputWaitError::Malformed)
}

fn oversized_output_error(size: u64) -> OutputWaitError {
    OutputWaitError::Read(io::Error::new(
        io::ErrorKind::InvalidData,
        format!(
            "DNA output is {size} bytes; maximum accepted size is {MAX_VALID_DNA_FILE_SIZE} bytes"
        ),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::OpenOptions;
    use std::os::windows::fs::OpenOptionsExt;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_path(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("dma-tools-{name}-{nonce}.log"))
    }

    fn any_existing_file_generation() -> OutputGeneration {
        OutputGeneration {
            prepared_at: UNIX_EPOCH,
        }
    }

    #[test]
    fn cleanup_failure_is_reported_instead_of_accepting_stale_output() {
        let path = unique_path("cleanup-failure");
        fs::write(&path, "DNA = 0011 (0x3)\n").unwrap();
        let locked_file = OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(&path)
            .unwrap();

        let result = prepare_output_file(&path, &Logger::new("DnaOutputTest"));

        drop(locked_file);
        fs::remove_file(&path).unwrap();

        assert!(result.is_err());
    }

    #[test]
    fn rejects_output_older_than_the_current_operation() {
        let path = unique_path("stale");
        fs::write(&path, "DNA = 0011 (0x3)\n").unwrap();
        let generation = OutputGeneration {
            prepared_at: SystemTime::now() + Duration::from_secs(60),
        };

        let result = wait_for_parsed_output(
            &path,
            generation,
            1,
            Duration::ZERO,
            &Logger::new("DnaOutputTest"),
        );

        fs::remove_file(path).unwrap();
        assert!(matches!(result, Err(OutputWaitError::Stale)));
    }

    #[test]
    fn accepts_output_created_after_preparation() {
        let path = unique_path("fresh");
        let generation = prepare_output_file(&path, &Logger::new("DnaOutputTest")).unwrap();
        fs::write(&path, "DNA = 0011 (0x3)\n").unwrap();

        let result = wait_for_parsed_output(
            &path,
            generation,
            1,
            Duration::ZERO,
            &Logger::new("DnaOutputTest"),
        );

        fs::remove_file(path).unwrap();
        assert_eq!(result.unwrap().dna_value, "0x3");
    }

    #[test]
    fn retries_incomplete_output_until_it_is_valid() {
        let path = unique_path("incomplete-retry");
        fs::write(&path, "tiny").unwrap();
        let writer_path = path.clone();
        let writer = thread::spawn(move || {
            thread::sleep(Duration::from_millis(5));
            fs::write(writer_path, "DNA = 0011 (0x3)\n").unwrap();
        });

        let result = wait_for_parsed_output(
            &path,
            any_existing_file_generation(),
            3,
            Duration::from_millis(20),
            &Logger::new("DnaOutputTest"),
        );

        writer.join().unwrap();
        assert_eq!(result.unwrap().dna_value, "0x3");
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn retries_malformed_output_until_it_is_valid() {
        let path = unique_path("malformed-retry");
        fs::write(&path, "DNA = still being written\n").unwrap();
        let writer_path = path.clone();
        let writer = thread::spawn(move || {
            thread::sleep(Duration::from_millis(5));
            fs::write(writer_path, "DNA = 1101 (0xD)\n").unwrap();
        });

        let result = wait_for_parsed_output(
            &path,
            any_existing_file_generation(),
            3,
            Duration::from_millis(20),
            &Logger::new("DnaOutputTest"),
        );

        writer.join().unwrap();
        assert_eq!(result.unwrap().dna_value, "0xD");
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn reports_malformed_output_after_attempts_are_exhausted() {
        let path = unique_path("malformed");
        fs::write(&path, "DNA = invalid (not-hex)\n").unwrap();

        let result = wait_for_parsed_output(
            &path,
            any_existing_file_generation(),
            2,
            Duration::from_millis(1),
            &Logger::new("DnaOutputTest"),
        );

        fs::remove_file(path).unwrap();
        assert!(matches!(result, Err(OutputWaitError::Malformed(_))));
    }

    #[test]
    fn returns_unavailable_after_attempts_are_exhausted() {
        let path = unique_path("missing");
        assert!(matches!(
            wait_for_parsed_output(
                &path,
                any_existing_file_generation(),
                2,
                Duration::from_millis(1),
                &Logger::new("DnaOutputTest")
            ),
            Err(OutputWaitError::Unavailable)
        ));
    }

    #[test]
    fn rejects_oversized_output_without_reading_it() {
        let path = unique_path("oversized");
        let file = fs::File::create(&path).unwrap();
        file.set_len(MAX_VALID_DNA_FILE_SIZE + 1).unwrap();
        drop(file);

        let result = wait_for_parsed_output(
            &path,
            any_existing_file_generation(),
            1,
            Duration::ZERO,
            &Logger::new("DnaOutputTest"),
        );

        fs::remove_file(path).unwrap();
        assert!(matches!(
            result,
            Err(OutputWaitError::Read(error)) if error.kind() == io::ErrorKind::InvalidData
        ));
    }
}
