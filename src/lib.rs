// Disable warning for `non_snake_case` in the crate.
// It's not the better way to disable this warning only for the crate name.
// See https://github.com/rust-lang/rust/issues/45127
#![allow(non_snake_case)]

//! This crate provide an API similar to `std::process` to create and handle
//! processes on Windows using the Win32 API through the [windows-rs] crate (see
//! [this example].
//!
//! It's main difference with `std::process::Command` is that it allows running
//! a command string instead of having to pass the command executable and the
//! arguments separately.
//!
//! This is equivalent of running:
//!
//! ```
//! std::process::Command("cmd.exe")
//!     .arg("/c")
//!     .arg(any_command_string)
//!     .spawn()
//! ```
//!
//! The only difference will be that the `Child` instance will use the PID of
//! the command instead of the PID of `cmd.exe`. This is important because
//! calling `.terminate()` in the code above does not work as it kills the PID
//! of `cmd.exe` instead of the actual command that has been ran.
//!
//! # Usage
//!
//! Add the following to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! create_process_w = { version = "0.1.0", package = "CreateProcessW" }
//! ```
//!
//! You can also use `CreateProcessW` directly, but this doesn't respect Rust's
//! naming recommendations.
//!
use std::ffi::OsString;
use std::path::PathBuf;

#[derive(Debug)]
/// a process builder, providing control over how a new process should be
/// spawned.
pub struct Command {
    /// The command line you want to execute
    command: OsString,
    /// Disable/enable handles inherance
    inherit_handles: bool,
    /// The full path of the current directory for the process
    current_directory: Option<PathBuf>,
}

impl Command {
    /// Create a new `Command`, with the following default configuration:
    ///
    /// * Inherit handles of the calling process.
    /// * Inherit the current drive and directory of the calling process.
    ///
    /// Builder methods are provided to change these defaults and otherwise
    /// configure the process.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use create_process_w::Command;
    ///
    /// Command::new("notepad.exe")
    ///     .spawn()
    ///     .expect("notepad failed to start");
    /// ```
    ///
    /// Equivalent to the `lpCommandLine` parameter of the [`CreateProcessW`] function.
    pub fn new(command: impl Into<OsString>) -> Self {
        Self {
            command: command.into(),
            inherit_handles: true,
            current_directory: None,
        }
    }

    /// Enable/disable handles inherance.
    ///
    /// If this parameter is `true`, each inheritable handle in the calling
    /// process is inherited by the new process. If the parameter is `false`,
    /// the handles are not inherited. Note that inherited handles have the
    /// same value and access rights as the original handles.
    pub fn inherit_handles(&mut self, inherit: bool) -> &mut Self {
        self.inherit_handles = inherit;
        self
    }

    /// Sets the working directory for the child process.
    ///
    /// It's the full path to the current directory for the process.
    ///
    /// Note that you can use a raw string to avoid error when copy-pasting the
    /// path (`r"<path>"`).
    pub fn current_directory(&mut self, dir: impl Into<PathBuf>) -> &mut Self {
        self.current_directory = Some(dir.into());
        self
    }

    /// Executes the command as a child process, returning a handle to it.
    pub fn spawn(&mut self) -> Result<Child> {
        Child::new(
            &self.command,
            self.inherit_handles,
            self.current_directory.as_deref(),
        )
    }

    /// Executes a command as a child process, waiting for it to finish and
    /// collecting its status.
    pub fn status(&mut self) -> Result<ExitStatus> {
        self.spawn()?.wait()
    }
}

use std::ffi::{c_void, OsStr};
use std::mem::size_of;
use std::path::Path;
use windows::Win32::Foundation::{GetLastError, PWSTR, STATUS_PENDING};
use windows::Win32::Security::SECURITY_ATTRIBUTES;
use windows::Win32::System::Threading::{
    GetExitCodeProcess, TerminateProcess, WaitForSingleObject, PROCESS_CREATION_FLAGS,
    PROCESS_INFORMATION, STARTUPINFOW, WAIT_OBJECT_0,
};
use windows::Win32::System::WindowsProgramming::INFINITE;

#[derive(Debug)]
pub struct Child {
    process_information: PROCESS_INFORMATION,
}

impl Child {
    fn new(
        command: &OsStr,
        inherit_handles: bool,
        current_directory: Option<&Path>,
    ) -> Result<Self> {
        let mut startup_info = STARTUPINFOW::default();
        let mut process_info = PROCESS_INFORMATION::default();

        startup_info.cb = size_of::<STARTUPINFOW>() as u32;

        let process_creation_flags = PROCESS_CREATION_FLAGS(0);

        let res = unsafe {
            if let Some(directory) = current_directory {
                let directory = directory.as_os_str();
                windows::Win32::System::Threading::CreateProcessW(
                    PWSTR::default(),
                    command,
                    std::ptr::null() as *const SECURITY_ATTRIBUTES,
                    std::ptr::null() as *const SECURITY_ATTRIBUTES,
                    inherit_handles,
                    process_creation_flags,
                    std::ptr::null() as *const c_void,
                    directory,
                    &startup_info,
                    &mut process_info as *mut PROCESS_INFORMATION,
                )
            } else {
                windows::Win32::System::Threading::CreateProcessW(
                    PWSTR::default(),
                    command,
                    std::ptr::null() as *const SECURITY_ATTRIBUTES,
                    std::ptr::null() as *const SECURITY_ATTRIBUTES,
                    inherit_handles,
                    process_creation_flags,
                    std::ptr::null() as *const c_void,
                    PWSTR::default(),
                    &startup_info,
                    &mut process_info as *mut PROCESS_INFORMATION,
                )
            }
        };

        if res.as_bool() {
            Ok(Self {
                process_information: process_info,
            })
        } else {
            Err(Error::CreationFailed(unsafe { GetLastError().0 }))
        }
    }

    pub fn wait(&self) -> Result<ExitStatus> {
        unsafe {
            let mut exit_code: u32 = 0;
            let res = WaitForSingleObject(self.process_information.hProcess, INFINITE);

            if res == WAIT_OBJECT_0 {
                if GetExitCodeProcess(
                    self.process_information.hProcess,
                    &mut exit_code as *mut u32,
                )
                .as_bool()
                {
                    close_handles(&self.process_information);
                    Ok(ExitStatus(exit_code))
                } else {
                    Err(Error::GetExitCodeFailed(GetLastError().0))
                }
            } else {
                Err(Error::GetExitCodeFailed(GetLastError().0))
            }
        }
    }

    pub fn try_wait(&self) -> Result<Option<ExitStatus>> {
        unsafe {
            let mut exit_code: u32 = 0;

            let res = GetExitCodeProcess(
                self.process_information.hProcess,
                &mut exit_code as *mut u32,
            );

            if res.as_bool() {
                if exit_code == STATUS_PENDING.0 {
                    Ok(None)
                } else {
                    close_handles(&self.process_information);
                    Ok(Some(ExitStatus(exit_code)))
                }
            } else {
                Err(Error::GetExitCodeFailed(GetLastError().0))
            }
        }
    }

    pub fn kill(&self) -> Result<()> {
        unsafe {
            let res = TerminateProcess(self.process_information.hProcess, 0);

            if res.as_bool() {
                Ok(())
            } else {
                Err(Error::KillFailed(GetLastError().0))
            }
        }
    }
}

pub struct ExitStatus(u32);

impl ExitStatus {
    pub fn success(&self) -> bool {
        self.0 == 0
    }

    pub fn code(&self) -> u32 {
        self.0
    }
}

use thiserror::Error;

type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot create process: {0}")]
    CreationFailed(u32),
    #[error("cannot get exit status: {0}")]
    GetExitCodeFailed(u32),
    #[error("cannot kill process: {0}")]
    KillFailed(u32),
}

use windows::Win32::Foundation::CloseHandle;

unsafe fn close_handles(process_info: &PROCESS_INFORMATION) {
    CloseHandle(process_info.hProcess);
    CloseHandle(process_info.hThread);
}
