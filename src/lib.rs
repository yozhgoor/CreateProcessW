// Disable warning for `non_snake_case` in the crate and when the lib is used as
// a dependency. It's not the better way to disable this warning only for the
// crate name. See https://github.com/rust-lang/rust/issues/45127
#![allow(non_snake_case)]
#![deny(missing_docs)]

//! This crate provides an API similar to [`std::process`](::std::process) to create
//! and handle processes on Windows using the Win32 API (see [this example][create-processes-example]).
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
//! The [`Command`](crate::Command) struct is used to configure and spawn processes:
//!
//! ```no_run
//! use CreateProcessW::Command;
//!
//! let command = Command::new("cargo.exe clippy -- -D warnings")
//!     .inherit_handles(true)
//!     .current_dir(r"C:\Users\<user>\repos\<repo_name>");
//! ```
//!
//! ## Spawning a process
//!
//! The [`spawn`](crate::Command) function spawns the process and returns a
//! [`Child`](crate::Child) that represents the spawned child process.
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
//! The [`status`](crate::Command) function spawns a child process, waits for
//! it to finish and returns its [`ExitStatus`](crate::ExitStatus).
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
//! [create-processes-example]: https://docs.microsoft.com/en-us/windows/win32/procthread/creating-processes

mod binding;

use std::{
    ffi::{OsStr, OsString},
    fmt,
    io::Error,
    iter::once,
    mem::size_of,
    os::windows::ffi::OsStrExt,
    path::{Path, PathBuf},
    ptr::{null, null_mut},
};

use crate::binding::{
    CloseHandle, CreateProcessW, GetExitCodeProcess, TerminateProcess, WaitForSingleObject, BOOL,
    DWORD, INFINITE, PCWSTR, PDWORD, PROCESS_INFORMATION, PWSTR, STARTUPINFOW, STATUS_PENDING,
    UINT, WAIT_OBJECT_0,
};

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
            inherit_handles: false,
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
    ///     .current_dir(r"C:\Users\<user>\repos\<repo_name>")
    ///     .status()
    ///     .expect("cargo check command failed");
    /// ```
    ///
    /// Equivalent to the `lpCurrentDirectory` parameter of the
    /// [`CreateProcessW`][create-process-w-parameters] function.
    ///
    /// [create-process-w-parameters]: https://docs.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-createprocessw#parameters
    pub fn current_dir(&mut self, dir: impl Into<PathBuf>) -> &mut Self {
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
    pub fn spawn(&mut self) -> Result<Child, Error> {
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
    pub fn status(&mut self) -> Result<ExitStatus, Error> {
        self.spawn()?.wait()
    }
}

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
    // Create a new process and initialize it's memory. If it cannot be
    // created, an [`CreateFailed`][Error::CreateFailed] error is returned.
    //
    // Equivalent to [`CreateProcessW`][create-process-w]
    //
    // [create-process-w]: https://docs.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-createprocessw
    fn new(
        command: &OsStr,
        inherit_handles: bool,
        current_directory: Option<&Path>,
    ) -> Result<Self, Error> {
        let mut startup_information = STARTUPINFOW::default();
        let mut process_information = PROCESS_INFORMATION::default();

        startup_information.cb = size_of::<STARTUPINFOW>() as u32;

        let process_creation_flags = 0 as DWORD;

        let current_directory_ptr = current_directory
            .map(|path| {
                let wide_path: Vec<u16> = path.as_os_str().encode_wide().chain(once(0)).collect();

                wide_path.as_ptr()
            })
            .unwrap_or(std::ptr::null_mut());

        // Convert command to a wide string with a null terminator.
        let command = command.encode_wide().chain(once(0)).collect::<Vec<_>>();

        let res = unsafe {
            CreateProcessW(
                null(),
                command.as_ptr() as PWSTR,
                null_mut(),
                null_mut(),
                inherit_handles as BOOL,
                process_creation_flags as DWORD,
                null_mut(),
                current_directory_ptr as PCWSTR,
                &startup_information,
                &mut process_information,
            )
        };

        if res != 0 {
            Ok(Self {
                process_information,
            })
        } else {
            Err(Error::last_os_error())
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
    pub fn kill(&self) -> Result<(), Error> {
        let res = unsafe { TerminateProcess(self.process_information.hProcess, 0 as UINT) };

        if res != 0 {
            Ok(())
        } else {
            Err(Error::last_os_error())
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
    pub fn wait(&self) -> Result<ExitStatus, Error> {
        let mut exit_code = 0;

        let wait = unsafe {
            WaitForSingleObject(self.process_information.hProcess, INFINITE) == WAIT_OBJECT_0
        };

        if wait {
            let res = unsafe {
                GetExitCodeProcess(self.process_information.hProcess, &mut exit_code as PDWORD)
            };

            if res != 0 {
                unsafe {
                    CloseHandle(self.process_information.hProcess);
                    CloseHandle(self.process_information.hThread);
                }

                Ok(ExitStatus(exit_code))
            } else {
                Err(Error::last_os_error())
            }
        } else {
            Err(Error::last_os_error())
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
    pub fn try_wait(&self) -> Result<Option<ExitStatus>, Error> {
        let mut exit_code = 0;

        let res = unsafe {
            GetExitCodeProcess(self.process_information.hProcess, &mut exit_code as PDWORD)
        };

        if res != 0 {
            if exit_code == STATUS_PENDING {
                Ok(None)
            } else {
                unsafe {
                    CloseHandle(self.process_information.hProcess);
                    CloseHandle(self.process_information.hThread);
                }

                Ok(Some(ExitStatus(exit_code)))
            }
        } else {
            Err(Error::last_os_error())
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

/// Describes the result of a process after it has terminated.
///
/// This struct is used to represent the exit status or other termination of a
/// child process. Child processes are created via the [`Command`] struct and
/// their exit status is exposed through the [`status`][Command::status]
/// method, or the [`wait`][Child::wait] method of a [`Child`] process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

impl fmt::Display for ExitStatus {
    /// Formats the value using the given formatter.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
