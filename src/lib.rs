#![allow(non_snake_case)]

use std::ffi::c_void;
use std::mem::size_of;
use std::path::Path;
use thiserror::Error;
use windows::Win32::Foundation::{CloseHandle, GetLastError, PWSTR};
use windows::Win32::Security::SECURITY_ATTRIBUTES;
use windows::Win32::System::Threading::{
    GetExitCodeProcess, TerminateProcess, WaitForSingleObject, PROCESS_CREATION_FLAGS,
    PROCESS_INFORMATION, STARTUPINFOW,
};
use windows::Win32::System::WindowsProgramming::INFINITE;

#[derive(Debug)]
pub struct ChildProcess {
    command: String,
    process_information: PROCESS_INFORMATION,
}

impl ChildProcess {
    pub fn new(
        command: &str,
        inherit_handles: bool,
        current_directory: Option<&Path>,
    ) -> Result<Self, ChildProcessError> {
        unsafe {
            let mut si = STARTUPINFOW::default();
            let mut pi = PROCESS_INFORMATION::default();

            si.cb = size_of::<STARTUPINFOW>() as u32;

            let process_creation_flags = PROCESS_CREATION_FLAGS(0);

            let res = if let Some(directory) = current_directory {
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
                    &si,
                    &mut pi as *mut PROCESS_INFORMATION,
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
                    &si,
                    &mut pi as *mut PROCESS_INFORMATION,
                )
            };

            if res.as_bool() {
                Ok(Self {
                    command: command.to_string(),
                    process_information: pi,
                })
            } else {
                Err(ChildProcessError::CreationFailed(format!(
                    "{:?}",
                    GetLastError()
                )))
            }
        }
    }

    pub fn wait(&self) -> ExitStatus {
        unsafe {
            let exit_code = WaitForSingleObject(self.process_information.hProcess, INFINITE);
            CloseHandle(self.process_information.hProcess);
            CloseHandle(self.process_information.hThread);

            ExitStatus(exit_code)
        }
    }

    pub fn try_wait(&self) -> Result<Option<ExitStatus>, ExitStatusError> {
        let mut exit_code: u32 = 0;
        unsafe {
            if GetExitCodeProcess(
                self.process_information.hProcess,
                &mut exit_code as *mut u32,
            )
            .as_bool()
            {
                match exit_code {
                    259 => Ok(None),
                    _ => Ok(Some(ExitStatus(exit_code))),
                }
            } else {
                Err(ExitStatusError::WaitFailed(format!("{:?}", GetLastError())))
            }
        }
    }

    pub fn kill(&self) -> Result<(), ChildProcessError> {
        unsafe {
            if TerminateProcess(self.process_information.hProcess, 0).as_bool() {
                Ok(())
            } else {
                Err(ChildProcessError::KillFailed(format!(
                    "{:?}",
                    GetLastError()
                )))
            }
        }
    }
}

#[derive(Error, Debug)]
pub enum ChildProcessError {
    #[error("cannot create childprocess: {0}")]
    CreationFailed(String),
    #[error("cannot kill process: {0}")]
    KillFailed(String),
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

#[derive(Error, Debug)]
pub enum ExitStatusError {
    #[error("cannot wait: {0}")]
    WaitFailed(String),
}
