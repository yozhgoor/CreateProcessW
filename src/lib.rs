#![allow(non_snake_case)]

use std::ffi::c_void;
use std::mem::size_of;
use windows::Win32::Foundation::{CloseHandle, GetLastError, PWSTR};
use windows::Win32::Security::SECURITY_ATTRIBUTES;
use windows::Win32::System::Threading::{
    TerminateProcess, WaitForSingleObject, PROCESS_CREATION_FLAGS, PROCESS_INFORMATION,
    STARTUPINFOW,
};
use windows::Win32::System::WindowsProgramming::INFINITE;

#[derive(Debug)]
pub struct ChildProcess {
    command: String,
    process_information: PROCESS_INFORMATION,
}

pub type ChildProcessError = String;

pub struct ExitCode(u32);

impl ChildProcess {
    pub fn new(
        command: &str,
        inherit_handles: bool,
        current_directory: Option<&str>,
    ) -> Result<Self, ChildProcessError> {
        unsafe {
            let mut si = STARTUPINFOW::default();
            let mut pi = PROCESS_INFORMATION::default();

            si.cb = size_of::<STARTUPINFOW>() as u32;

            let process_creation_flags = PROCESS_CREATION_FLAGS(0);

            let res = if let Some(directory) = current_directory {
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
                Err(format!("cannot create process: {:?}", GetLastError()))
            }
        }
    }

    pub fn command(&self) -> String {
        self.command.clone()
    }

    pub fn wait(&self) -> ExitCode {
        unsafe {
            let exit_code = WaitForSingleObject(self.process_information.hProcess, INFINITE);
            close_handle(self.process_information);

            ExitCode(exit_code)
        }
    }

    pub fn kill(&self) -> Result<(), ChildProcessError> {
        unsafe {
            let res = TerminateProcess(self.process_information.hProcess, 0);
            close_handle(self.process_information);

            if res.as_bool() {
                Ok(())
            } else {
                Err(format!("cannot kill process: {:?}", GetLastError()))
            }
        }
    }
}

impl ExitCode {
    pub fn success(&self) -> bool {
        self.0 == 0
    }

    pub fn display(&self) -> u32 {
        self.0
    }
}

fn close_handle(process_information: PROCESS_INFORMATION) {
    unsafe {
        CloseHandle(process_information.hProcess);
        CloseHandle(process_information.hThread);
    }
}
