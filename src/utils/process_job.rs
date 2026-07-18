use std::io;
use std::mem::{size_of, zeroed};
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::process::Child;
use std::ptr;
use std::thread;
use std::time::{Duration, Instant};
use winapi::shared::minwindef::{DWORD, FALSE};
use winapi::um::handleapi::INVALID_HANDLE_VALUE;
use winapi::um::jobapi2::{
    AssignProcessToJobObject, CreateJobObjectW, QueryInformationJobObject, SetInformationJobObject,
    TerminateJobObject,
};
use winapi::um::processthreadsapi::{OpenThread, ResumeThread};
use winapi::um::tlhelp32::{
    CreateToolhelp32Snapshot, TH32CS_SNAPTHREAD, THREADENTRY32, Thread32First, Thread32Next,
};
use winapi::um::winnt::{
    JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE, JOBOBJECT_BASIC_ACCOUNTING_INFORMATION,
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectBasicAccountingInformation,
    JobObjectExtendedLimitInformation, THREAD_SUSPEND_RESUME,
};

/// Prevents a newly created process from executing until its primary thread is
/// explicitly resumed. This gives callers time to assign it to a Job Object
/// before it can touch hardware or create descendants.
pub const CREATE_SUSPENDED: u32 = 0x0000_0004;

/// Owns a Windows Job Object that terminates every assigned process when the
/// final job handle is closed, including during application shutdown.
pub struct ProcessJob {
    handle: OwnedHandle,
}

impl ProcessJob {
    pub fn new_kill_on_close() -> Result<Self, String> {
        // SAFETY: Null attributes/name request a private job with default security.
        let raw_handle = unsafe { CreateJobObjectW(ptr::null_mut(), ptr::null()) };
        if raw_handle.is_null() {
            return Err(format!(
                "Failed to create process job: {}",
                io::Error::last_os_error()
            ));
        }

        // SAFETY: CreateJobObjectW returned a new owned handle on success.
        let handle = unsafe { OwnedHandle::from_raw_handle(raw_handle.cast()) };
        // SAFETY: The Windows structure is plain data and zero is its documented
        // baseline before selecting the limit flags below.
        let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { zeroed() };
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;

        // SAFETY: `limits` points to a correctly sized structure for the selected
        // information class, and `handle` remains valid for the duration of the call.
        let configured = unsafe {
            SetInformationJobObject(
                handle.as_raw_handle().cast(),
                JobObjectExtendedLimitInformation,
                (&mut limits as *mut JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as DWORD,
            )
        };
        if configured == FALSE {
            return Err(format!(
                "Failed to configure process job: {}",
                io::Error::last_os_error()
            ));
        }

        Ok(Self { handle })
    }

    /// Assigns a newly spawned process to this job. Call this immediately after
    /// spawning and before handing the child to another thread.
    pub fn assign(&self, child: &Child) -> Result<(), String> {
        // SAFETY: Both handles are valid and borrowed for this call. Windows owns
        // the association; neither handle is consumed.
        let assigned = unsafe {
            AssignProcessToJobObject(
                self.handle.as_raw_handle().cast(),
                child.as_raw_handle().cast(),
            )
        };
        if assigned == FALSE {
            return Err(format!(
                "Failed to assign process to job: {}",
                io::Error::last_os_error()
            ));
        }

        Ok(())
    }

    /// Assigns a process created with [`CREATE_SUSPENDED`] to this job and then
    /// resumes its initial thread. The process cannot execute outside the job.
    pub fn assign_and_resume(&self, child: &Child) -> Result<(), String> {
        self.assign(child)?;
        Self::resume_initial_thread(child.id())
    }

    fn resume_initial_thread(process_id: DWORD) -> Result<(), String> {
        // SAFETY: A system-wide thread snapshot requires no caller-provided
        // pointers and returns a new handle owned by the caller.
        let raw_snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) };
        if raw_snapshot == INVALID_HANDLE_VALUE {
            return Err(format!(
                "Failed to enumerate suspended process threads: {}",
                io::Error::last_os_error()
            ));
        }

        // SAFETY: CreateToolhelp32Snapshot returned a new owned handle.
        let snapshot = unsafe { OwnedHandle::from_raw_handle(raw_snapshot.cast()) };
        // SAFETY: THREADENTRY32 is plain data. Windows requires dwSize to be
        // initialized before the first enumeration call.
        let mut entry: THREADENTRY32 = unsafe { zeroed() };
        entry.dwSize = size_of::<THREADENTRY32>() as DWORD;

        // SAFETY: `entry` is correctly sized and initialized, while `snapshot`
        // remains valid throughout enumeration.
        let mut has_entry = unsafe {
            Thread32First(
                snapshot.as_raw_handle().cast(),
                &mut entry as *mut THREADENTRY32,
            )
        };
        while has_entry != FALSE {
            if entry.th32OwnerProcessID == process_id {
                // SAFETY: The enumerated thread identifier belongs to the
                // suspended child. The returned handle is owned by the caller.
                let raw_thread =
                    unsafe { OpenThread(THREAD_SUSPEND_RESUME, FALSE, entry.th32ThreadID) };
                if raw_thread.is_null() {
                    return Err(format!(
                        "Failed to open suspended process thread: {}",
                        io::Error::last_os_error()
                    ));
                }

                // SAFETY: OpenThread returned a new owned handle on success.
                let thread_handle = unsafe { OwnedHandle::from_raw_handle(raw_thread.cast()) };
                // SAFETY: The handle identifies a live thread and grants
                // THREAD_SUSPEND_RESUME access.
                let previous_suspend_count =
                    unsafe { ResumeThread(thread_handle.as_raw_handle().cast()) };
                if previous_suspend_count == DWORD::MAX {
                    return Err(format!(
                        "Failed to resume owned process: {}",
                        io::Error::last_os_error()
                    ));
                }
                if previous_suspend_count != 1 {
                    return Err(format!(
                        "Owned process had unexpected suspend count {previous_suspend_count}; expected 1"
                    ));
                }

                return Ok(());
            }

            // SAFETY: Same valid snapshot and initialized output structure as
            // Thread32First above.
            has_entry = unsafe {
                Thread32Next(
                    snapshot.as_raw_handle().cast(),
                    &mut entry as *mut THREADENTRY32,
                )
            };
        }

        Err(format!(
            "Could not find the initial thread for suspended process {process_id}"
        ))
    }

    /// Confirms that the job is empty, forcibly terminating any descendants
    /// that outlived the process the caller directly waited for.
    pub fn terminate_remaining_and_wait(
        &self,
        timeout: Duration,
        poll_interval: Duration,
    ) -> Result<(), String> {
        if self.active_processes()? == 0 {
            return Ok(());
        }

        self.terminate_and_wait(timeout, poll_interval)
    }

    /// Terminates every process in the job and confirms that none remain.
    pub fn terminate_and_wait(
        &self,
        timeout: Duration,
        poll_interval: Duration,
    ) -> Result<(), String> {
        // SAFETY: The job handle is valid. Exit code 1 is used only for forced
        // cancellation and is not observed as a successful child exit.
        let terminated = unsafe { TerminateJobObject(self.handle.as_raw_handle().cast(), 1) };
        if terminated == FALSE && self.active_processes()? != 0 {
            return Err(format!(
                "Failed to terminate process job: {}",
                io::Error::last_os_error()
            ));
        }

        let deadline = Instant::now() + timeout;
        loop {
            if self.active_processes()? == 0 {
                return Ok(());
            }
            if Instant::now() >= deadline {
                return Err(format!(
                    "Process job did not terminate within {} seconds",
                    timeout.as_secs_f32()
                ));
            }

            let remaining = deadline.saturating_duration_since(Instant::now());
            thread::sleep(poll_interval.max(Duration::from_millis(1)).min(remaining));
        }
    }

    fn active_processes(&self) -> Result<DWORD, String> {
        // SAFETY: The accounting structure is plain data and is fully initialized
        // by QueryInformationJobObject on success.
        let mut accounting: JOBOBJECT_BASIC_ACCOUNTING_INFORMATION = unsafe { zeroed() };
        // SAFETY: `accounting` has the exact type and size required by the selected
        // information class, and the optional return-length pointer may be null.
        let queried = unsafe {
            QueryInformationJobObject(
                self.handle.as_raw_handle().cast(),
                JobObjectBasicAccountingInformation,
                (&mut accounting as *mut JOBOBJECT_BASIC_ACCOUNTING_INFORMATION).cast(),
                size_of::<JOBOBJECT_BASIC_ACCOUNTING_INFORMATION>() as DWORD,
                ptr::null_mut(),
            )
        };
        if queried == FALSE {
            return Err(format!(
                "Failed to inspect process job: {}",
                io::Error::last_os_error()
            ));
        }

        Ok(accounting.ActiveProcesses)
    }
}
