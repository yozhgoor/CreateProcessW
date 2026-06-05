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
    env,
    ffi::{c_void, OsStr, OsString},
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
    CREATE_UNICODE_ENVIRONMENT, DWORD, INFINITE, PCWSTR, PDWORD, PROCESS_INFORMATION, PWSTR,
    SECURITY_ATTRIBUTES, STARTUPINFOW, STATUS_PENDING, UINT, WAIT_OBJECT_0,
};

/// A process builder, providing control over how a new process should be
/// spawned.
#[derive(Debug)]
pub struct Command {
    command: OsString,
    inherit_handles: bool,
    current_directory: Option<PathBuf>,
    env_clear: bool,
    env_vars: Vec<(OsString, Option<OsString>)>,
}

impl Command {
    /// Create a new [`Command`], with the following default configuration:
    ///
    /// * Do not Inherit handles of the calling process.
    /// * Inherit the current drive and directory of the calling process.
    /// * Inherit the environment of the calling process.
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
            env_clear: false,
            env_vars: Vec::new(),
        }
    }

    /// Enable/disable handle inheritance.
    ///
    /// When `true`, two things happen:
    /// 1. Each inheritable handle in the calling process is inherited by the
    ///    new process (maps to `bInheritHandles`).
    /// 2. The process and thread handles returned in [`Child`] are themselves
    ///    marked as inheritable, so *future* child processes can inherit them
    ///    too (maps to `bInheritHandle` on `lpProcessAttributes` / `lpThreadAttributes`).
    ///
    /// When `false`, neither happens.
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

    /// Inserts or updates an environment variable mapping.
    ///
    /// When inheriting the parent's environment (the default), the last call
    /// to `env` for a given key wins. Earlier calls with the same key are
    /// overridden.
    ///
    /// A key should not contain ASCII `=` or a NUL byte.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use CreateProcessW::Command;
    ///
    /// Command::new("cmd.exe /c echo %MY_VAR%")
    ///     .env("MY_VAR", "hello")
    ///     .spawn()
    ///     .expect("failed to execute process");
    /// ```
    pub fn env<K, V>(&mut self, key: K, val: V) -> &mut Self
    where
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        self.env_vars.push((
            key.as_ref().to_os_string(),
            Some(val.as_ref().to_os_string()),
        ));
        self
    }

    /// Adds or updates multiple environment variable mappings.
    ///
    /// Works like repeated calls to [`env`](Command::env). The last
    /// occurrence of a duplicate key wins.
    pub fn envs<I, K, V>(&mut self, vars: I) -> &mut Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        for (key, val) in vars {
            self.env(key, val);
        }
        self
    }

    /// Removes an environment variable from the inherited environment.
    ///
    /// If [`env_clear`](Command::env_clear) is called *before* this method,
    /// removal is a no-op at that point (the child already starts with an
    ///  empty environment). If [`env_clear`](Command::env_clear) is called
    /// *after* this method, the removal is erased along with all other
    /// environment configuration.
    ///
    /// Note: The last operation on a key wins. Calling `env_remove` after
    /// [`env`](Command::env) for the same key removes it and calling
    /// [`env`](Command::env) after `env_remove` sets it.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use CreateProcessW::Command;
    ///
    /// Command::new("cmd.exe /c set")
    ///     .env_remove("PATH")
    ///     .spawn()
    ///     .expect("failed to execute process");
    /// ```
    pub fn env_remove<K>(&mut self, key: K) -> &mut Self
    where
        K: AsRef<OsStr>,
    {
        self.env_vars.push((key.as_ref().to_os_string(), None));
        self
    }

    /// Clears the entire environment map for the child process.
    ///
    /// The child will **not** inherit any environment variables from the
    /// parent process. Only variables added with [`env`](Command::env) or
    /// [`envs`](Command::envs) will be present.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use CreateProcessW::Command;
    ///
    /// Command::new("cmd.exe /c set")
    ///     .env_clear()
    ///     .env("MY_VAR", "hello")
    ///     .spawn()
    ///     .expect("failed to execute process");
    /// ```
    pub fn env_clear(&mut self) -> &mut Self {
        self.env_clear = true;
        self.env_vars.clear();
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
            self.env_clear,
            std::mem::take(&mut self.env_vars),
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
        env_clear: bool,
        env_vars: Vec<(OsString, Option<OsString>)>,
    ) -> Result<Self, Error> {
        let mut startup_information = STARTUPINFOW::default();
        let mut process_information = PROCESS_INFORMATION::default();

        startup_information.cb = size_of::<STARTUPINFOW>() as u32;

        let env_block = build_env_block(env_clear, env_vars);
        let lp_env_ptr = env_block
            .as_ref()
            .map(|b| b.as_ptr() as *mut c_void)
            .unwrap_or(null_mut());

        let process_creation_flags = if lp_env_ptr.is_null() {
            0
        } else {
            CREATE_UNICODE_ENVIRONMENT
        };

        // Skip allocation when `inherit_handles` is false.
        let mut security_attributes;
        let (lp_process_attributes, lp_thread_attributes) = if inherit_handles {
            security_attributes = SECURITY_ATTRIBUTES::new(true);
            (
                &mut security_attributes as *mut SECURITY_ATTRIBUTES,
                &mut security_attributes as *mut SECURITY_ATTRIBUTES,
            )
        } else {
            (null_mut(), null_mut())
        };

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
                lp_process_attributes,
                lp_thread_attributes,
                inherit_handles as BOOL,
                process_creation_flags as DWORD,
                lp_env_ptr,
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

/// Builds a Unicode environment block.
///
/// Returns `None` if the child should inherit the parent's environment
/// (i.e. `lpEnvironment` should be `NULL`).
///
/// Otherwise returns a `Vec<u16>` containing the double-null-terminated
/// block in the format:
/// ```text
/// K\0E\0Y\0=\0V\0A\0L\0\0\0
/// K\0E\0Y\0=\0V\0A\0L\0\0\0
/// \0\0
/// ```
fn build_env_block(
    env_clear: bool,
    env_vars: Vec<(OsString, Option<OsString>)>,
) -> Option<Vec<u16>> {
    fn ascii_lower_wide(s: &OsStr) -> impl Iterator<Item = u16> + '_ {
        s.encode_wide().map(|c| {
            if (b'A' as u16..=b'Z' as u16).contains(&c) {
                c + 32
            } else {
                c
            }
        })
    }

    fn eq_ignore_ascii_case(a: &OsStr, b: &OsStr) -> bool {
        ascii_lower_wide(a).eq(ascii_lower_wide(b))
    }

    if !env_clear && env_vars.is_empty() {
        return None;
    }

    let mut map: Vec<(OsString, OsString)> = if env_clear {
        Vec::new()
    } else {
        env::vars_os().collect()
    };

    let mut seen: Vec<OsString> = Vec::new();
    for (key, val) in env_vars.into_iter().rev() {
        if seen.iter().any(|k| eq_ignore_ascii_case(k, &key)) {
            continue;
        }
        seen.push(key.clone());
        map.retain(|(k, _)| !eq_ignore_ascii_case(k, &key));
        if let Some(val) = val {
            map.push((key, val));
        }
    }

    let mut pairs: Vec<_> = map
        .drain(..)
        .map(|(key, val)| {
            let lowered: Vec<u16> = ascii_lower_wide(&key).collect();
            (lowered, key, val)
        })
        .collect();
    pairs.sort_by(|(a, _, _), (b, _, _)| a.cmp(b));
    map = pairs.into_iter().map(|(_, key, val)| (key, val)).collect();

    let mut block: Vec<u16> = Vec::new();
    for (key, val) in &map {
        block.extend(key.encode_wide());
        block.push(b'=' as u16);
        block.extend(val.encode_wide());
        block.push(0);
    }

    if map.is_empty() {
        block.extend(&[0, 0]);
    } else {
        block.push(0);
    }

    Some(block)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binding::{GetHandleInformation, HANDLE_FLAG_INHERIT};

    /// Returns true if the handle has HANDLE_FLAG_INHERIT set.
    unsafe fn is_inheritable(handle: *mut c_void) -> bool {
        let mut flags: DWORD = 0;
        let ok = GetHandleInformation(handle, &mut flags) != 0;
        assert!(ok, "GetHandleInformation failed");
        (flags & HANDLE_FLAG_INHERIT) != 0
    }

    #[test]
    fn default_spawn_does_not_give_inheritable_handles() {
        let child = Command::new("cmd.exe /c exit 0").spawn().unwrap();
        let hproc = child.process_information.hProcess;
        let hthread = child.process_information.hThread;
        // Check flags before waiting, since wait() closes the handles.
        let proc_inheritable = unsafe { is_inheritable(hproc) };
        let thread_inheritable = unsafe { is_inheritable(hthread) };
        child.wait().unwrap();
        assert!(
            !proc_inheritable,
            "process handle should NOT be inheritable by default"
        );
        assert!(
            !thread_inheritable,
            "thread handle should NOT be inheritable by default"
        );
    }

    #[test]
    fn inherit_handles_true_gives_inheritable_handles() {
        let child = Command::new("cmd.exe /c exit 0")
            .inherit_handles(true)
            .spawn()
            .unwrap();
        let hproc = child.process_information.hProcess;
        let hthread = child.process_information.hThread;
        let proc_inheritable = unsafe { is_inheritable(hproc) };
        let thread_inheritable = unsafe { is_inheritable(hthread) };
        child.wait().unwrap();
        assert!(
            proc_inheritable,
            "process handle should be inheritable when inherit_handles(true)"
        );
        assert!(
            thread_inheritable,
            "thread handle should be inheritable when inherit_handles(true)"
        );
    }

    #[test]
    fn env_var_is_passed_to_child() {
        let child =
            Command::new(r#"cmd.exe /c "if "%MY_VAR%"=="hello_test" (exit 0) else (exit 1)""#)
                .env("MY_VAR", "hello_test")
                .spawn()
                .unwrap();

        let status = child.wait().unwrap();
        assert_eq!(status.code(), 0, "MY_VAR should be 'hello_test'");
    }

    #[test]
    fn env_clear_with_single_var() {
        let child = Command::new(
            r#"cmd.exe /c "if defined PATH (exit 1) else (if "%CUSTOM%"=="value" (exit 0) else (exit 2))""#,
        )
        .env_clear()
        .env("CUSTOM", "value")
        .spawn()
        .unwrap();

        let status = child.wait().unwrap();
        assert_eq!(
            status.code(),
            0,
            "PATH should be unset and CUSTOM should be 'value'"
        );
    }

    #[test]
    fn env_remove_removes_var() {
        let child = Command::new("cmd.exe /c \"if defined PATH (exit 1) else (exit 0)\"")
            .env_remove("PATH")
            .spawn()
            .unwrap();

        let status = child.wait().unwrap();
        assert_eq!(status.code(), 0);
    }

    #[test]
    fn no_env_args_inherits_parent() {
        let child = Command::new("cmd.exe /c \"if defined PATH (exit 0) else (exit 1)\"")
            .spawn()
            .unwrap();

        let status = child.wait().unwrap();
        assert_eq!(status.code(), 0);
    }

    #[test]
    fn last_duplicate_key_wins() {
        let child = Command::new(
            r#"cmd.exe /c "if "%MY_VAR%"=="second" (exit 2) else (if "%MY_VAR%"=="first" (exit 0) else (exit 3))""#,
        )
        .env("MY_VAR", "first")
        .env("MY_VAR", "second")
        .spawn()
        .unwrap();

        let status = child.wait().unwrap();
        assert_eq!(
            status.code(),
            2,
            "duplicate key should keep the last value 'second'"
        );
    }

    #[test]
    fn last_duplicate_key_wins_case_insensitive() {
        let child = Command::new(
            r#"cmd.exe /c "if "%MYVAR%"=="second" (exit 2) else (if "%MYVAR%"=="first" (exit 0) else (exit 3))""#,
        )
        .env("MyVar", "first")
        .env("MYVAR", "second")
        .spawn()
        .unwrap();

        let status = child.wait().unwrap();
        assert_eq!(
            status.code(),
            2,
            "case-insensitive duplicate key should keep the last value 'second'"
        );
    }

    #[test]
    fn env_overrides_earlier_remove() {
        let child =
            Command::new(r#"cmd.exe /c "if "%MY_VAR%"=="hello_override" (exit 0) else (exit 1)""#)
                .env_remove("MY_VAR")
                .env("MY_VAR", "hello_override")
                .spawn()
                .unwrap();

        let status = child.wait().unwrap();
        assert_eq!(status.code(), 0, "env should override earlier env_remove");
    }

    #[test]
    fn remove_overrides_earlier_env() {
        let child = Command::new(r#"cmd.exe /c "if defined MY_VAR (exit 1) else (exit 0)""#)
            .env("MY_VAR", "some_value")
            .env_remove("MY_VAR")
            .spawn()
            .unwrap();

        let status = child.wait().unwrap();
        assert_eq!(status.code(), 0, "env_remove should override earlier env");
    }
}
