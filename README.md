# CreateProcessW

This crate provide an API similar to `std::process` to create and handle
processes on Windows using the Win32 API through the [windows-rs][windows-rs]
crate (see [this example][creating-processes]).

It's main difference with `std::process::Command` is that it allows running a
command string instead of having to pass the command executable and the
arguments separately.

This is the equivalent of running:

```rust
std::process::Command("cmd.exe")
    .arg("/c")
    .arg(any_command_string)
    .spawn()
```

The only difference will be that the `Child` instance will use the PID of the
command instead of the PID of `cmd.exe`. This is important because calling
`.terminate()` in the code above does not work as it kills the PID of `cmd.exe`
instead of the actual command that has been ran.

## Usage

Add the following to your `Cargo.toml`:

```toml
[dependencies]
create_process_w = { version = "0.1.0", package = "CreateProcessW" }
```

You can also use `CreateProcessW` directly, but this doesn't respect Rust's
naming recommendations.

## Create a new Command

To configure and create a new process, you can use the `Command` struct. The
only argument needed is the command line you want to execute.

```rust
use create_process_w::Command;

let command = Command::new("cargo.exe check")
    .inherit_handles(false)
    .current_dir(r"C:\Users\user_name\repos\repo_name");
```

### Customization

You can customize the command with these methods:

* `new` create a Command using the provided command line.
    The first part of the string before a space specifies the module name.
    If you are using a long file name that contains a space, use quoted strings
    to indicate where the file name ends and arguments begin. If the file name
    does not contain an extension, `.exe` is appended. This is the equivalent of the
    `lpCommandLine` parameter of the [`CreateProcessW`][create-process-w-parameters]
    function.

* `inherit_handles` enable/disable handles inherance.
    If this parameter is `true`, each inheritable handle in the calling process
    is inherited by the new process. If the parameter is `false`, the handles
    are not inherited. Note that inherited handles have the same value and
    access rights as the original handles. The default value is `true` and this
    is the equivalent of the `bInheritHandles` parameter of the
    [`CreateProcessW`][create-process-w-parameters] function.

* `current_directory` is the full path to the current directory for the process.
    If you don't provide a value, the new process will have the same current
    drive and directory as the calling process and this is equivalent of the
    `lpCurrentDirectory` parameter of the
    [`CreateProcessW`][create-process-w-parameters] function.

## Execute the command

This library give you two way to execute a command that match `std::process`'s
API:

* `spawn` return a handle to the child process as a `Child` struct.

    ```rust
    use create_process_w::Command;

    let child = Command::new("notepad.exe").spawn().expect("cannot spawn notepad");

    if let Some(status) = child.try_wait().expect("waiting process failed") {
        println!("Process exited with status code {}", status.status());
    } else {
        println!("Process is running");
    }

    child.kill().expect("cannot kill process");
    child.wait().expect("cannot wait process");
    ```

* `status` wait for it to finish and return an `ExitStatus`.

    ```rust
    use create_process_w::Command;

    let status = Command::new("notepad.exe").status();

    if status.success() {
        println!("Success!");
    } else {
        println!("Process exited with status code {}", status.code());
    }
    ```

* `output` is not available at the moment.

[windows-rs]: https://github.com/microsoft/windows-rs
[creating-processes]: https://docs.microsoft.com/en-us/windows/win32/procthread/creating-processes
[create-process-w-parameters]: https://docs.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-createprocessw#parameters
