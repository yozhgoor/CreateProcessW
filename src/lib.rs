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
                Err(ChildProcessError::CreationFailed(get_last_error()))
            }
        }
    }

    pub fn wait(&self) -> ExitStatus {
        let mut exit_code: u32 = 0;

        unsafe {
            WaitForSingleObject(self.process_information.hProcess, INFINITE);
            GetExitCodeProcess(
                self.process_information.hProcess,
                &mut exit_code as *mut u32,
            );

            CloseHandle(self.process_information.hProcess);
            CloseHandle(self.process_information.hThread);
        }

        ExitStatus(exit_code)
    }

    pub fn try_wait(&self) -> Result<Option<ExitStatus>, ExitStatusError> {
        let mut exit_code: u32 = 0;

        let res = unsafe {
            GetExitCodeProcess(
                self.process_information.hProcess,
                &mut exit_code as *mut u32,
            )
        };

        if res.as_bool() {
            match exit_code {
                259 => Ok(None),
                _ => Ok(Some(ExitStatus(exit_code))),
            }
        } else {
            Err(ExitStatusError::WaitFailed(get_last_error()))
        }
    }

    pub fn kill(&self) -> Result<(), ChildProcessError> {
        let res = unsafe { TerminateProcess(self.process_information.hProcess, 0) };

        if res.as_bool() {
            Ok(())
        } else {
            Err(ChildProcessError::KillFailed(get_last_error()))
        }
    }
}

#[derive(Error, Debug)]
pub enum ChildProcessError {
    #[error("cannot create process: {0}")]
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
    #[error("cannot wait process: {0}")]
    WaitFailed(String),
}

fn get_last_error() -> String {
    unsafe { format!("{:?}", GetLastError()) }
}
