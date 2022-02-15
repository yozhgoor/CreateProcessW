# CreateProcessW

[![actions status][actions-badge]][actions-url]
[![crate version][crates-version-badge]][crates-url]
[![documentation][docs-badge]][docs-url]
[![dependencies status][deps-badge]][deps-url]
![licenses][licenses-badge]

[actions-badge]: https://github.com/yozhgoor/CreateProcessW/workflows/main/badge.svg
[actions-url]: https://github.com/yozhgoor/CreateProcessW/actions
[crates-version-badge]: https://img.shields.io/crates/v/CreateProcessW
[crates-url]: https://crates.io/crates/CreateProcessW
[docs-badge]: https://docs.rs/CreateProcessW/badge.svg
[docs-url]: https://docs.rs/CreateProcessW
[deps-badge]: https://deps.rs/crate/CreateProcessW/0.1.2/status.svg
[deps-url]: https://deps.rs/crate/CreateProcessW
[licenses-badge]: https://img.shields.io/crates/l/CreateProcessW

<!-- cargo-rdme start -->

This crate provides an API similar to [`std::process`](https://doc.rust-lang.org/stable/std/process/) to create
and handle processes on Windows using the Win32 API through the
[windows-rs][windows-rs] crate (see [this example][create-processes-example]).

Its main difference with `std::process::Command` is that it allows running
a command string instead of having to pass the command executable and the
arguments separately.

This is equivalent of running:

```rust
std::process::Command::new("cmd.exe")
    .arg("/c")
    .arg("any_command_string")
    .spawn().expect("cannot spawn command");
```

The only difference will be that the `Child` instance will use the PID of
the command instead of the PID of `cmd.exe`. This is important because
calling `.kill()` in the code above does not work as it kills the PID
of `cmd.exe` instead of the actual command that has been ran.

# Usage

Add the following to your `Cargo.toml`:

```toml
[dependencies]
CreateProcessW = "0.1.0"
```

This crate doesn't follow Rust's naming recommendations. If you want to stay
consistent with other imported crates, use the following:

```toml
[dependencies]
create_process_w = { version = "0.1.0", package = "CreateProcessW" }
```

# Create a command

The [`Command`](https://docs.rs/CreateProcessW/latest/CreateProcessW/struct.Command.html) struct is used to configure and spawn processes:

```rust
use CreateProcessW::Command;

let command = Command::new("cargo.exe clippy -- -D warnings")
    .inherit_handles(true)
    .current_dir(r"C:\Users\<user>\repos\<repo_name>");
```

## Spawning a process

The [`spawn`](crate::Command::spawn) function spawns the process and returns a
[`Child`](https://docs.rs/CreateProcessW/latest/CreateProcessW/struct.Child.html) that represents the spawned child process.

```rust
use CreateProcessW::Command;

let child = Command::new("notepad.exe")
    .spawn()
    .expect("notepad failed to start");


std::thread::sleep(std::time::Duration::from_secs(2));

child.kill().expect("cannot kill process");
let status = child.wait().expect("cannot wait process");

if status.success() {
    println!("Success!");
} else {
    println!("Process exited with status {}", status.code());
}
```

The [`status`](crate::Command::status) function spawns a child process, waits for
it to finish and returns its [`ExitStatus`](https://docs.rs/CreateProcessW/latest/CreateProcessW/struct.ExitStatus.html).

```rust
use CreateProcessW::Command;

let status = Command::new("notepad.exe")
    .status()
    .expect("notepad failed to start");

if status.success() {
    println!("Success!")
} else {
    println!("Process exited with status {}", status.code())
}
```

[windows-rs]: https://github.com/microsoft/windows-rs
[create-processes-example]: https://docs.microsoft.com/en-us/windows/win32/procthread/creating-processes

<!-- cargo-rdme end -->
