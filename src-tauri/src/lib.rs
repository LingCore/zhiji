mod checks;
mod gaming_optimizer;
mod monitor_identity;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
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
            monitor_identity::monitor_identity_confirm_override,
            monitor_identity::monitor_identity_restore_change
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
