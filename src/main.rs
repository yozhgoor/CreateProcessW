use windows::Win32::Security::SECURITY_ATTRIBUTES;
use windows::Win32::System::Threading::{PROCESS_CREATION_FLAGS, PROCESS_INFORMATION, STARTUPINFOW, TerminateProcess, WaitForSingleObject};
use windows::Win32::Foundation::{PWSTR, CloseHandle, GetLastError};
use windows::Win32::System::WindowsProgramming::INFINITE;
use std::os::raw::c_void;

fn main() {
    let si = STARTUPINFOW::default();
    let mut pi = PROCESS_INFORMATION::default();

    let kill = false;
    
    unsafe {
        if windows::Win32::System::Threading::CreateProcessW(
            PWSTR::default(),
            "notepad.exe",
            std::ptr::null() as *const SECURITY_ATTRIBUTES,
            std::ptr::null() as *const SECURITY_ATTRIBUTES,
            false,
            PROCESS_CREATION_FLAGS(0),
            std::ptr::null() as *const c_void,
            PWSTR::default(), 
            &si,
            &mut pi as *mut PROCESS_INFORMATION,
        ).as_bool() {   
            if kill {
                std::thread::sleep(std::time::Duration::from_secs(2));

                TerminateProcess( pi.hProcess, 0 );
            } else {
                WaitForSingleObject( pi.hProcess, INFINITE );
            }

            CloseHandle (pi.hProcess);
            CloseHandle (pi.hThread);

            std::thread::sleep(std::time::Duration::from_secs(2));

            println!("Success");
        } else {
            println!("failed {:?}", GetLastError());
        }
    }
}


// fn create_process(command: String, foreground: bool, keep_on_exit: bool, working_dir: Option<PathBuf>, stdout: bool, stderr: bool)