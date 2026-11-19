pub mod discovery;
pub mod estimate;
pub mod fingerprint;
pub mod hardware;
pub mod keep_awake;
pub mod mcp;
pub mod media;
pub mod policy;
pub mod profiles;
pub mod queue;
pub mod report;
pub mod runner;
pub mod worker;

/// Constructs a `std::process::Command` that suppresses console window popups on Windows.
/// On Windows, `CREATE_NO_WINDOW = 0x08000000` prevents flashing black terminal windows (`conhost.exe`).
pub fn silent_command<S: AsRef<std::ffi::OsStr>>(program: S) -> std::process::Command {
    #[allow(unused_mut)]
    let mut cmd = std::process::Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}
