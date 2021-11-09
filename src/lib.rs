#![allow(non_snake_case)]

use std::ffi::c_void;
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

impl ChildProcess {
    pub fn new(
        command: &str,
        inherit_handles: bool,
        current_directory: Option<&str>,
    ) -> Result<Self, ChildProcessError> {
        unsafe {
            let si = STARTUPINFOW::default();
            let mut pi = PROCESS_INFORMATION::default();

            let process = if let Some(directory) = current_directory {
                windows::Win32::System::Threading::CreateProcessW(
                    PWSTR::default(),
                    command,
                    std::ptr::null() as *const SECURITY_ATTRIBUTES,
                    std::ptr::null() as *const SECURITY_ATTRIBUTES,
                    inherit_handles,
                    PROCESS_CREATION_FLAGS(0),
                    std::ptr::null() as *const c_void,
                    directory,
                    &si,
                    &mut pi as *mut PROCESS_INFORMATION,
                )
                .as_bool()
            } else {
                windows::Win32::System::Threading::CreateProcessW(
                    PWSTR::default(),
                    command,
                    std::ptr::null() as *const SECURITY_ATTRIBUTES,
                    std::ptr::null() as *const SECURITY_ATTRIBUTES,
                    inherit_handles,
                    PROCESS_CREATION_FLAGS(0),
                    std::ptr::null() as *const c_void,
                    PWSTR::default(),
                    &si,
                    &mut pi as *mut PROCESS_INFORMATION,
                )
                .as_bool()
            };

            if process {
                Ok(Self {
                    command: command.to_string(),
                    process_information: pi,
                })
            } else {
                Err(format!("Cannot create process: {:?}", GetLastError()))
            }
        }
    }

    pub fn command(&self) -> String {
        self.command.clone()
    }

    pub fn wait(&self) {
        unsafe {
            WaitForSingleObject(self.process_information.hProcess, INFINITE);
            close_handle(self.process_information);
        }
    }

    pub fn kill(&self) -> Result<(), ChildProcessError> {
        unsafe {
            if TerminateProcess(self.process_information.hProcess, 0).as_bool() {
                close_handle(self.process_information);

                Ok(())
            } else {
                Err(String::from("an error occurred when killing the process"))
            }
        }
    }
}

fn close_handle(process_information: PROCESS_INFORMATION) {
    unsafe {
        CloseHandle(process_information.hProcess);
        CloseHandle(process_information.hThread);
    }
}
