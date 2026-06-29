use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use tauri::Manager;

#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
use windows_sys::Win32::UI::Shell::IsUserAnAdmin;

const CREATE_NO_WINDOW: u32 = 0x08000000;
const COMMAND_TIMEOUT_SECS: u64 = 120;
const ADMIN_TIMEOUT_SECS: u64 = 15 * 60;
const CHANGE_LOG_FILE: &str = "monitor-identity-changes.json";
const WATCHDOG_DIR: &str = "monitor-identity-watchdogs";
const GENERATED_INF_DIR: &str = "monitor-identity-inf";
const DEFAULT_ROLLBACK_TIMEOUT_SECS: u64 = 30;
const EDID_BLOCK_LEN: usize = 128;

#[derive(Debug, Serialize)]
pub struct MonitorIdentityStatus {
    is_administrator: bool,
    active_monitor_count: usize,
    pending_confirmation: Option<PendingMonitorIdentityConfirmation>,
    change_count: usize,
    monitors: Vec<MonitorIdentityInfo>,
    changes: Vec<MonitorIdentityChangeRecord>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MonitorIdentityInfo {
    instance_name: String,
    device_instance_id: String,
    hardware_id: String,
    registry_path: Option<String>,
    active: bool,
    edid_present: bool,
    override_present: bool,
    windows_reported: Option<WindowsReportedMonitorIdentity>,
    current: Option<EdidIdentity>,
    original: Option<EdidIdentity>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WindowsReportedMonitorIdentity {
    manufacturer_id: Option<String>,
    product_code: Option<String>,
    serial_number: Option<String>,
    monitor_name: Option<String>,
    hardware_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EdidIdentity {
    manufacturer_id: String,
    product_code_hex: String,
    numeric_serial: u32,
    serial_number: Option<String>,
    monitor_name: Option<String>,
    checksum_valid: bool,
    windows_hardware_id: String,
}

#[derive(Debug, Deserialize)]
pub struct MonitorIdentityOverrideRequest {
    monitor_device_instance_id: String,
    manufacturer_id: String,
    product_code_hex: String,
    numeric_serial: Option<u32>,
    serial_number: Option<String>,
    monitor_name: Option<String>,
    rollback_timeout_secs: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct MonitorIdentityActionResult {
    action: String,
    succeeded: bool,
    message: String,
    output: String,
    pending_confirmation: Option<PendingMonitorIdentityConfirmation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingMonitorIdentityConfirmation {
    token: String,
    change_id: String,
    monitor_device_instance_id: String,
    expires_at: String,
    seconds_remaining: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorIdentityChangeRecord {
    id: String,
    status: String,
    #[serde(default = "default_monitor_identity_apply_mode")]
    apply_mode: String,
    applied_at: String,
    confirmed_at: Option<String>,
    rolled_back_at: Option<String>,
    rollback_token: String,
    expires_at: String,
    monitor_device_instance_id: String,
    original_hardware_id: String,
    target_hardware_id: String,
    registry_path: String,
    original_identity: EdidIdentity,
    target_identity: EdidIdentity,
    original_edid_hex: String,
    previous_override_edid_hex: Option<String>,
    new_override_edid_hex: String,
    generated_inf_path: String,
    #[serde(default)]
    published_driver_inf: Option<String>,
    #[serde(default)]
    published_driver_name_path: Option<String>,
    confirm_file_path: String,
    watchdog_status_path: String,
    output: String,
}

#[derive(Debug, Deserialize)]
struct MonitorSnapshotList {
    items: Vec<MonitorRegistrySnapshot>,
}

#[derive(Debug, Clone, Deserialize)]
struct MonitorRegistrySnapshot {
    instance_name: String,
    device_instance_id: String,
    hardware_id: String,
    registry_path: Option<String>,
    active: bool,
    #[serde(default)]
    wmi_manufacturer_id: Option<String>,
    #[serde(default)]
    wmi_product_code: Option<String>,
    #[serde(default)]
    wmi_serial_number: Option<String>,
    #[serde(default)]
    wmi_monitor_name: Option<String>,
    edid_hex: Option<String>,
    override_edid_hex: Option<String>,
}

struct CommandOutput {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum MonitorIdentityApplyMode {
    RegistryOverride,
    InfDriver,
}

impl MonitorIdentityApplyMode {
    fn record_value(self) -> &'static str {
        match self {
            Self::RegistryOverride => "registry_override",
            Self::InfDriver => "inf_driver",
        }
    }

    fn action(self) -> &'static str {
        match self {
            Self::RegistryOverride => "monitor_identity_apply_override",
            Self::InfDriver => "monitor_identity_install_inf_override",
        }
    }

    fn success_message(self, timeout_secs: u64) -> String {
        match self {
            Self::RegistryOverride => format!(
                "已应用显示器 EDID 身份覆盖。请在 {timeout_secs} 秒内确认，否则会自动回滚。"
            ),
            Self::InfDriver => format!(
                "已安装显示器 INF 覆盖并重扫设备。请在 {timeout_secs} 秒内确认，否则会自动回滚。"
            ),
        }
    }
}

fn default_monitor_identity_apply_mode() -> String {
    MonitorIdentityApplyMode::RegistryOverride
        .record_value()
        .to_string()
}

#[tauri::command]
pub fn monitor_identity_get_status(app: tauri::AppHandle) -> Result<MonitorIdentityStatus, String> {
    let config_dir = app_config_dir(&app)?;
    sync_watchdog_outcomes(&config_dir)?;
    build_status(&config_dir)
}

#[tauri::command]
pub fn monitor_identity_apply_override(
    app: tauri::AppHandle,
    request: MonitorIdentityOverrideRequest,
) -> Result<MonitorIdentityActionResult, String> {
    apply_monitor_identity_change(app, request, MonitorIdentityApplyMode::RegistryOverride)
}

#[tauri::command]
pub fn monitor_identity_install_inf_override(
    app: tauri::AppHandle,
    request: MonitorIdentityOverrideRequest,
) -> Result<MonitorIdentityActionResult, String> {
    apply_monitor_identity_change(app, request, MonitorIdentityApplyMode::InfDriver)
}

fn apply_monitor_identity_change(
    app: tauri::AppHandle,
    request: MonitorIdentityOverrideRequest,
    apply_mode: MonitorIdentityApplyMode,
) -> Result<MonitorIdentityActionResult, String> {
    let config_dir = app_config_dir(&app)?;
    sync_watchdog_outcomes(&config_dir)?;

    let normalized = NormalizedOverrideRequest::from_request(request)?;
    let snapshots = collect_monitor_snapshots()?;
    let snapshot = snapshots
        .into_iter()
        .find(|monitor| {
            monitor
                .device_instance_id
                .eq_ignore_ascii_case(&normalized.monitor_device_instance_id)
        })
        .ok_or_else(|| "所选显示器已不存在。".to_string())?;

    let registry_path = snapshot
        .registry_path
        .clone()
        .ok_or_else(|| "所选显示器没有可写入的 Device Parameters 注册表路径。".to_string())?;
    let original_edid_hex = snapshot
        .edid_hex
        .clone()
        .ok_or_else(|| "所选显示器没有可读取的原始 EDID。".to_string())?;
    let original_edid = hex_to_bytes(&original_edid_hex)?;
    let original_identity = parse_edid_identity(&original_edid)?;
    let mut target_edid = original_edid.clone();
    apply_identity_to_edid(&mut target_edid, &normalized)?;
    let target_identity = parse_edid_identity(&target_edid)?;
    let new_override_edid_hex = bytes_to_hex(&target_edid[..EDID_BLOCK_LEN]);

    let now = chrono::Local::now();
    let timeout_secs = normalized.rollback_timeout_secs;
    let expires_at = now + chrono::Duration::seconds(timeout_secs as i64);
    let change_id = format!(
        "monitor-{}",
        now.timestamp_nanos_opt()
            .unwrap_or_else(|| now.timestamp_millis())
    );
    let rollback_token = format!(
        "{}-{}",
        std::process::id(),
        now.timestamp_nanos_opt()
            .unwrap_or_else(|| now.timestamp_millis())
    );
    let watchdog_dir = config_dir.join(WATCHDOG_DIR).join(&rollback_token);
    fs::create_dir_all(&watchdog_dir)
        .map_err(|error| format!("无法创建显示器自动回滚目录：{error}"))?;
    let confirm_file_path = watchdog_dir.join("keep-change.confirmed");
    let watchdog_status_path = watchdog_dir.join("watchdog.status");
    let watchdog_script_path = watchdog_dir.join("rollback-watchdog.ps1");
    let published_driver_name_path = watchdog_dir.join("published-driver.txt");

    let inf_dir = config_dir.join(GENERATED_INF_DIR);
    fs::create_dir_all(&inf_dir)
        .map_err(|error| format!("无法创建显示器 INF 备份目录：{error}"))?;
    let generated_inf_path = inf_dir.join(format!("{change_id}.inf"));
    let catalog_file_name = format!("{change_id}.cat");
    let inf = generate_monitor_inf(
        &snapshot.hardware_id,
        &target_identity.windows_hardware_id,
        &target_edid,
        &catalog_file_name,
    );
    fs::write(&generated_inf_path, inf)
        .map_err(|error| format!("无法写入显示器 INF 备份：{error}"))?;

    let rollback_script = build_watchdog_script(WatchdogScriptArgs {
        timeout_secs,
        confirm_file_path: &confirm_file_path,
        status_path: &watchdog_status_path,
        registry_path: &registry_path,
        previous_override_edid_hex: snapshot.override_edid_hex.as_deref(),
        device_instance_id: &snapshot.device_instance_id,
        published_driver_name_path: (apply_mode == MonitorIdentityApplyMode::InfDriver)
            .then_some(published_driver_name_path.as_path()),
    });
    fs::write(&watchdog_script_path, rollback_script)
        .map_err(|error| format!("无法写入显示器自动回滚脚本：{error}"))?;

    let mut record = MonitorIdentityChangeRecord {
        id: change_id.clone(),
        status: "pending".to_string(),
        apply_mode: apply_mode.record_value().to_string(),
        applied_at: now.to_rfc3339(),
        confirmed_at: None,
        rolled_back_at: None,
        rollback_token: rollback_token.clone(),
        expires_at: expires_at.to_rfc3339(),
        monitor_device_instance_id: snapshot.device_instance_id.clone(),
        original_hardware_id: snapshot.hardware_id.clone(),
        target_hardware_id: target_identity.windows_hardware_id.clone(),
        registry_path: registry_path.clone(),
        original_identity,
        target_identity,
        original_edid_hex,
        previous_override_edid_hex: snapshot.override_edid_hex.clone(),
        new_override_edid_hex: new_override_edid_hex.clone(),
        generated_inf_path: generated_inf_path.to_string_lossy().to_string(),
        published_driver_inf: None,
        published_driver_name_path: (apply_mode == MonitorIdentityApplyMode::InfDriver)
            .then(|| published_driver_name_path.to_string_lossy().to_string()),
        confirm_file_path: confirm_file_path.to_string_lossy().to_string(),
        watchdog_status_path: watchdog_status_path.to_string_lossy().to_string(),
        output: String::new(),
    };

    append_change_record(&config_dir, record.clone())?;

    let apply_script = match apply_mode {
        MonitorIdentityApplyMode::RegistryOverride => build_apply_override_script(
            &registry_path,
            &new_override_edid_hex,
            &snapshot.device_instance_id,
            &watchdog_script_path,
        ),
        MonitorIdentityApplyMode::InfDriver => build_install_inf_override_script(
            &registry_path,
            &new_override_edid_hex,
            &snapshot.device_instance_id,
            &watchdog_script_path,
            &generated_inf_path,
            &published_driver_name_path,
            snapshot.override_edid_hex.as_deref(),
        ),
    };
    let output = match run_admin_script(apply_mode.action(), &apply_script) {
        Ok(output) => output,
        Err(error) => {
            let _ = remove_change_record(&config_dir, &change_id);
            return Err(error);
        }
    };

    record.output = output.clone();
    if apply_mode == MonitorIdentityApplyMode::InfDriver {
        record.published_driver_inf = read_published_driver_name(&published_driver_name_path)
            .or_else(|| extract_published_driver_inf(&output));
    }
    update_change_record(&config_dir, record.clone())?;

    let active_count = collect_monitor_snapshots()
        .map(|items| items.into_iter().filter(|item| item.active).count())
        .unwrap_or(0);
    if active_count == 0 {
        let _ = restore_change_record(&config_dir, &change_id, "应用后没有活动显示器。");
        return Err(
            "EDID 覆盖已写入，但 Windows 报告活动显示器数量为 0；已请求自动回滚。".to_string(),
        );
    }

    let pending = pending_confirmation_for_record(&record);
    Ok(MonitorIdentityActionResult {
        action: apply_mode.action().to_string(),
        succeeded: true,
        message: apply_mode.success_message(timeout_secs),
        output,
        pending_confirmation: Some(pending),
    })
}

#[tauri::command]
pub fn monitor_identity_confirm_override(
    app: tauri::AppHandle,
    token: String,
) -> Result<MonitorIdentityActionResult, String> {
    let config_dir = app_config_dir(&app)?;
    sync_watchdog_outcomes(&config_dir)?;
    let mut changes = load_change_log(&config_dir)?;
    let Some(record) = changes
        .iter_mut()
        .find(|record| record.rollback_token == token && record.status == "pending")
    else {
        return Err("没有找到与该确认令牌匹配的待确认显示器身份修改。".to_string());
    };

    let confirm_path = PathBuf::from(&record.confirm_file_path);
    if let Some(parent) = confirm_path.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("无法创建确认标记目录：{error}"))?;
    }
    fs::write(&confirm_path, chrono::Local::now().to_rfc3339())
        .map_err(|error| format!("无法写入显示器确认标记：{error}"))?;
    record.status = "confirmed".to_string();
    record.confirmed_at = Some(chrono::Local::now().to_rfc3339());
    save_change_log(&config_dir, &changes)?;

    Ok(MonitorIdentityActionResult {
        action: "monitor_identity_confirm_override".to_string(),
        succeeded: true,
        message: "已保留显示器身份覆盖，并取消自动回滚。".to_string(),
        output: confirm_path.to_string_lossy().to_string(),
        pending_confirmation: None,
    })
}

#[tauri::command]
pub fn monitor_identity_restore_change(
    app: tauri::AppHandle,
    change_id: Option<String>,
) -> Result<MonitorIdentityActionResult, String> {
    let config_dir = app_config_dir(&app)?;
    sync_watchdog_outcomes(&config_dir)?;
    let selected_change_id = match change_id {
        Some(value) if !value.trim().is_empty() => value,
        _ => latest_restorable_change_id(&config_dir)?,
    };
    let output = restore_change_record(&config_dir, &selected_change_id, "手动回滚")?;

    Ok(MonitorIdentityActionResult {
        action: "monitor_identity_restore_change".to_string(),
        succeeded: true,
        message: "已还原显示器身份覆盖。".to_string(),
        output,
        pending_confirmation: None,
    })
}

fn build_status(config_dir: &Path) -> Result<MonitorIdentityStatus, String> {
    let snapshots = collect_monitor_snapshots().unwrap_or_default();
    let monitors = snapshots
        .into_iter()
        .map(monitor_info_from_snapshot)
        .collect::<Vec<_>>();
    let active_monitor_count = monitors.iter().filter(|monitor| monitor.active).count();
    let changes = load_change_log(config_dir)?;
    let pending_confirmation = changes
        .iter()
        .rev()
        .find(|record| record.status == "pending")
        .map(pending_confirmation_for_record);

    Ok(MonitorIdentityStatus {
        is_administrator: is_administrator(),
        active_monitor_count,
        pending_confirmation,
        change_count: changes.len(),
        monitors,
        changes,
    })
}

fn monitor_info_from_snapshot(snapshot: MonitorRegistrySnapshot) -> MonitorIdentityInfo {
    let windows_reported = windows_reported_identity_from_snapshot(&snapshot);
    let original = snapshot
        .edid_hex
        .as_deref()
        .and_then(|hex| hex_to_bytes(hex).ok())
        .and_then(|bytes| parse_edid_identity(&bytes).ok());
    let current = snapshot
        .override_edid_hex
        .as_deref()
        .or(snapshot.edid_hex.as_deref())
        .and_then(|hex| hex_to_bytes(hex).ok())
        .and_then(|bytes| parse_edid_identity(&bytes).ok());

    MonitorIdentityInfo {
        instance_name: snapshot.instance_name,
        device_instance_id: snapshot.device_instance_id,
        hardware_id: snapshot.hardware_id,
        registry_path: snapshot.registry_path,
        active: snapshot.active,
        edid_present: snapshot.edid_hex.is_some(),
        override_present: snapshot.override_edid_hex.is_some(),
        windows_reported,
        current,
        original,
    }
}

fn windows_reported_identity_from_snapshot(
    snapshot: &MonitorRegistrySnapshot,
) -> Option<WindowsReportedMonitorIdentity> {
    if snapshot.wmi_manufacturer_id.is_none()
        && snapshot.wmi_product_code.is_none()
        && snapshot.wmi_serial_number.is_none()
        && snapshot.wmi_monitor_name.is_none()
    {
        return None;
    }

    let hardware_id = snapshot
        .wmi_manufacturer_id
        .as_deref()
        .zip(snapshot.wmi_product_code.as_deref())
        .map(|(manufacturer, product)| {
            format!(
                "MONITOR\\{}{}",
                manufacturer.to_ascii_uppercase(),
                product.to_ascii_uppercase()
            )
        });

    Some(WindowsReportedMonitorIdentity {
        manufacturer_id: snapshot.wmi_manufacturer_id.clone(),
        product_code: snapshot.wmi_product_code.clone(),
        serial_number: snapshot.wmi_serial_number.clone(),
        monitor_name: snapshot.wmi_monitor_name.clone(),
        hardware_id,
    })
}

fn pending_confirmation_for_record(
    record: &MonitorIdentityChangeRecord,
) -> PendingMonitorIdentityConfirmation {
    let seconds_remaining = chrono::DateTime::parse_from_rfc3339(&record.expires_at)
        .ok()
        .map(|expires| {
            let now = chrono::Local::now().fixed_offset();
            expires.signed_duration_since(now).num_seconds().max(0) as u64
        })
        .unwrap_or(0);

    PendingMonitorIdentityConfirmation {
        token: record.rollback_token.clone(),
        change_id: record.id.clone(),
        monitor_device_instance_id: record.monitor_device_instance_id.clone(),
        expires_at: record.expires_at.clone(),
        seconds_remaining,
    }
}

#[derive(Debug)]
struct NormalizedOverrideRequest {
    monitor_device_instance_id: String,
    manufacturer_id: String,
    product_code: u16,
    numeric_serial: Option<u32>,
    serial_number: Option<String>,
    monitor_name: Option<String>,
    rollback_timeout_secs: u64,
}

impl NormalizedOverrideRequest {
    fn from_request(request: MonitorIdentityOverrideRequest) -> Result<Self, String> {
        let manufacturer_id = normalize_manufacturer_id(&request.manufacturer_id)?;
        let product_code = parse_product_code(&request.product_code_hex)?;
        let serial_number = normalize_descriptor_text(request.serial_number, "序列号文本")?;
        let monitor_name = normalize_descriptor_text(request.monitor_name, "显示器名称")?;
        let rollback_timeout_secs = request
            .rollback_timeout_secs
            .unwrap_or(DEFAULT_ROLLBACK_TIMEOUT_SECS)
            .clamp(10, 120);
        let monitor_device_instance_id = request.monitor_device_instance_id.trim().to_string();
        if monitor_device_instance_id.is_empty() {
            return Err("必须选择一个显示器设备实例。".to_string());
        }

        Ok(Self {
            monitor_device_instance_id,
            manufacturer_id,
            product_code,
            numeric_serial: request.numeric_serial,
            serial_number,
            monitor_name,
            rollback_timeout_secs,
        })
    }
}

fn normalize_manufacturer_id(value: &str) -> Result<String, String> {
    let normalized = value.trim().to_ascii_uppercase();
    if normalized.len() != 3 || !normalized.bytes().all(|byte| byte.is_ascii_uppercase()) {
        return Err("厂商 ID（Manufacturer）必须是 3 个英文字母。".to_string());
    }
    Ok(normalized)
}

fn parse_product_code(value: &str) -> Result<u16, String> {
    let normalized = value
        .trim()
        .trim_start_matches("0x")
        .trim_start_matches("0X")
        .to_ascii_uppercase();
    if normalized.is_empty()
        || normalized.len() > 4
        || !normalized.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err("产品码（Product Code）必须是 1 到 4 位十六进制字符。".to_string());
    }
    u16::from_str_radix(&normalized, 16).map_err(|error| format!("产品码无效：{error}"))
}

fn normalize_descriptor_text(
    value: Option<String>,
    field_name: &str,
) -> Result<Option<String>, String> {
    let Some(value) = value else {
        return Ok(None);
    };
    let trimmed = value.trim().to_string();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if trimmed.len() > 13 {
        return Err(format!("{field_name} 必须不超过 13 个 ASCII 字符。"));
    }
    if !trimmed.bytes().all(|byte| (0x20..=0x7e).contains(&byte)) {
        return Err(format!("{field_name} 只能包含可打印 ASCII 字符。"));
    }
    Ok(Some(trimmed))
}

fn parse_edid_identity(edid: &[u8]) -> Result<EdidIdentity, String> {
    if edid.len() < EDID_BLOCK_LEN {
        return Err("EDID 数据块至少需要 128 字节。".to_string());
    }
    let manufacturer_id = decode_manufacturer_id(edid[8], edid[9])?;
    let product_code = u16::from_le_bytes([edid[10], edid[11]]);
    let product_code_hex = format!("{product_code:04X}");
    let numeric_serial = u32::from_le_bytes([edid[12], edid[13], edid[14], edid[15]]);
    let serial_number = find_descriptor_text(edid, 0xff);
    let monitor_name = find_descriptor_text(edid, 0xfc);
    let checksum_valid = edid[..EDID_BLOCK_LEN]
        .iter()
        .fold(0u8, |sum, byte| sum.wrapping_add(*byte))
        == 0;
    let windows_hardware_id = format!("MONITOR\\{manufacturer_id}{product_code_hex}");

    Ok(EdidIdentity {
        manufacturer_id,
        product_code_hex,
        numeric_serial,
        serial_number,
        monitor_name,
        checksum_valid,
        windows_hardware_id,
    })
}

fn apply_identity_to_edid(
    edid: &mut [u8],
    request: &NormalizedOverrideRequest,
) -> Result<(), String> {
    if edid.len() < EDID_BLOCK_LEN {
        return Err("EDID 数据块至少需要 128 字节。".to_string());
    }
    let manufacturer = encode_manufacturer_id(&request.manufacturer_id)?;
    edid[8] = manufacturer[0];
    edid[9] = manufacturer[1];
    let product = request.product_code.to_le_bytes();
    edid[10] = product[0];
    edid[11] = product[1];
    if let Some(serial) = request.numeric_serial {
        edid[12..=15].copy_from_slice(&serial.to_le_bytes());
    }
    if let Some(serial_number) = request.serial_number.as_deref() {
        replace_descriptor_text(edid, 0xff, serial_number)?;
    }
    if let Some(monitor_name) = request.monitor_name.as_deref() {
        replace_descriptor_text(edid, 0xfc, monitor_name)?;
    }
    update_base_block_checksum(edid);
    Ok(())
}

fn decode_manufacturer_id(high: u8, low: u8) -> Result<String, String> {
    let word = u16::from_be_bytes([high, low]);
    let codes = [
        ((word >> 10) & 0x1f) as u8,
        ((word >> 5) & 0x1f) as u8,
        (word & 0x1f) as u8,
    ];
    if codes.iter().any(|code| *code == 0 || *code > 26) {
        return Err("EDID 厂商 ID 包含无效字母编码。".to_string());
    }
    Ok(codes
        .iter()
        .map(|code| char::from(b'A' + code - 1))
        .collect())
}

fn encode_manufacturer_id(value: &str) -> Result<[u8; 2], String> {
    let normalized = normalize_manufacturer_id(value)?;
    let mut word = 0u16;
    for byte in normalized.bytes() {
        word = (word << 5) | u16::from(byte - b'A' + 1);
    }
    Ok(word.to_be_bytes())
}

fn descriptor_ranges() -> impl Iterator<Item = std::ops::Range<usize>> {
    (0..4).map(|index| {
        let start = 54 + index * 18;
        start..start + 18
    })
}

fn find_descriptor_text(edid: &[u8], tag: u8) -> Option<String> {
    descriptor_ranges().find_map(|range| {
        let descriptor = edid.get(range)?;
        if descriptor[0] == 0 && descriptor[1] == 0 && descriptor[2] == 0 && descriptor[3] == tag {
            Some(decode_descriptor_text(&descriptor[5..18]))
        } else {
            None
        }
    })
}

fn decode_descriptor_text(bytes: &[u8]) -> String {
    let end = bytes
        .iter()
        .position(|byte| *byte == 0x0a || *byte == 0x00)
        .unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).trim().to_string()
}

fn replace_descriptor_text(edid: &mut [u8], tag: u8, value: &str) -> Result<(), String> {
    let Some(range) = descriptor_ranges().find(|range| {
        edid.get(range.clone())
            .map(|descriptor| {
                descriptor[0] == 0
                    && descriptor[1] == 0
                    && descriptor[2] == 0
                    && descriptor[3] == tag
            })
            .unwrap_or(false)
    }) else {
        return Err(format!(
            "EDID 不包含描述符 tag 0x{tag:02X}；为避免覆盖时序块，已拒绝修改。"
        ));
    };

    let bytes = value.as_bytes();
    let descriptor = &mut edid[range];
    descriptor[5..18].fill(0x20);
    descriptor[5..5 + bytes.len()].copy_from_slice(bytes);
    if bytes.len() < 13 {
        descriptor[5 + bytes.len()] = 0x0a;
    }
    Ok(())
}

fn update_base_block_checksum(edid: &mut [u8]) {
    edid[127] = 0;
    let sum = edid[..127]
        .iter()
        .fold(0u8, |sum, byte| sum.wrapping_add(*byte));
    edid[127] = 0u8.wrapping_sub(sum);
}

fn generate_monitor_inf(
    original_hardware_id: &str,
    target_hardware_id: &str,
    edid: &[u8],
    catalog_file_name: &str,
) -> String {
    let bytes = edid[..EDID_BLOCK_LEN]
        .chunks(16)
        .map(|chunk| {
            chunk
                .iter()
                .map(|byte| format!("0x{byte:02X}"))
                .collect::<Vec<_>>()
                .join(",")
        })
        .collect::<Vec<_>>()
        .join(",\\\n  ");
    format!(
        r#"; Generated by PC Requirements Checker for audit/backup.
; This monitor INF applies the EDID block through the device hardware key.

[Version]
Signature="$WINDOWS NT$"
Class=Monitor
ClassGuid={{4D36E96E-E325-11CE-BFC1-08002BE10318}}
Provider=%ProviderName%
CatalogFile={catalog_file_name}
DriverVer=06/28/2026,1.0.0.0

[Manufacturer]
%ProviderName%=MonitorModels,NTamd64,NTarm64

[MonitorModels.NTamd64]
%MonitorName%=MonitorInstall.NTamd64,{original_hardware_id}

[MonitorModels.NTarm64]
%MonitorName%=MonitorInstall.NTarm64,{original_hardware_id}

[MonitorInstall.NTamd64]

[MonitorInstall.NTamd64.HW]
AddReg=MonitorOverride

[MonitorInstall.NTarm64]

[MonitorInstall.NTarm64.HW]
AddReg=MonitorOverride

[MonitorOverride]
HKR,EDID_OVERRIDE,"0",0x00000001,\
  {bytes}

[Strings]
ProviderName="PC Requirements Checker"
MonitorName="{target_hardware_id} EDID identity override"
"#
    )
}

fn build_apply_override_script(
    registry_path: &str,
    edid_hex: &str,
    device_instance_id: &str,
    watchdog_script_path: &Path,
) -> String {
    let registry_path = escape_ps_single(registry_path);
    let edid_hex = escape_ps_single(edid_hex);
    let device_instance_id = escape_ps_single(device_instance_id);
    let watchdog_script_path = escape_ps_single(&watchdog_script_path.to_string_lossy());
    format!(
        r#"
function Convert-HexToBytes([string]$hex) {{
  $bytes = New-Object byte[] ($hex.Length / 2)
  for ($i = 0; $i -lt $bytes.Length; $i++) {{
    $bytes[$i] = [Convert]::ToByte($hex.Substring($i * 2, 2), 16)
  }}
  return $bytes
}}

$overridePath = 'HKLM:\{registry_path}\EDID_OVERRIDE'
New-Item -Path $overridePath -Force | Out-Null
New-ItemProperty -LiteralPath $overridePath -Name '0' -PropertyType Binary -Value (Convert-HexToBytes '{edid_hex}') -Force | Out-Null
"已写入 EDID_OVERRIDE\0：{registry_path}"

$restart = & pnputil /restart-device '{device_instance_id}' 2>&1 | Out-String
"pnputil /restart-device 输出："
$restart.Trim()

$scan = & pnputil /scan-devices 2>&1 | Out-String
"pnputil /scan-devices 输出："
$scan.Trim()

Start-Process -FilePath 'powershell.exe' -ArgumentList @('-NoProfile','-ExecutionPolicy','Bypass','-File','{watchdog_script_path}') -WindowStyle Hidden | Out-Null
"已启动 30 秒自动回滚保护进程。"
"#
    )
}

fn build_restore_override_body(previous_override_edid_hex: Option<&str>) -> String {
    match previous_override_edid_hex {
        Some(hex) => {
            let hex = escape_ps_single(hex);
            format!(
                r#"
New-Item -Path $overridePath -Force | Out-Null
New-ItemProperty -LiteralPath $overridePath -Name '0' -PropertyType Binary -Value (Convert-HexToBytes '{hex}') -Force | Out-Null
"已恢复之前的 EDID_OVERRIDE 值。"
"#
            )
        }
        None => r#"
if (Test-Path -LiteralPath $overridePath) {
  Remove-ItemProperty -LiteralPath $overridePath -Name '0' -ErrorAction SilentlyContinue
  try { Remove-Item -LiteralPath $overridePath -Force -ErrorAction SilentlyContinue } catch {}
}
"已删除 EDID_OVERRIDE 值。"
"#
        .to_string(),
    }
}

fn build_install_inf_override_script(
    registry_path: &str,
    edid_hex: &str,
    device_instance_id: &str,
    watchdog_script_path: &Path,
    generated_inf_path: &Path,
    published_driver_name_path: &Path,
    previous_override_edid_hex: Option<&str>,
) -> String {
    let registry_path = escape_ps_single(registry_path);
    let edid_hex = escape_ps_single(edid_hex);
    let device_instance_id = escape_ps_single(device_instance_id);
    let watchdog_script_path = escape_ps_single(&watchdog_script_path.to_string_lossy());
    let generated_inf_path = escape_ps_single(&generated_inf_path.to_string_lossy());
    let published_driver_name_path =
        escape_ps_single(&published_driver_name_path.to_string_lossy());
    let restore_body = build_restore_override_body(previous_override_edid_hex);
    format!(
        r#"
function Convert-HexToBytes([string]$hex) {{
  $bytes = New-Object byte[] ($hex.Length / 2)
  for ($i = 0; $i -lt $bytes.Length; $i++) {{
    $bytes[$i] = [Convert]::ToByte($hex.Substring($i * 2, 2), 16)
  }}
  return $bytes
}}

function Find-WindowsKitTool([string]$toolName) {{
  $roots = @(
    "${{env:ProgramFiles(x86)}}\Windows Kits\10\bin",
    "$env:ProgramFiles\Windows Kits\10\bin"
  ) | Where-Object {{ $_ -and (Test-Path -LiteralPath $_) }}
  $tools = @()
  foreach ($root in $roots) {{
    $tools += @(Get-ChildItem -LiteralPath $root -Recurse -File -Filter $toolName -ErrorAction SilentlyContinue)
  }}
  $preferred = $tools | Where-Object {{ $_.FullName -match '\\x64\\' }} | Sort-Object FullName -Descending | Select-Object -First 1
  if ($preferred) {{ return $preferred.FullName }}
  $fallback = $tools | Sort-Object FullName -Descending | Select-Object -First 1
  if ($fallback) {{ return $fallback.FullName }}
  throw "找不到 Windows Kits 工具 $toolName；请安装 Windows Driver Kit 或包含该工具的 Windows Kits 组件。"
}}

function Ensure-MonitorCatalogCertificate([string]$workDir) {{
  $subject = 'CN=PC Requirements Checker Monitor Identity Test Signing'
  $cert = Get-ChildItem -Path Cert:\LocalMachine\My -CodeSigningCert -ErrorAction SilentlyContinue |
    Where-Object {{ $_.Subject -eq $subject -and $_.HasPrivateKey }} |
    Sort-Object NotAfter -Descending |
    Select-Object -First 1
  if (-not $cert) {{
    $cert = New-SelfSignedCertificate `
      -Type CodeSigningCert `
      -Subject $subject `
      -CertStoreLocation Cert:\LocalMachine\My `
      -KeyAlgorithm RSA `
      -KeyLength 2048 `
      -HashAlgorithm SHA256 `
      -KeyExportPolicy Exportable `
      -KeyUsage DigitalSignature `
      -NotAfter (Get-Date).AddYears(10)
    "已创建本机显示器 INF 测试签名证书：$($cert.Thumbprint)"
  }} else {{
    "复用本机显示器 INF 测试签名证书：$($cert.Thumbprint)"
  }}

  $cerPath = Join-Path $workDir 'pc-monitor-identity-test-signing.cer'
  Export-Certificate -Cert $cert -FilePath $cerPath -Force | Out-Null
  Import-Certificate -FilePath $cerPath -CertStoreLocation Cert:\LocalMachine\Root | Out-Null
  Import-Certificate -FilePath $cerPath -CertStoreLocation Cert:\LocalMachine\TrustedPublisher | Out-Null
  return $cert
}}

function Ensure-SignedMonitorCatalog([string]$infPath) {{
  $infDirectory = Split-Path -Parent $infPath
  $infName = Split-Path -Leaf $infPath
  $infText = Get-Content -LiteralPath $infPath -Raw
  $catalogMatch = [regex]::Match($infText, '(?im)^\s*CatalogFile(?:\.[^\s=]+)?\s*=\s*(.+?)\s*$')
  if (-not $catalogMatch.Success) {{
    throw "显示器 INF 缺少 CatalogFile，无法生成可安装的签名驱动包。"
  }}
  $catalogName = $catalogMatch.Groups[1].Value.Trim().Trim('"')
  if ($catalogName -notmatch '^[A-Za-z0-9_.-]+\.cat$') {{
    throw "显示器 INF 的 CatalogFile 名称不安全：$catalogName"
  }}
  $catalogPath = Join-Path $infDirectory $catalogName
  $existingSignature = if (Test-Path -LiteralPath $catalogPath) {{ Get-AuthenticodeSignature -LiteralPath $catalogPath }} else {{ $null }}
  if ($existingSignature -and $existingSignature.Status -eq 'Valid') {{
    "已存在有效 catalog 签名：$catalogName"
    return $catalogPath
  }}

  $makeCat = Find-WindowsKitTool 'makecat.exe'
  $signTool = Find-WindowsKitTool 'signtool.exe'
  $cdfName = [IO.Path]::ChangeExtension($catalogName, '.cdf')
  $cdfPath = Join-Path $infDirectory $cdfName
  $cdf = @"
[CatalogHeader]
Name=$catalogName
PublicVersion=0x0000001
EncodingType=0x00010001
CATATTR1=0x10010001:OSAttr:2:10.0

[CatalogFiles]
<hash>MonitorInf=$infName
"@
  Set-Content -LiteralPath $cdfPath -Value $cdf -Encoding ASCII

  Push-Location $infDirectory
  try {{
    $makeCatOutput = & $makeCat -v $cdfName 2>&1 | Out-String
    $makeCatExit = $LASTEXITCODE
  }} finally {{
    Pop-Location
  }}
  "makecat 输出："
  $makeCatOutput.Trim()
  if ($makeCatExit -ne 0 -or -not (Test-Path -LiteralPath $catalogPath)) {{
    throw "生成显示器 INF catalog 失败。"
  }}

  $cert = Ensure-MonitorCatalogCertificate $infDirectory
  $signOutput = & $signTool sign /fd SHA256 /sm /s My /sha1 $cert.Thumbprint $catalogPath 2>&1 | Out-String
  $signExit = $LASTEXITCODE
  "signtool sign 输出："
  $signOutput.Trim()
  if ($signExit -ne 0) {{
    throw "签名显示器 INF catalog 失败。"
  }}

  $signature = Get-AuthenticodeSignature -LiteralPath $catalogPath
  if ($signature.Status -ne 'Valid') {{
    throw "显示器 INF catalog 签名无效：$($signature.Status) $($signature.StatusMessage)"
  }}
  "已生成并签名显示器 INF catalog：$catalogName"
  return $catalogPath
}}

function Restore-PreviousEdidOverride {{
{restore_body}
}}

$overridePath = 'HKLM:\{registry_path}\EDID_OVERRIDE'
$publishedDriverNamePath = '{published_driver_name_path}'
$infPath = '{generated_inf_path}'
Ensure-SignedMonitorCatalog $infPath | Out-Null

Remove-Item -LiteralPath $publishedDriverNamePath -Force -ErrorAction SilentlyContinue
New-Item -Path $overridePath -Force | Out-Null
New-ItemProperty -LiteralPath $overridePath -Name '0' -PropertyType Binary -Value (Convert-HexToBytes '{edid_hex}') -Force | Out-Null
"已写入 EDID_OVERRIDE\0：{registry_path}"

$add = & pnputil /add-driver $infPath /install 2>&1 | Out-String
$addExit = $LASTEXITCODE
"pnputil /add-driver /install 输出："
$add.Trim()
if ($addExit -ne 0) {{
  Restore-PreviousEdidOverride
  throw ("pnputil /add-driver /install 失败，已恢复之前的 EDID_OVERRIDE。输出：" + $add.Trim())
}}

$published = [regex]::Match($add, '(?i)oem\d+\.inf')
if ($published.Success) {{
  $published.Value.ToLowerInvariant() | Out-File -FilePath $publishedDriverNamePath -Encoding ASCII
  "已记录发布驱动包：$($published.Value.ToLowerInvariant())"
}} else {{
  "没有从 pnputil 输出中识别到 oem*.inf；如需回滚，将仅恢复 EDID_OVERRIDE。"
}}

$drivers = & pnputil /enum-devices /instanceid '{device_instance_id}' /drivers 2>&1 | Out-String
"pnputil /enum-devices /drivers 输出："
$drivers.Trim()

$restart = & pnputil /restart-device '{device_instance_id}' 2>&1 | Out-String
"pnputil /restart-device 输出："
$restart.Trim()

$scan = & pnputil /scan-devices 2>&1 | Out-String
"pnputil /scan-devices 输出："
$scan.Trim()

Start-Process -FilePath 'powershell.exe' -ArgumentList @('-NoProfile','-ExecutionPolicy','Bypass','-File','{watchdog_script_path}') -WindowStyle Hidden | Out-Null
"已启动 30 秒自动回滚保护进程。"
"#
    )
}

struct WatchdogScriptArgs<'a> {
    timeout_secs: u64,
    confirm_file_path: &'a Path,
    status_path: &'a Path,
    registry_path: &'a str,
    previous_override_edid_hex: Option<&'a str>,
    device_instance_id: &'a str,
    published_driver_name_path: Option<&'a Path>,
}

fn build_watchdog_script(args: WatchdogScriptArgs<'_>) -> String {
    let confirm_file_path = escape_ps_single(&args.confirm_file_path.to_string_lossy());
    let status_path = escape_ps_single(&args.status_path.to_string_lossy());
    let registry_path = escape_ps_single(args.registry_path);
    let device_instance_id = escape_ps_single(args.device_instance_id);
    let restore_body = build_restore_override_body(args.previous_override_edid_hex);
    let driver_rollback_body = args
        .published_driver_name_path
        .map(|path| {
            let path = escape_ps_single(&path.to_string_lossy());
            format!(
                r#"
$publishedNamePath = '{path}'
if (Test-Path -LiteralPath $publishedNamePath) {{
  $publishedName = (Get-Content -LiteralPath $publishedNamePath -Raw).Trim()
  if ($publishedName -match '(?i)^oem\d+\.inf$') {{
    $delete = & pnputil /delete-driver $publishedName /uninstall /force 2>&1 | Out-String
    "pnputil /delete-driver 输出："
    $delete.Trim()
  }}
}}
"#
            )
        })
        .unwrap_or_default();

    format!(
        r#"
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
$confirmPath = '{confirm_file_path}'
$statusPath = '{status_path}'
$overridePath = 'HKLM:\{registry_path}\EDID_OVERRIDE'

function Convert-HexToBytes([string]$hex) {{
  $bytes = New-Object byte[] ($hex.Length / 2)
  for ($i = 0; $i -lt $bytes.Length; $i++) {{
    $bytes[$i] = [Convert]::ToByte($hex.Substring($i * 2, 2), 16)
  }}
  return $bytes
}}

Start-Sleep -Seconds {timeout}
try {{
  if (Test-Path -LiteralPath $confirmPath) {{
    "confirmed" | Out-File -FilePath $statusPath -Encoding UTF8
    exit 0
  }}

{restore_body}
{driver_rollback_body}

  $restart = & pnputil /restart-device '{device_instance_id}' 2>&1 | Out-String
  $scan = & pnputil /scan-devices 2>&1 | Out-String
  ("rolled_back`n" + $restart + "`n" + $scan) | Out-File -FilePath $statusPath -Encoding UTF8
  exit 0
}} catch {{
  ("rollback_failed`n" + $_.Exception.Message) | Out-File -FilePath $statusPath -Encoding UTF8
  exit 1
}}
"#,
        timeout = args.timeout_secs
    )
}

fn restore_change_record(
    config_dir: &Path,
    change_id: &str,
    reason: &str,
) -> Result<String, String> {
    let changes = load_change_log(config_dir)?;
    let record = changes
        .iter()
        .find(|record| record.id == change_id)
        .cloned()
        .ok_or_else(|| format!("找不到显示器身份变更记录：{change_id}。"))?;

    let confirm_path = PathBuf::from(&record.confirm_file_path);
    if let Some(parent) = confirm_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(&confirm_path, format!("rollback requested: {reason}"));

    let restore_script = build_restore_script(&record);
    let output = run_admin_script("monitor_identity_restore_change", &restore_script)?;
    mark_change_rolled_back(config_dir, change_id)?;
    Ok(output)
}

fn build_restore_script(record: &MonitorIdentityChangeRecord) -> String {
    let registry_path = escape_ps_single(&record.registry_path);
    let device_instance_id = escape_ps_single(&record.monitor_device_instance_id);
    let restore_body = build_restore_override_body(record.previous_override_edid_hex.as_deref());
    let driver_restore_body = record
        .published_driver_inf
        .as_deref()
        .filter(|name| is_published_driver_inf_name(name))
        .map(|name| {
            let name = escape_ps_single(name);
            format!(
                r#"
$delete = & pnputil /delete-driver '{name}' /uninstall /force 2>&1 | Out-String
"pnputil /delete-driver 输出："
$delete.Trim()
"#
            )
        })
        .unwrap_or_default();

    format!(
        r#"
function Convert-HexToBytes([string]$hex) {{
  $bytes = New-Object byte[] ($hex.Length / 2)
  for ($i = 0; $i -lt $bytes.Length; $i++) {{
    $bytes[$i] = [Convert]::ToByte($hex.Substring($i * 2, 2), 16)
  }}
  return $bytes
}}

$overridePath = 'HKLM:\{registry_path}\EDID_OVERRIDE'
{restore_body}
{driver_restore_body}

$restart = & pnputil /restart-device '{device_instance_id}' 2>&1 | Out-String
"pnputil /restart-device 输出："
$restart.Trim()

$scan = & pnputil /scan-devices 2>&1 | Out-String
"pnputil /scan-devices 输出："
$scan.Trim()
"#
    )
}

fn collect_monitor_snapshots() -> Result<Vec<MonitorRegistrySnapshot>, String> {
    #[cfg(not(windows))]
    {
        return Ok(Vec::new());
    }

    #[cfg(windows)]
    {
        let script = r#"
$ErrorActionPreference = 'Stop'
function BytesToHex($bytes) {
  if ($null -eq $bytes) { return $null }
  return -join ([byte[]]$bytes | ForEach-Object { $_.ToString('X2') })
}
function DecodeMonitorString($value) {
  if ($null -eq $value) { return $null }
  $chars = @()
  foreach ($code in @($value)) {
    if ([int]$code -eq 0) { continue }
    $chars += [char][int]$code
  }
  $text = (-join $chars).Trim()
  if ([string]::IsNullOrWhiteSpace($text)) { return $null }
  return $text
}

$items = @()
$monitors = @(Get-CimInstance -Namespace root\wmi -ClassName WmiMonitorID -ErrorAction SilentlyContinue)
foreach ($monitor in $monitors) {
  $instance = [string]$monitor.InstanceName
  $device = $instance -replace '_\d+$', ''
  $parts = $device -split '\\'
  $registryPath = $null
  $hardwareId = ''
  $edid = $null
  $overrideEdid = $null
  if ($parts.Count -ge 3) {
    $hardwareId = "MONITOR\$($parts[1])"
    $registryPath = "SYSTEM\CurrentControlSet\Enum\$($parts[0])\$($parts[1])\$($parts[2])\Device Parameters"
    $keyPath = "HKLM:\$registryPath"
    try {
      $props = Get-ItemProperty -LiteralPath $keyPath -ErrorAction Stop
      $edid = $props.EDID
    } catch {}
    try {
      $overrideProps = Get-ItemProperty -LiteralPath "$keyPath\EDID_OVERRIDE" -ErrorAction Stop
      $overrideEdid = $overrideProps.'0'
    } catch {}
  }
  $items += [pscustomobject]@{
    instance_name = $instance
    device_instance_id = $device
    hardware_id = $hardwareId
    registry_path = $registryPath
    active = [bool]$monitor.Active
    wmi_manufacturer_id = (DecodeMonitorString $monitor.ManufacturerName)
    wmi_product_code = (DecodeMonitorString $monitor.ProductCodeID)
    wmi_serial_number = (DecodeMonitorString $monitor.SerialNumberID)
    wmi_monitor_name = (DecodeMonitorString $monitor.UserFriendlyName)
    edid_hex = BytesToHex $edid
    override_edid_hex = BytesToHex $overrideEdid
  }
}

[pscustomobject]@{ items = @($items) } | ConvertTo-Json -Depth 5 -Compress
"#;

        let output = run_program(
            "powershell.exe",
            &[
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                script,
            ],
        )?;
        if output.code != Some(0) {
            return Err(join_non_empty(&[output.stderr, output.stdout], "\n"));
        }
        let parsed: MonitorSnapshotList = serde_json::from_str(&output.stdout)
            .map_err(|error| format!("无法解析显示器清单 JSON：{error}"))?;
        Ok(parsed.items)
    }
}

fn app_config_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map_err(|error| format!("无法定位应用配置目录：{error}"))
}

fn change_log_path(config_dir: &Path) -> PathBuf {
    config_dir.join(CHANGE_LOG_FILE)
}

fn load_change_log(config_dir: &Path) -> Result<Vec<MonitorIdentityChangeRecord>, String> {
    let path = change_log_path(config_dir);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let text = fs::read_to_string(&path).map_err(|error| format!("无法读取变更记录：{error}"))?;
    serde_json::from_str(&text).map_err(|error| format!("显示器变更记录格式无效：{error}"))
}

fn save_change_log(
    config_dir: &Path,
    changes: &[MonitorIdentityChangeRecord],
) -> Result<(), String> {
    fs::create_dir_all(config_dir).map_err(|error| format!("无法创建应用配置目录：{error}"))?;
    let path = change_log_path(config_dir);
    let temp_path = path.with_extension("json.tmp");
    let json = serde_json::to_string_pretty(changes)
        .map_err(|error| format!("无法序列化显示器变更记录：{error}"))?;
    fs::write(&temp_path, json).map_err(|error| format!("无法写入显示器变更记录：{error}"))?;
    fs::rename(&temp_path, &path).map_err(|error| format!("无法替换显示器变更记录：{error}"))
}

fn append_change_record(
    config_dir: &Path,
    record: MonitorIdentityChangeRecord,
) -> Result<(), String> {
    let mut changes = load_change_log(config_dir)?;
    changes.push(record);
    save_change_log(config_dir, &changes)
}

fn update_change_record(
    config_dir: &Path,
    record: MonitorIdentityChangeRecord,
) -> Result<(), String> {
    let mut changes = load_change_log(config_dir)?;
    if let Some(existing) = changes.iter_mut().find(|change| change.id == record.id) {
        *existing = record;
    } else {
        changes.push(record);
    }
    save_change_log(config_dir, &changes)
}

fn remove_change_record(config_dir: &Path, change_id: &str) -> Result<(), String> {
    let mut changes = load_change_log(config_dir)?;
    changes.retain(|change| change.id != change_id);
    save_change_log(config_dir, &changes)
}

fn latest_restorable_change_id(config_dir: &Path) -> Result<String, String> {
    let changes = load_change_log(config_dir)?;
    changes
        .iter()
        .rev()
        .find(|record| record.status != "rolled_back")
        .map(|record| record.id.clone())
        .ok_or_else(|| "没有可还原的显示器身份变更记录。".to_string())
}

fn mark_change_rolled_back(config_dir: &Path, change_id: &str) -> Result<(), String> {
    let mut changes = load_change_log(config_dir)?;
    let Some(record) = changes.iter_mut().find(|record| record.id == change_id) else {
        return Ok(());
    };
    record.status = "rolled_back".to_string();
    record.rolled_back_at = Some(chrono::Local::now().to_rfc3339());
    save_change_log(config_dir, &changes)
}

fn sync_watchdog_outcomes(config_dir: &Path) -> Result<(), String> {
    let mut changes = load_change_log(config_dir)?;
    let mut changed = false;
    for record in &mut changes {
        if record.status != "pending" {
            continue;
        }
        let status_path = PathBuf::from(&record.watchdog_status_path);
        let status_text = fs::read_to_string(status_path).unwrap_or_default();
        if status_text.contains("rolled_back") {
            record.status = "rolled_back".to_string();
            record.rolled_back_at = Some(chrono::Local::now().to_rfc3339());
            changed = true;
        } else if status_text.contains("confirmed") {
            record.status = "confirmed".to_string();
            record.confirmed_at = Some(chrono::Local::now().to_rfc3339());
            changed = true;
        }
    }
    if changed {
        save_change_log(config_dir, &changes)?;
    }
    Ok(())
}

fn hex_to_bytes(hex: &str) -> Result<Vec<u8>, String> {
    let normalized = hex.trim();
    if normalized.len() % 2 != 0 {
        return Err("十六进制字符串长度必须是偶数。".to_string());
    }
    (0..normalized.len())
        .step_by(2)
        .map(|index| {
            u8::from_str_radix(&normalized[index..index + 2], 16)
                .map_err(|error| format!("Invalid hex byte: {error}"))
        })
        .collect()
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<String>()
}

fn read_published_driver_name(path: &Path) -> Option<String> {
    fs::read_to_string(path)
        .ok()
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| is_published_driver_inf_name(value))
}

fn extract_published_driver_inf(output: &str) -> Option<String> {
    output
        .split(|ch: char| ch.is_whitespace() || ch == ':' || ch == ';' || ch == ',' || ch == '"')
        .map(|token| {
            token
                .trim_matches(|ch: char| !ch.is_ascii_alphanumeric() && ch != '.')
                .to_ascii_lowercase()
        })
        .find(|token| is_published_driver_inf_name(token))
}

fn is_published_driver_inf_name(value: &str) -> bool {
    let value = value.trim();
    if !value.to_ascii_lowercase().starts_with("oem")
        || !value.to_ascii_lowercase().ends_with(".inf")
    {
        return false;
    }
    let digits = &value[3..value.len().saturating_sub(4)];
    !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())
}

fn run_admin_script(action: &str, script_body: &str) -> Result<String, String> {
    let action_dir = admin_action_temp_dir(action)?;
    let log_path = action_dir.join("action.log");
    let wrapped_script = wrap_action_script(script_body, &log_path);
    let encoded_script = encode_powershell_command(&wrapped_script);
    let run_result = if is_administrator() {
        run_program_with_timeout(
            "powershell.exe",
            &[
                "-NoProfile",
                "-NonInteractive",
                "-EncodedCommand",
                &encoded_script,
            ],
            Duration::from_secs(ADMIN_TIMEOUT_SECS),
        )
    } else {
        run_elevated_powershell_encoded(&encoded_script)
    };
    let output = fs::read_to_string(&log_path).unwrap_or_else(|_| String::new());
    let _ = fs::remove_file(&log_path);
    let _ = fs::remove_dir(&action_dir);
    let command_output = run_result?;
    if command_output.code == Some(0) {
        Ok(join_non_empty(
            &[output, command_output.stdout, command_output.stderr],
            "\n",
        ))
    } else {
        Err(join_non_empty(
            &[
                output,
                command_output.stderr,
                command_output.stdout,
                format!("Exit code: {:?}", command_output.code),
            ],
            "\n",
        ))
    }
}

fn admin_action_temp_dir(action: &str) -> Result<PathBuf, String> {
    let base_dir = std::env::temp_dir().join("pc-requirements-checker");
    fs::create_dir_all(&base_dir).map_err(|error| format!("无法创建临时操作目录：{error}"))?;
    let stamp = chrono::Local::now().timestamp_millis();
    let safe_action = action.replace(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_', "_");
    let action_dir = base_dir.join(format!("{safe_action}_{}_{}", std::process::id(), stamp));
    fs::create_dir(&action_dir).map_err(|error| format!("无法创建临时操作工作区：{error}"))?;
    Ok(action_dir)
}

fn wrap_action_script(script_body: &str, log_path: &Path) -> String {
    let log_path = escape_ps_single(&log_path.to_string_lossy());
    format!(
        r#"
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
$pcRequirementsLogPath = '{log_path}'
try {{
  $pcRequirementsOutput = @()
  $pcRequirementsOutput += & {{
{script_body}
  }} | Out-String
  ($pcRequirementsOutput -join "`n").Trim() | Out-File -FilePath $pcRequirementsLogPath -Encoding UTF8
  exit 0
}} catch {{
  ("ERROR: " + $_.Exception.Message) | Out-File -FilePath $pcRequirementsLogPath -Encoding UTF8
  exit 1
}}
"#
    )
}

fn encode_powershell_command(script: &str) -> String {
    let bytes = script
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect::<Vec<_>>();
    BASE64_STANDARD.encode(bytes)
}

fn run_elevated_powershell_encoded(encoded_script: &str) -> Result<CommandOutput, String> {
    let command = format!(
        "$p = Start-Process -FilePath 'powershell.exe' -ArgumentList @('-NoProfile','-NonInteractive','-EncodedCommand','{encoded_script}') -Verb RunAs -WindowStyle Hidden -Wait -PassThru; if ($null -eq $p) {{ exit 1 }} else {{ exit $p.ExitCode }}"
    );
    run_program_with_timeout(
        "powershell.exe",
        &["-NoProfile", "-NonInteractive", "-Command", &command],
        Duration::from_secs(ADMIN_TIMEOUT_SECS),
    )
}

#[cfg(windows)]
fn is_administrator() -> bool {
    unsafe { IsUserAnAdmin() != 0 }
}

#[cfg(not(windows))]
fn is_administrator() -> bool {
    false
}

fn run_program(program: &str, args: &[&str]) -> Result<CommandOutput, String> {
    run_program_with_timeout(program, args, Duration::from_secs(COMMAND_TIMEOUT_SECS))
}

fn run_program_with_timeout(
    program: &str,
    args: &[&str],
    timeout: Duration,
) -> Result<CommandOutput, String> {
    let mut command = Command::new(program);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);

    let mut child = command
        .spawn()
        .map_err(|error| format!("无法运行 {program}：{error}"))?;
    let started_at = Instant::now();

    loop {
        if child
            .try_wait()
            .map_err(|error| format!("等待 {program} 结束失败：{error}"))?
            .is_some()
        {
            let output = child
                .wait_with_output()
                .map_err(|error| format!("读取 {program} 输出失败：{error}"))?;
            return Ok(CommandOutput {
                code: output.status.code(),
                stdout: String::from_utf8_lossy(&output.stdout).trim().to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            });
        }

        if started_at.elapsed() >= timeout {
            let _ = child.kill();
            let output = child.wait_with_output().ok();
            return Ok(CommandOutput {
                code: None,
                stdout: output
                    .as_ref()
                    .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
                    .unwrap_or_default(),
                stderr: format!("执行超过 {} 秒后超时", timeout.as_secs()),
            });
        }

        std::thread::sleep(Duration::from_millis(50));
    }
}

fn escape_ps_single(value: &str) -> String {
    value.replace('\'', "''")
}

fn join_non_empty(parts: &[String], separator: &str) -> String {
    parts
        .iter()
        .filter(|part| !part.trim().is_empty())
        .map(|part| part.trim())
        .collect::<Vec<_>>()
        .join(separator)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_edid() -> Vec<u8> {
        let mut edid = vec![0u8; EDID_BLOCK_LEN];
        edid[0..8].copy_from_slice(&[0x00, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x00]);
        let manufacturer = encode_manufacturer_id("LHC").unwrap();
        edid[8] = manufacturer[0];
        edid[9] = manufacturer[1];
        edid[10..12].copy_from_slice(&0x906a_u16.to_le_bytes());
        edid[12..16].copy_from_slice(&123456_u32.to_le_bytes());
        edid[54..72].copy_from_slice(&[
            0x00, 0x00, 0x00, 0xff, 0x00, b'O', b'L', b'D', b'S', b'N', 0x0a, 0x20, 0x20, 0x20,
            0x20, 0x20, 0x20, 0x20,
        ]);
        edid[72..90].copy_from_slice(&[
            0x00, 0x00, 0x00, 0xfc, 0x00, b'P', b'2', b'7', b'1', b'0', b'V', 0x0a, 0x20, 0x20,
            0x20, 0x20, 0x20, 0x20,
        ]);
        update_base_block_checksum(&mut edid);
        edid
    }

    #[test]
    fn manufacturer_code_round_trips() {
        let encoded = encode_manufacturer_id("DEL").unwrap();
        assert_eq!(
            decode_manufacturer_id(encoded[0], encoded[1]).unwrap(),
            "DEL"
        );
        assert!(encode_manufacturer_id("D3L").is_err());
    }

    #[test]
    fn parses_and_applies_identity_without_touching_timings() {
        let mut edid = sample_edid();
        let original = parse_edid_identity(&edid).unwrap();
        assert_eq!(original.windows_hardware_id, "MONITOR\\LHC906A");
        assert_eq!(original.serial_number.as_deref(), Some("OLDSN"));
        assert_eq!(original.monitor_name.as_deref(), Some("P2710V"));
        assert!(original.checksum_valid);

        let request = NormalizedOverrideRequest {
            monitor_device_instance_id: "DISPLAY\\LHC906A\\ABC".to_string(),
            manufacturer_id: "DEL".to_string(),
            product_code: 0xa123,
            numeric_serial: Some(987654),
            serial_number: Some("SN123".to_string()),
            monitor_name: Some("FAKE PANEL".to_string()),
            rollback_timeout_secs: 30,
        };
        apply_identity_to_edid(&mut edid, &request).unwrap();
        let changed = parse_edid_identity(&edid).unwrap();
        assert_eq!(changed.windows_hardware_id, "MONITOR\\DELA123");
        assert_eq!(changed.numeric_serial, 987654);
        assert_eq!(changed.serial_number.as_deref(), Some("SN123"));
        assert_eq!(changed.monitor_name.as_deref(), Some("FAKE PANEL"));
        assert!(changed.checksum_valid);
    }

    #[test]
    fn refuses_to_replace_missing_descriptor() {
        let mut edid = sample_edid();
        edid[54..72].fill(0);
        let request = NormalizedOverrideRequest {
            monitor_device_instance_id: "DISPLAY\\LHC906A\\ABC".to_string(),
            manufacturer_id: "DEL".to_string(),
            product_code: 0xa123,
            numeric_serial: None,
            serial_number: Some("SN123".to_string()),
            monitor_name: None,
            rollback_timeout_secs: 30,
        };
        assert!(apply_identity_to_edid(&mut edid, &request).is_err());
    }

    #[test]
    fn generated_inf_contains_edid_override_block() {
        let edid = sample_edid();
        let inf = generate_monitor_inf(
            "MONITOR\\LHC906A",
            "MONITOR\\DELA123",
            &edid,
            "monitor-test.cat",
        );
        assert!(inf.contains("Class=Monitor"));
        assert!(inf.contains("CatalogFile=monitor-test.cat"));
        assert!(inf.contains("HKR,EDID_OVERRIDE,\"0\",0x00000001"));
        assert!(inf.contains("MONITOR\\LHC906A"));
        assert!(inf.contains("MONITOR\\DELA123 EDID identity override"));
    }

    #[test]
    fn watchdog_script_restores_or_removes_override_after_timeout() {
        let script = build_watchdog_script(WatchdogScriptArgs {
            timeout_secs: 30,
            confirm_file_path: Path::new("C:\\Temp\\keep.confirmed"),
            status_path: Path::new("C:\\Temp\\watchdog.status"),
            registry_path: r"SYSTEM\CurrentControlSet\Enum\DISPLAY\LHC906A\ABC\Device Parameters",
            previous_override_edid_hex: None,
            device_instance_id: r"DISPLAY\LHC906A\ABC",
            published_driver_name_path: None,
        });
        assert!(script.contains("Start-Sleep -Seconds 30"));
        assert!(script.contains("Remove-ItemProperty"));
        assert!(script.contains("rolled_back"));
        assert!(script.contains("pnputil /restart-device"));
    }

    #[test]
    fn generated_inf_uses_hardware_install_section() {
        let edid = sample_edid();
        let inf = generate_monitor_inf(
            "MONITOR\\LHC906A",
            "MONITOR\\DELA123",
            &edid,
            "monitor-test.cat",
        );
        assert!(inf.contains("[MonitorInstall.NTamd64.HW]"));
        assert!(inf.contains("[MonitorInstall.NTarm64.HW]"));
        assert!(inf.contains("%MonitorName%=MonitorInstall.NTamd64,MONITOR\\LHC906A"));
        assert!(inf.contains("HKR,EDID_OVERRIDE,\"0\",0x00000001"));
    }

    #[test]
    fn install_inf_script_records_published_driver_and_restores_on_failure() {
        let script = build_install_inf_override_script(
            r"SYSTEM\CurrentControlSet\Enum\DISPLAY\LHC906A\ABC\Device Parameters",
            "00FF",
            r"DISPLAY\LHC906A\ABC",
            Path::new(r"C:\Temp\watchdog.ps1"),
            Path::new(r"C:\Temp\monitor.inf"),
            Path::new(r"C:\Temp\published-driver.txt"),
            None,
        );
        assert!(script.contains("pnputil /add-driver"));
        assert!(script.contains("/install"));
        assert!(script.contains("published-driver.txt"));
        assert!(script.contains("CatalogFile"));
        assert!(script.contains("makecat.exe"));
        assert!(script.contains("signtool.exe"));
        assert!(script.contains("New-SelfSignedCertificate"));
        assert!(script.contains("TrustedPublisher"));
        assert!(script.contains("Restore-PreviousEdidOverride"));
        assert!(script.contains("pnputil /enum-devices"));
        assert!(
            script.find("CatalogFile").unwrap()
                < script.find("New-Item -Path $overridePath").unwrap()
        );
    }

    #[test]
    fn published_driver_name_is_parsed_conservatively() {
        let output = "Driver package added successfully.\nPublished Name: oem42.inf";
        assert_eq!(
            extract_published_driver_inf(output).as_deref(),
            Some("oem42.inf")
        );
        assert!(is_published_driver_inf_name("oem7.inf"));
        assert!(!is_published_driver_inf_name("monitor.inf"));
        assert!(!is_published_driver_inf_name("oem.inf"));
    }
}
