#![allow(non_snake_case)]

use std::ffi::{c_void, OsStr, OsString};
use std::mem::size_of;
use std::path::{Path, PathBuf};
use thiserror::Error;
use windows::Win32::Foundation::{CloseHandle, GetLastError, PWSTR};
use windows::Win32::Security::SECURITY_ATTRIBUTES;
use windows::Win32::System::Threading::{
    GetExitCodeProcess, TerminateProcess, WaitForSingleObject, PROCESS_CREATION_FLAGS,
    PROCESS_INFORMATION, STARTUPINFOW,
};
use windows::Win32::System::WindowsProgramming::INFINITE;

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot create process: {0}")]
    CreationFailed(u32),
    #[error("cannot wait process: {0}")]
    WaitFailed(u32),
    #[error("cannot kill process: {0}")]
    KillFailed(u32),
}

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct Command {
    command: OsString,
    inherit_handles: Option<bool>,
    current_directory: Option<PathBuf>,
}

impl Command {
    pub fn new(command: impl AsRef<OsStr>) -> Command {
        Command {
            command: command.as_ref().to_owned(),
            inherit_handles: None,
            current_directory: None,
        }
    }

    pub fn inherit(mut self, bool: bool) -> Command {
        self.inherit_handles = Some(bool);
        self
    }

    pub fn current_dir(mut self, dir: impl AsRef<Path>) -> Command {
        self.current_directory = Some(dir.as_ref().to_owned());
        self
    }

    pub fn spawn(&mut self) -> Result<Child> {
        match Child::new(
            &self.command,
            self.inherit_handles.unwrap_or(false),
            self.current_directory.as_ref(),
        ) {
            Ok(child) => Ok(child),
            Err(err) => Err(err),
        }
    }

    pub fn status(&mut self) -> Result<ExitStatus> {
        match self.spawn() {
            Ok(child) => Ok(child.wait()),
            Err(err) => Err(err),
        }
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
        current_directory: Option<&PathBuf>,
    ) -> Result<Self> {
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
                    process_information: pi,
                })
            } else {
                Err(Error::CreationFailed(get_last_error()))
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

    pub fn try_wait(&self) -> Result<Option<ExitStatus>> {
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
            Err(Error::WaitFailed(get_last_error()))
        }
    }

    pub fn kill(&self) -> Result<()> {
        let res = unsafe { TerminateProcess(self.process_information.hProcess, 0) };

        if res.as_bool() {
            Ok(())
        } else {
            Err(Error::KillFailed(get_last_error()))
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

fn get_last_error() -> u32 {
    unsafe { GetLastError().0 }
}
