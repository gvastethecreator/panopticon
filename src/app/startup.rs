//! Windows logon / startup integration helpers.

use std::path::Path;

use anyhow::{Context, Result};
use windows::Win32::Foundation::ERROR_FILE_NOT_FOUND;
use windows::Win32::System::Registry::{
    RegDeleteKeyValueW, RegSetKeyValueW, HKEY_CURRENT_USER, REG_SZ,
};

use super::menu_utils::encode_wide;

const RUN_SUBKEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
const STARTUP_VALUE_NAME: &str = "Panopticon";

/// Synchronize the Windows logon entry with the persisted setting.
pub(crate) fn sync_run_at_startup(enabled: bool, workspace_name: Option<&str>) {
    let result = if enabled {
        register_run_at_startup(workspace_name)
    } else {
        unregister_run_at_startup()
    };

    if let Err(error) = result {
        tracing::warn!(%error, enabled, workspace = ?workspace_name, "failed to sync run-at-startup state");
    }
}

fn register_run_at_startup(workspace_name: Option<&str>) -> Result<()> {
    let executable = std::env::current_exe().context("resolve current executable path")?;
    let command = startup_command_for_path(&executable, workspace_name);
    let value_name = encode_wide(STARTUP_VALUE_NAME);
    let subkey = encode_wide(RUN_SUBKEY);
    let value_data = encode_wide(&command);
    let value_byte_len = u32::try_from(std::mem::size_of_val(value_data.as_slice()))
        .context("startup registry value too large")?;

    // SAFETY: static HKCU hive, UTF-16 NUL-terminated subkey/value strings, and
    // a valid REG_SZ UTF-16 buffer whose lifetime outlives the call.
    unsafe {
        RegSetKeyValueW(
            HKEY_CURRENT_USER,
            windows::core::PCWSTR(subkey.as_ptr()),
            windows::core::PCWSTR(value_name.as_ptr()),
            REG_SZ.0,
            Some(value_data.as_ptr().cast()),
            value_byte_len,
        )
    }
    .ok()
    .context("write startup registry value")
}

fn unregister_run_at_startup() -> Result<()> {
    let value_name = encode_wide(STARTUP_VALUE_NAME);
    let subkey = encode_wide(RUN_SUBKEY);

    // SAFETY: static HKCU hive and UTF-16 NUL-terminated subkey/value strings.
    let status = unsafe {
        RegDeleteKeyValueW(
            HKEY_CURRENT_USER,
            windows::core::PCWSTR(subkey.as_ptr()),
            windows::core::PCWSTR(value_name.as_ptr()),
        )
    };

    if status == ERROR_FILE_NOT_FOUND {
        Ok(())
    } else {
        status.ok().context("delete startup registry value")
    }
}

fn startup_command_for_path(executable: &Path, workspace_name: Option<&str>) -> String {
    let mut command = quote_argument(&executable.display().to_string());
    if let Some(workspace_name) = workspace_name.filter(|value| !value.trim().is_empty()) {
        command.push_str(" --workspace ");
        command.push_str(&quote_argument(workspace_name));
    }
    command
}

fn quote_argument(argument: &str) -> String {
    format!("\"{argument}\"")
}

#[cfg(test)]
mod tests {
    use super::startup_command_for_path;
    use std::path::Path;

    #[test]
    fn startup_command_quotes_executable_and_omits_default_workspace() {
        let command =
            startup_command_for_path(Path::new(r"C:\Apps\Panopticon\panopticon.exe"), None);

        assert_eq!(command, r#""C:\Apps\Panopticon\panopticon.exe""#);
    }

    #[test]
    fn startup_command_includes_workspace_argument_when_present() {
        let command = startup_command_for_path(
            Path::new(r"C:\Apps\Panopticon\panopticon.exe"),
            Some("focus board"),
        );

        assert_eq!(
            command,
            r#""C:\Apps\Panopticon\panopticon.exe" --workspace "focus board""#
        );
    }
}
