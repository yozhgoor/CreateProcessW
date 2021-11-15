# CreateProcessW

This crate provide an API similar to `std::process` to create and handle process
using the [windows-rs][windows-rs] crate and the Windows API (see [this example][creating-processes]).

# Usage

Add the following to your `Cargo.toml`:

```toml
[dependencies]
CreateProcessW = "0.1.0"
```

This crate doesn't follow Rust's naming convention, if you want to avoid the
`non_snake_case` name, you can use the following:

```toml
[dependencies]
create_process_w = { version = "0.1.0", package = "CreateProcessW" }
```

# Create a new Command

To configure and create a new process, you can use the `Command` struct. The
only argument needed is the command line you want to execute.

```rust
use CreateProcess::Command;

let command = Command::new("ls");
```

## Customization

You can customize the command with some methods:

* `inherit_handles` - If this parameter is TRUE, each inheritable handle in the
    calling process is inherited by the new process. If the parameter is FALSE,
    the handles are not inherited. Note that inherited handles have the same
    value and access rights as the original handles. The default value is
    `true`.

* `current_dir` - The full path to the current directory for the process. If you
    don't provide a value, the new process will have the same current drive and
    directory as the calling process.

Example:

```rust
use CreateProcessW::Command;

let command = Command::new("ls")
    .inherit_handles(false)
    .current_dir("C:\\Users\\user\\repos");
```

# Execute the command

This library give you two way to execute a command, `spawn` return a handle to
the child process and `status` wait for it to finish and collect its status.

## Spawning a process

The `spawn` method start a process and return a [`Child`] struct that allows to
handle the process. `Child` provide several methods to do this:

```rust
use CreateProcessW::Command;

let child = Command::new("notepad.exe").spawn().expect("cannot spawn notepad");

if let Some(status) = child.try_wait().expect("waiting process failed") {
    println!("Process exited with status code {}", status.status());
} else {
    println!("Process is running");
}

child.kill().expect("cannot kill process");
child.wait().expect("cannot wait process");
```

The `try_wait` method try to get the `ExitStatus` of the process. If the process
is running, this method return `None`, if the process is terminated this method
return `Some(ExitStatus)`.


The `kill` method terminate the process.

The `wait` method wait the end of the process and return an `ExitStatus`, it
also clean up the process by closing the handles.

## Get the exit status of the process

If you don't need to handle the child process, you can use `status`.

```rust
use CreateProcessW::Command;

let status = Command::new("notepad.exe").status();

if status.success() {
    println!("Success!");
} else {
    println!("Process exited with status code {}", status.code());
}
```

This method launch the process, waiting for it to finish, closing the handles
and return the exit status of the process.

[windows-rs]: https://github.com/microsoft/windows-rs
[creating-processes]: https://docs.microsoft.com/en-us/windows/win32/procthread/creating-processes
