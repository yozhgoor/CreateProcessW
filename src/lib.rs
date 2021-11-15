// Disable warning for `non_snake_case` in the crate.
// It's not the better way to disable this warning only for the crate name.
// See https://github.com/rust-lang/rust/issues/45127
#![allow(non_snake_case)]

use std::ffi::{c_void, OsStr, OsString};
use std::mem::size_of;
use std::path::{Path, PathBuf};
use thiserror::Error;
use windows::Win32::Foundation::{CloseHandle, GetLastError, PWSTR, STATUS_PENDING};
use windows::Win32::Security::SECURITY_ATTRIBUTES;
use windows::Win32::System::Threading::{
    GetExitCodeProcess, TerminateProcess, WaitForSingleObject, PROCESS_CREATION_FLAGS,
    PROCESS_INFORMATION, STARTUPINFOW, WAIT_OBJECT_0,
};
use windows::Win32::System::WindowsProgramming::INFINITE;

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot create process: {0}")]
    CreationFailed(u32),
    #[error("cannot get exit status: {0}")]
    GetExitCodeFailed(u32),
    #[error("cannot kill process: {0}")]
    KillFailed(u32),
}

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct Command {
    command: OsString,
    inherit_handles: bool,
    current_directory: Option<PathBuf>,
}

impl Command {
    pub fn new(command: impl Into<OsString>) -> Self {
        Self {
            command: command.into(),
            inherit_handles: true,
            current_directory: None,
        }
    }

    pub fn inherit_handles(&mut self, inherit: bool) -> &mut Self {
        self.inherit_handles = inherit;
        self
    }

    pub fn current_dir(&mut self, dir: impl Into<PathBuf>) -> &mut Self {
        self.current_directory = Some(dir.into());
        self
    }

    pub fn spawn(&mut self) -> Result<Child> {
        Child::new(
            &self.command,
            self.inherit_handles,
            self.current_directory.as_deref(),
        )
    }

    pub fn status(&mut self) -> Result<ExitStatus> {
        self.spawn()?.wait()
    }
}

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

unsafe fn close_handles(process_info: &PROCESS_INFORMATION) {
    CloseHandle(process_info.hProcess);
    CloseHandle(process_info.hThread);
}
