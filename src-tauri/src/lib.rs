mod checks;
mod gaming_optimizer;
mod monitor_identity;

use std::process::Command;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

const CREATE_NO_WINDOW: u32 = 0x08000000;
const PROJECT_REPOSITORY_URL: &str = "https://github.com/LingCore/zhiji";

#[tauri::command]
fn open_project_repository() -> Result<(), String> {
    open_external_url(PROJECT_REPOSITORY_URL)
}

fn open_external_url(url: &str) -> Result<(), String> {
    if url != PROJECT_REPOSITORY_URL || !url.starts_with("https://github.com/") {
        return Err("拒绝打开未允许的外部链接。".to_string());
    }

    let mut command = default_browser_command(url);
    command
        .status()
        .map_err(|error| format!("无法打开默认浏览器：{error}"))?
        .success()
        .then_some(())
        .ok_or_else(|| "默认浏览器打开失败。".to_string())
}

#[cfg(windows)]
fn default_browser_command(url: &str) -> Command {
    let mut command = Command::new("rundll32.exe");
    command
        .arg("url.dll,FileProtocolHandler")
        .arg(url)
        .creation_flags(CREATE_NO_WINDOW);
    command
}

#[cfg(target_os = "macos")]
fn default_browser_command(url: &str) -> Command {
    let mut command = Command::new("open");
    command.arg(url);
    command
}

#[cfg(all(unix, not(target_os = "macos")))]
fn default_browser_command(url: &str) -> Command {
    let mut command = Command::new("xdg-open");
    command.arg(url);
    command
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            open_project_repository,
            gaming_optimizer::gaming_get_optimizer_status,
            gaming_optimizer::gaming_apply_optimizer_action,
            gaming_optimizer::gaming_apply_competitive_preset,
            gaming_optimizer::gaming_restore_optimizer_changes,
            checks::run_checks,
            checks::apply_requirement_action,
            checks::set_virtual_memory_system_managed,
            checks::set_virtual_memory_custom,
            checks::configure_minidump_collection,
            checks::open_bluescreenview,
            checks::export_bluescreen_report,
            checks::restore_initial_config,
            checks::restart_to_firmware,
            monitor_identity::monitor_identity_get_status,
            monitor_identity::monitor_identity_apply_override,
            monitor_identity::monitor_identity_install_inf_override,
            monitor_identity::monitor_identity_reenumerate_device,
            monitor_identity::monitor_identity_confirm_override,
            monitor_identity::monitor_identity_restore_change
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::PROJECT_REPOSITORY_URL;

    #[test]
    fn project_repository_url_is_fixed_github_https_url() {
        assert_eq!(PROJECT_REPOSITORY_URL, "https://github.com/LingCore/zhiji");
        assert!(PROJECT_REPOSITORY_URL.starts_with("https://github.com/"));
    }
}
