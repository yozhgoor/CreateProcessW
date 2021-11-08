use std::ffi::c_void;
use windows::Win32::Foundation::{CloseHandle, GetLastError, PWSTR};
use windows::Win32::Security::SECURITY_ATTRIBUTES;
use windows::Win32::System::Threading::{
    TerminateProcess, WaitForSingleObject, PROCESS_CREATION_FLAGS, PROCESS_INFORMATION,
    STARTUPINFOW,
};
use windows::Win32::System::WindowsProgramming::INFINITE;

pub struct ChildProcess {
    process_information: PROCESS_INFORMATION,
}

pub type ChildProcessError = String;

impl ChildProcess {
    pub fn new(command: &str) -> Result<Self, ChildProcessError> {
        match create_process(command) {
            Ok(process_information) => Ok(Self { process_information }),
            Err(err) => Err(format!("an error occurred when creating process : {}", err)),
        }
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

fn create_process(command: &str) -> Result<PROCESS_INFORMATION, ChildProcessError> {
    unsafe {
        let si = STARTUPINFOW::default();
        let mut pi = PROCESS_INFORMATION::default();

        if windows::Win32::System::Threading::CreateProcessW(
            PWSTR::default(),
            command,
            std::ptr::null() as *const SECURITY_ATTRIBUTES,
            std::ptr::null() as *const SECURITY_ATTRIBUTES,
            false,
            PROCESS_CREATION_FLAGS(0),
            std::ptr::null() as *const c_void,
            PWSTR::default(),
            &si,
            &mut pi as *mut PROCESS_INFORMATION,
        ).as_bool() {
            Ok(pi)
        } else {
            Err(format!("{:?}", GetLastError()))
        }
    }
}

fn close_handle(process_information: PROCESS_INFORMATION) {
    unsafe {
        CloseHandle(process_information.hProcess);
        CloseHandle(process_information.hThread);
    }
}
