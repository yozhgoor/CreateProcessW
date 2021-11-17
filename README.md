# CreateProcessW

This crate provide an API similar to [`std::process`][std-process] to create
and handle processes on Windows using the Win32 API through the
[windows-rs][windows-rs] crate (see [this example][create-processes-example]).

[std-process]: https://doc.rust-lang.org/std/process/index.html
[windows-rs]: https://github.com/microsoft/windows-rs
[create-processes-example]: https://docs.microsoft.com/en-us/windows/win32/procthread/creating-processes

It's main difference with `std::process::Command` is that it allows running
a command string instead of having to pass the command executable and the
arguments separately.

This is equivalent of running:

```rust
std::process::Command("cmd.exe")
    .arg("/c")
    .arg(any_command_string)
    .spawn()
```

The only difference will be that the `Child` instance will use the PID of
the command instead of the PID of `cmd.exe`. This is important because
calling `.kill()` in the code above does not work as it kills the PID
of `cmd.exe` instead of the actual command that has been ran.

## Usage

Add the following to your `Cargo.toml`:

```toml
[dependencies]
create_process_w = { version = "0.1.0", package = "CreateProcessW" }
```

You can also use `CreateProcessW` directly, but this doesn't respect Rust's
naming recommendations.

## Create a command

The [`Command`] struct is used to configure and spawn processes:

```rust
use create_process_w::Command;

let command = Command::new("cargo.exe check")
    .inherit_handle(false)
    .current_directory(r"C:\Users\<user>\repos\<repo_name>");
```

### Spawning a process

The [`spawn`][Command::spawn] function spawn the process and return a [`Child`] that
represents the spawned child process.

```rust
use create_process_w::Command;

let child = Command::new("notepad.exe")
    .spawn()
    .expect("notepad failed to start");


std::thread::Duration(std::time::Duration::from_secs(2));

child.kill().expect("cannot kill process");
let status = child.wait().expect("cannot wait process");

if status.success() {
    println!("Success!");
} else {
    println!("Process exited with status {}", status.code());
}
```

The [`status`][Command::status] function spawn a child process, wait for it to finish and
return its [`ExitStatus`].

```rust
use create_process_w::Command;

let status = Command::new("notepad.exe")
    .status()
    .expect("notepad failed to start");

if status.success() {
    println!("Success!")
} else {
    println!("Process exited with status {}", status.code())
}
```

[`Command`]: https://docs.rs/CreateProcessW/latest/CreateProcessW/struct.Command.html
[`Child`]: https://docs.rs/CreateProcessW/latest/CreateProcessW/struct.Child.html
[Command::spawn]: https://docs.rs/CreateProcessW/latest/CreateProcessW/struct.Command.html#method.spawn
[Command::status]: https://docs.rs/CreateProcessW/latest/CreateProcessW/struct.Command.html#method.status
