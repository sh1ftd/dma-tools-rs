use super::{CompletionStatus, FlashingOption};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub const NORMAL_WRITE_THRESHOLD_MS: u32 = 10;
pub const MIN_NORMAL_WRITES_REQUIRED: usize = 5;
pub const MIN_SECTORS_BEFORE_CHECK: usize = 10;
const SECTOR_STUCK_THRESHOLD: Duration = Duration::from_secs(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OperationStage {
    #[default]
    Starting,
    InitializingJtag,
    LoadingBitstream,
    ResettingFpga,
    ProbingFlash,
    WritingImage,
    WritingSector(u32),
    Verifying,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenOcdEvent {
    StageChanged(OperationStage),
    SectorWritten {
        sector: Option<u32>,
        elapsed_ms: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SectorStats {
    pub total: usize,
    pub normal: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlashAssessment {
    NotApplicable,
    Pending,
    Success,
    SuccessWithLimitedSamples {
        total_sectors: usize,
    },
    ConnectionUnstable {
        normal_writes: usize,
        total_sectors: usize,
    },
    Indeterminate,
    Failed(String),
    UnexpectedDnaResult,
}

impl FlashAssessment {
    pub fn allows_source_cleanup(&self) -> bool {
        matches!(self, Self::Success | Self::SuccessWithLimitedSamples { .. })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FinalizationOutcome {
    NotTerminal,
    NotApplicable,
    CleanupNotRequested { assessment: FlashAssessment },
    SourcePreserved { assessment: FlashAssessment },
    SourceRemoved { path: PathBuf },
    SourceAlreadyMissing { path: PathBuf },
    CleanupAlreadyCompleted,
    SourceUnavailable { assessment: FlashAssessment },
    CleanupFailed { path: PathBuf, error: String },
}

#[derive(Debug, Clone, PartialEq)]
pub struct OperationSnapshot {
    pub status: CompletionStatus,
    pub safe_to_restart: bool,
    pub option: Option<FlashingOption>,
    pub stage: OperationStage,
    pub current_sector: Option<u32>,
    pub sector_stats: SectorStats,
    pub duration: Option<Duration>,
    pub assessment: FlashAssessment,
    pub terminated_early: bool,
}

#[derive(Debug, Clone, Copy)]
struct OperationProgress {
    stage: OperationStage,
    current_sector: Option<u32>,
    last_sector_at: Option<Instant>,
    sector_stats: SectorStats,
}

impl Default for OperationProgress {
    fn default() -> Self {
        Self {
            stage: OperationStage::Starting,
            current_sector: None,
            last_sector_at: None,
            sector_stats: SectorStats::default(),
        }
    }
}

#[derive(Clone, Default)]
pub struct OperationTracker {
    progress: Arc<Mutex<OperationProgress>>,
}

impl OperationTracker {
    pub fn reset(&self) {
        *self.progress.lock().unwrap() = OperationProgress::default();
    }

    pub fn record_line(&self, line: &str) -> Option<OpenOcdEvent> {
        let event = parse_openocd_line(line)?;
        self.record_event(event);
        Some(event)
    }

    fn record_event(&self, event: OpenOcdEvent) {
        let mut progress = self.progress.lock().unwrap();

        match event {
            OpenOcdEvent::StageChanged(stage) => {
                progress.stage = stage;
                if !matches!(stage, OperationStage::WritingSector(_)) {
                    progress.current_sector = None;
                    progress.last_sector_at = None;
                }
            }
            OpenOcdEvent::SectorWritten { sector, elapsed_ms } => {
                progress.sector_stats.total += 1;
                if elapsed_ms >= NORMAL_WRITE_THRESHOLD_MS {
                    progress.sector_stats.normal += 1;
                }

                if let Some(sector) = sector {
                    progress.stage = OperationStage::WritingSector(sector);
                    progress.current_sector = Some(sector);
                } else {
                    progress.stage = OperationStage::WritingImage;
                    progress.current_sector = None;
                }
                progress.last_sector_at = Some(Instant::now());
            }
        }
    }

    pub fn sector_stats(&self) -> SectorStats {
        self.progress.lock().unwrap().sector_stats
    }

    fn snapshot(&self, now: Instant) -> ProgressSnapshot {
        let progress = *self.progress.lock().unwrap();
        let stage = if progress.current_sector.is_some()
            && progress
                .last_sector_at
                .is_some_and(|seen| now.duration_since(seen) > SECTOR_STUCK_THRESHOLD)
        {
            OperationStage::Verifying
        } else {
            progress.stage
        };

        ProgressSnapshot {
            stage,
            current_sector: progress.current_sector,
            sector_stats: progress.sector_stats,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ProgressSnapshot {
    pub stage: OperationStage,
    pub current_sector: Option<u32>,
    pub sector_stats: SectorStats,
}

pub fn operation_progress_snapshot(tracker: &OperationTracker) -> ProgressSnapshot {
    tracker.snapshot(Instant::now())
}

pub fn parse_openocd_line(line: &str) -> Option<OpenOcdEvent> {
    if line.contains("sector") && line.contains("took") {
        let elapsed_ms = line
            .split("took")
            .nth(1)?
            .split_whitespace()
            .next()?
            .parse()
            .ok()?;
        let sector = line
            .split("sector")
            .nth(1)
            .and_then(|part| part.split("took").next())
            .and_then(|value| value.trim().parse().ok());

        return Some(OpenOcdEvent::SectorWritten { sector, elapsed_ms });
    }

    let stage = if line.contains("Writing the image to the flash memory") {
        OperationStage::WritingImage
    } else if line.contains("Probing the flash memory") {
        OperationStage::ProbingFlash
    } else if line.contains("Resetting and halting the FPGA") {
        OperationStage::ResettingFpga
    } else if line.contains("Loading the bitstream") {
        OperationStage::LoadingBitstream
    } else if line.contains("Initializing the JTAG interface") {
        OperationStage::InitializingJtag
    } else {
        return None;
    };

    Some(OpenOcdEvent::StageChanged(stage))
}

pub fn connection_is_unstable(stats: SectorStats) -> bool {
    stats.total >= MIN_SECTORS_BEFORE_CHECK && stats.normal < MIN_NORMAL_WRITES_REQUIRED
}

pub fn assess_flash(
    status: &CompletionStatus,
    stats: SectorStats,
    terminated_early: bool,
) -> FlashAssessment {
    match status {
        CompletionStatus::NotCompleted | CompletionStatus::InProgress(_) => {
            FlashAssessment::Pending
        }
        _ if terminated_early => FlashAssessment::ConnectionUnstable {
            normal_writes: stats.normal,
            total_sectors: stats.total,
        },
        CompletionStatus::Failed(error) => FlashAssessment::Failed(error.clone()),
        CompletionStatus::DnaReadCompleted(_) => FlashAssessment::UnexpectedDnaResult,
        CompletionStatus::Completed => {
            if connection_is_unstable(stats) {
                FlashAssessment::ConnectionUnstable {
                    normal_writes: stats.normal,
                    total_sectors: stats.total,
                }
            } else if stats.total >= MIN_SECTORS_BEFORE_CHECK {
                FlashAssessment::Success
            } else if stats.total == 0 {
                FlashAssessment::Indeterminate
            } else {
                FlashAssessment::SuccessWithLimitedSamples {
                    total_sectors: stats.total,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_all_known_stages() {
        let cases = [
            (
                "Initializing the JTAG interface...",
                OperationStage::InitializingJtag,
            ),
            ("Loading the bitstream...", OperationStage::LoadingBitstream),
            (
                "Resetting and halting the FPGA...",
                OperationStage::ResettingFpga,
            ),
            ("Probing the flash memory...", OperationStage::ProbingFlash),
            (
                "Writing the image to the flash memory...",
                OperationStage::WritingImage,
            ),
        ];

        for (line, expected) in cases {
            assert_eq!(
                parse_openocd_line(line),
                Some(OpenOcdEvent::StageChanged(expected))
            );
        }
    }

    #[test]
    fn parses_standard_and_prefixed_sector_lines() {
        assert_eq!(
            parse_openocd_line("Info : sector 12 took 25 ms"),
            Some(OpenOcdEvent::SectorWritten {
                sector: Some(12),
                elapsed_ms: 25,
            })
        );
        assert_eq!(
            parse_openocd_line("[ERROR] Info : sector 3 took 1 ms"),
            Some(OpenOcdEvent::SectorWritten {
                sector: Some(3),
                elapsed_ms: 1,
            })
        );
    }

    #[test]
    fn rejects_malformed_sector_lines() {
        assert_eq!(parse_openocd_line("Info : sector 4 took nope ms"), None);
        assert_eq!(parse_openocd_line("Info : sector 4 completed"), None);
    }

    #[test]
    fn does_not_invent_success_from_an_unproduced_log_phrase() {
        assert_eq!(
            parse_openocd_line("Firmware flash completed successfully"),
            None
        );
    }

    #[test]
    fn tracker_reset_clears_progress_without_ui_state() {
        let tracker = OperationTracker::default();
        tracker.record_line("Info : sector 8 took 15 ms");
        assert_eq!(
            tracker.sector_stats(),
            SectorStats {
                total: 1,
                normal: 1
            }
        );

        tracker.reset();

        let snapshot = operation_progress_snapshot(&tracker);
        assert_eq!(snapshot.stage, OperationStage::Starting);
        assert_eq!(snapshot.current_sector, None);
        assert_eq!(snapshot.sector_stats, SectorStats::default());
    }

    #[test]
    fn tracker_exposes_verifying_after_sector_stalls() {
        let tracker = OperationTracker::default();
        tracker.record_line("Info : sector 8 took 15 ms");

        let snapshot = tracker.snapshot(Instant::now() + Duration::from_secs(2));

        assert_eq!(snapshot.stage, OperationStage::Verifying);
        assert_eq!(snapshot.current_sector, Some(8));
    }

    #[test]
    fn assessment_preserves_existing_thresholds() {
        assert_eq!(
            assess_flash(
                &CompletionStatus::Completed,
                SectorStats {
                    total: 10,
                    normal: 4
                },
                false,
            ),
            FlashAssessment::ConnectionUnstable {
                normal_writes: 4,
                total_sectors: 10,
            }
        );
        assert_eq!(
            assess_flash(
                &CompletionStatus::Completed,
                SectorStats {
                    total: 10,
                    normal: 5
                },
                false,
            ),
            FlashAssessment::Success
        );
        assert_eq!(
            assess_flash(
                &CompletionStatus::Completed,
                SectorStats {
                    total: 9,
                    normal: 2
                },
                false,
            ),
            FlashAssessment::SuccessWithLimitedSamples { total_sectors: 9 }
        );
        assert_eq!(
            assess_flash(
                &CompletionStatus::Completed,
                SectorStats {
                    total: 9,
                    normal: 5,
                },
                false,
            ),
            FlashAssessment::SuccessWithLimitedSamples { total_sectors: 9 }
        );
        assert_eq!(
            assess_flash(&CompletionStatus::Completed, SectorStats::default(), false,),
            FlashAssessment::Indeterminate
        );
    }

    #[test]
    fn assessment_uses_process_completion_and_observed_sector_thresholds() {
        assert_eq!(
            assess_flash(
                &CompletionStatus::InProgress("working".to_string()),
                SectorStats {
                    total: 10,
                    normal: 10,
                },
                false,
            ),
            FlashAssessment::Pending
        );
        assert_eq!(
            assess_flash(
                &CompletionStatus::Failed("boom".to_string()),
                SectorStats {
                    total: 10,
                    normal: 10,
                },
                false,
            ),
            FlashAssessment::Failed("boom".to_string())
        );
    }

    #[test]
    fn early_termination_is_a_typed_terminal_assessment() {
        let stats = SectorStats {
            total: 10,
            normal: 4,
        };

        for status in [
            CompletionStatus::Completed,
            CompletionStatus::Failed("forced process termination".to_string()),
        ] {
            assert_eq!(
                assess_flash(&status, stats, true),
                FlashAssessment::ConnectionUnstable {
                    normal_writes: 4,
                    total_sectors: 10,
                }
            );
        }

        assert_eq!(
            assess_flash(
                &CompletionStatus::InProgress("stopping".to_string()),
                stats,
                true,
            ),
            FlashAssessment::Pending,
            "the retry loop must still wait for the owned process to become terminal"
        );
    }

    #[test]
    fn only_success_assessments_allow_source_cleanup() {
        assert!(FlashAssessment::Success.allows_source_cleanup());
        assert!(
            FlashAssessment::SuccessWithLimitedSamples { total_sectors: 3 }.allows_source_cleanup()
        );

        for assessment in [
            FlashAssessment::Indeterminate,
            FlashAssessment::ConnectionUnstable {
                normal_writes: 4,
                total_sectors: 10,
            },
            FlashAssessment::Failed("failed".to_string()),
            FlashAssessment::Pending,
        ] {
            assert!(!assessment.allows_source_cleanup());
        }
    }
}
