// Disable warning for `non_snake_case` in the crate and when the lib is used as
// a dependency. It's not the better way to disable this warning only for the
// crate name. See https://github.com/rust-lang/rust/issues/45127
#![allow(non_snake_case)]
#![deny(missing_docs)]
// intra-doc links need to be explicit in the README.tpl because they're not
// parsed correctly when generating the README via cargo-readme.
// See https://github.com/livioribeiro/cargo-readme/issues/55
// The generation of the README need to be automated when a PR is pushed on main.

//! This crate provides an API similar to [`std::process`][std-process] to create
//! and handle processes on Windows using the Win32 API through the
//! [windows-rs][windows-rs] crate (see [this example][create-processes-example]).
//!
//! Its main difference with `std::process::Command` is that it allows running
//! a command string instead of having to pass the command executable and the
//! arguments separately.
//!
//! This is equivalent of running:
//!
//! ```no_run
//! std::process::Command::new("cmd.exe")
//!     .arg("/c")
//!     .arg("any_command_string")
//!     .spawn().expect("cannot spawn command");
//! ```
//!
//! The only difference will be that the `Child` instance will use the PID of
//! the command instead of the PID of `cmd.exe`. This is important because
//! calling `.kill()` in the code above does not work as it kills the PID
//! of `cmd.exe` instead of the actual command that has been ran.
//!
//! # Usage
//!
//! Add the following to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! CreateProcessW = "0.1.0"
//! ```
//!
//! This crate doesn't follow Rust's naming recommendations. If you want to stay
//! consistent with other imported crates, use the following:
//!
//! ```toml
//! [dependencies]
//! create_process_w = { version = "0.1.0", package = "CreateProcessW" }
//! ```
//!
//! # Create a command
//!
//! The [`Command`] struct is used to configure and spawn processes:
//!
//! ```no_run
//! use CreateProcessW::Command;
//!
//! let command = Command::new("cargo.exe clippy -- -D warnings")
//!     .inherit_handles(false)
//!     .current_directory(r"C:\Users\<user>\repos\<repo_name>");
//! ```
//!
//! ## Spawning a process
//!
//! The [`spawn`][Command::spawn] function spawns the process and returns a
//! [`Child`] that represents the spawned child process.
//!
//! ```no_run
//! use CreateProcessW::Command;
//!
//! let child = Command::new("notepad.exe")
//!     .spawn()
//!     .expect("notepad failed to start");
//!
//!
//! std::thread::sleep(std::time::Duration::from_secs(2));
//!
//! child.kill().expect("cannot kill process");
//! let status = child.wait().expect("cannot wait process");
//!
//! if status.success() {
//!     println!("Success!");
//! } else {
//!     println!("Process exited with status {}", status.code());
//! }
//! ```
//!
//! The [`status`][Command::status] function spawns a child process, waits for
//! it to finish and returns its [`ExitStatus`].
//!
//! ```no_run
//! use CreateProcessW::Command;
//!
//! let status = Command::new("notepad.exe")
//!     .status()
//!     .expect("notepad failed to start");
//!
//! if status.success() {
//!     println!("Success!")
//! } else {
//!     println!("Process exited with status {}", status.code())
//! }
//! ```
//!
//! [std-process]: https://doc.rust-lang.org/std/process/index.html
//! [windows-rs]: https://github.com/microsoft/windows-rs
//! [create-processes-example]: https://docs.microsoft.com/en-us/windows/win32/procthread/creating-processes
use std::ffi::OsString;
use std::path::PathBuf;

/// A process builder, providing control over how a new process should be
/// spawned.
#[derive(Debug)]
pub struct Command {
    command: OsString,
    inherit_handles: bool,
    current_directory: Option<PathBuf>,
}

impl Command {
    /// Create a new [`Command`], with the following default configuration:
    ///
    /// * Inherit handles of the calling process.
    /// * Inherit the current drive and directory of the calling process.
    ///
    /// Builder methods are provided to change these defaults and otherwise
    /// configure the process.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use CreateProcessW::Command;
    ///
    /// Command::new("notepad.exe")
    ///     .spawn()
    ///     .expect("notepad failed to start");
    /// ```
    ///
    /// Equivalent to the `lpCommandLine` parameter of the
    /// [`CreateProcessW`][create-process-w-parameters] function.
    ///
    /// [create-process-w-parameters]: https://docs.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-createprocessw#parameters
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
    ///
    /// Equivalent to the `bInheritHandles` parameter of the
    /// [`CreateProcessW`][create-process-w-parameters] function.
    ///
    /// [create-process-w-parameters]: https://docs.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-createprocessw#parameters
    pub fn inherit_handles(&mut self, inherit: bool) -> &mut Self {
        self.inherit_handles = inherit;
        self
    }

    /// Sets the working directory for the child process.
    ///
    /// It's the full path to the current directory for the process. Note that
    /// you can use a raw string to avoid error when copy-pasting the path.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use CreateProcessW::Command;
    ///
    /// let check = Command::new("cargo.exe check")
    ///     .current_directory(r"C:\Users\<user>\repos\<repo_name>")
    ///     .status()
    ///     .expect("cargo check command failed");
    /// ```
    ///
    /// Equivalent to the `lpCurrentDirectory` parameter of the
    /// [`CreateProcessW`][create-process-w-parameters] function.
    ///
    /// [create-process-w-parameters]: https://docs.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-createprocessw#parameters
    pub fn current_directory(&mut self, dir: impl Into<PathBuf>) -> &mut Self {
        self.current_directory = Some(dir.into());
        self
    }

    /// Executes the command as a child process, returning a handle to it.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use CreateProcessW::Command;
    ///
    /// Command::new("notepad.exe")
    ///     .spawn()
    ///     .expect("notepad failed to start");
    /// ```
    pub fn spawn(&mut self) -> Result<Child> {
        Child::new(
            &self.command,
            self.inherit_handles,
            self.current_directory.as_deref(),
        )
    }

    /// Executes a command as a child process, waiting for it to finish and
    /// collecting its status.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use CreateProcessW::Command;
    ///
    /// let status = Command::new("notepad.exe")
    ///     .status()
    ///     .expect("failed to execute process");
    ///
    /// println!("process finished with: {}", status.code());
    ///
    /// assert!(status.success());
    /// ```
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
    GetExitCodeProcess, TerminateProcess, WaitForSingleObject,
    PROCESS_CREATION_FLAGS, PROCESS_INFORMATION, STARTUPINFOW, WAIT_OBJECT_0,
};
use windows::Win32::System::WindowsProgramming::INFINITE;

/// Representation of a running or exited child process.
///
/// This structure is used to represent and manage child processes. A child
/// process is created via the [`Command`] struct, which configures the spawning
/// process and can itself be constructed using a builder-style interface.
///
/// # Warnings
///
/// Calling [`wait`][Child::wait] is necessary for the OS to release resources.
/// A process that terminated but has not been waited on is still around as a
/// "zombie". Leaving too many zombies around may exhaust global resources.
///
/// This library does *not* automatically wait on child processes (not even if
/// the `Child` is dropped), it is up to the application developer to do so. As
/// a consequence, dropping `Child` handles without waiting on them first is not
/// recommended in long-running applications.
///
/// # Examples
///
/// ```no_run
/// use CreateProcessW::Command;
///
/// let mut child = Command::new("notepad.exe")
///     .spawn()
///     .expect("failed to execute child");
///
/// let status = child.wait().expect("failed to wait on child");
///
/// assert!(status.success());
/// ```
#[derive(Debug)]
pub struct Child {
    process_information: PROCESS_INFORMATION,
}

impl Child {
    #[doc(hidden)]
    /// Create a new process and initialize it's memory. If it cannot be
    /// created, an [`CreateFailed`][Error::CreateFailed] error is returned.
    ///
    /// Equivalent to [`CreateProcessW`][create-process-w]
    ///
    /// [create-process-w]: https://docs.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-createprocessw
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
    /// Forces the child process to exit. If the child has already exited, a
    /// [`KillFailed`][Error::KillFailed] error is returned.
    ///
    /// This function is used to unconditionally cause a process to exit and
    /// stops execution of all threads within the process and requests
    /// cancellation of all pending I/O. The terminated process cannot exit
    /// until all pending I/O has been completed and canceled. When a
    /// process terminates, its kernel object is not destroyed until all
    /// processes that have open handles to the process have released those
    /// handles.
    ///
    /// Equivalent to the [`TerminateProcess`][terminate-process] function.
    /// Note that the value passed as the `uExitCode` is always `0` at the
    /// moment.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use CreateProcessW::Command;
    ///
    /// let mut command = Command::new("notepad.exe");
    ///
    /// if let Ok(mut child) = command.spawn() {
    ///     child.kill().expect("notepad wasn't running");
    /// } else {
    ///     println!("notepad didn't start");
    /// }
    /// ```
    ///
    /// [terminate-process]: https://docs.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-terminateprocess
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

    /// Waits for the child to exit completely, returning the status that it
    /// exited with and closing handles. This function will continue to have the
    /// same return value after it has been called at least once.
    ///
    /// If the function fail, it return a
    /// [`GetExitCodeFailed`][Error::GetExitCodeFailed] error.
    ///
    /// This is equivalent to calling the
    /// [`WaitForSingleObject][wait-for-single-object] and the
    /// [`CloseHandle`][close-handle] functions.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use CreateProcessW::Command;
    ///
    /// let mut command = Command::new("notepad.exe");
    ///
    /// if let Ok(mut child) = command.spawn() {
    ///     child.wait().expect("command wasn't running");
    ///     println!("Child has finished its execution!");
    /// } else {
    ///     println!("notepad didn't start");
    /// }
    /// ```
    ///
    /// [wait-for-single-object]: https://docs.microsoft.com/en-us/windows/win32/api/synchapi/nf-synchapi-waitforsingleobject
    /// [close-handle]: https://docs.microsoft.com/en-us/windows/win32/api/handleapi/nf-handleapi-closehandle
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
                Err(Error::WaitFailed(GetLastError().0))
            }
        }
    }

    /// Attempts to collect the exit status of the child if it has already
    /// exited.
    ///
    /// This function will not block the calling thread and will only check to
    /// see if the child process has exited or not.
    ///
    /// If the child has exited, then `Ok(Some(status))` is returned. If the
    /// exit status is not available at this time then `Ok(None)` is returned.
    /// If an error occurs, then that error is returned.
    ///
    /// Equivalent to the [`GetExitCodeProcess`][get-exit-code-process]
    /// function.
    ///
    /// Note that this function will call [`CloseHandle`][close-handle] if the
    /// child has exited. If the function fail, a
    /// [`GetExitCodeProcess`][Error::GetExitCodeFailed] error is returned.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use CreateProcessW::Command;
    ///
    /// let mut child = Command::new("notepad.exe").spawn().unwrap();
    ///
    /// match child.try_wait() {
    ///     Ok(Some(status)) => println!("exited with: {}", status.code()),
    ///     Ok(None) => {
    ///         println!("status not ready yet, let's really wait");
    ///         let status = child.wait().expect("cannot wait process");
    ///         println!("waited: {}", status.code());
    ///     }
    ///     Err(e) => println!("error attempting to wait: {}", e),
    /// }
    /// ```
    ///
    /// [close-handle]: https://docs.microsoft.com/en-us/windows/win32/api/handleapi/nf-handleapi-closehandle
    /// [get-exit-code]: https://docs.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-getexitcodeprocess
    ///
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

    /// Returns the process identifier associated with this child.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use CreateProcessW::Command;
    ///
    /// let mut command = Command::new("notepad.exe");
    ///
    /// if let Ok(child) = command.spawn() {
    ///     println!("Child's ID is {}", child.id());
    /// } else {
    ///     println!("notepad didn't start");
    /// }
    /// ```
    pub fn id(&self) -> u32 {
        self.process_information.dwProcessId
    }
}

use windows::Win32::Foundation::CloseHandle;

// This function isn't really necessary, but avoid code repetition
unsafe fn close_handles(process_info: &PROCESS_INFORMATION) {
    CloseHandle(process_info.hProcess);
    CloseHandle(process_info.hThread);
}

/// Describe the result of a process after it has terminated.
///
/// This struct is used to represent the exit status or other termination of a
/// child process. Child processes are created via the [`Command`] struct and
/// their exit status is exposed through the [`status`][Command::status]
/// method, or the [`wait`][Child::wait] method of a [`Child`] process.
pub struct ExitStatus(u32);

impl ExitStatus {
    /// Success is defined as a zero exit status.
    ///
    /// This function return `true` if the `ExitStatus` is zero and `false`
    /// otherwise.
    pub fn success(&self) -> bool {
        self.0 == 0
    }

    /// Returns the exit code of the process
    pub fn code(&self) -> u32 {
        self.0
    }
}

use thiserror::Error;

type Result<T> = std::result::Result<T, Error>;

/// Error type of a failed function on a child process.
///
/// Note that this error type is not the same of a non-zero exit status. The
/// error code linked to the error type is the result of the
/// [`GetLastError`][get-last-error] function. The variants give some context
/// to the user but if you want more information about an error code, you can
/// look at the [`System Error Codes`][system-error-codes].
///
/// [get-last-error]: https://docs.microsoft.com/en-us/windows/win32/api/errhandlingapi/nf-errhandlingapi-getlasterror
/// [system-error-codes]: https://docs.microsoft.com/en-us/windows/win32/debug/system-error-codes
#[derive(Error, Debug)]
pub enum Error {
    /// An error occurred when calling [`CreateProcessW`](https://docs.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-createprocessw)
    #[error("cannot create process (code {:#x})", 0)]
    CreationFailed(u32),

    /// An error occurred when calling [`WaitForSingleObject`](https://docs.microsoft.com/en-us/windows/win32/api/synchapi/nf-synchapi-waitforsingleobject).
    #[error("cannot wait process (code {:#x})", 0)]
    WaitFailed(u32),

    /// An error occurred when calling [`TerminateProcess`](https://docs.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-terminateprocess)
    #[error("cannot kill process (code {:#x})", 0)]
    KillFailed(u32),

    /// An error occurred when calling [`GetExitCodeProcess`](https://docs.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-getexitcodeprocess)
    #[error("cannot get exit status (code {:#x})", 0)]
    GetExitCodeFailed(u32),

    /// An error occurred when calling
    /// [`GetProcessId`](https://docs.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-getprocessid).
    #[error("cannot get process id (code {:#x})", 0)]
    GetProcessIdFailed(u32),
}

impl Error {
    /// Return the system error code of the Error
    ///
    /// This error code isn't formatted like the code in the error string.
    ///
    /// To more information about this [codes][system-error-codes.]
    ///
    /// [system-error-codes]: https://docs.microsoft.com/en-us/windows/win32/debug/system-error-codes
    pub fn code(&self) -> u32 {
        match *self {
            Self::CreationFailed(code) => code,
            Self::WaitFailed(code) => code,
            Self::KillFailed(code) => code,
            Self::GetExitCodeFailed(code) => code,
            Self::GetProcessIdFailed(code) => code,
        }
    }
}
