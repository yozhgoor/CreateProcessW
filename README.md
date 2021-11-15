# CreateProcessW

This crate provide an API similar to `std::process` to create and handle process
using the [windows-rs][windows-rs] crate (see [this example][creating-processes]).

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


[creating-processes]: https://docs.microsoft.com/en-us/windows/win32/procthread/creating-processes
