fn main() {
    let wait = false;

    let process = match CreateProcessW::ChildProcess::new("notepad.exe") {
        Ok(child) => child,
        Err(err) => {
            panic!("An error occurred: {}", err);
        }
    };

    if wait {
        process.wait();
    } else {
        std::thread::sleep(std::time::Duration::from_secs(2));

        match process.kill() {
            Ok(_) => {}
            Err(err) => println!("{}", err),
        }
    }
}
