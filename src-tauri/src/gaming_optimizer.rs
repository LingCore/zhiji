use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tauri::Manager;

#[cfg(windows)]
use windows_sys::Win32::UI::Shell::IsUserAnAdmin;
#[cfg(windows)]
use winreg::{enums::*, RegKey};

const CHANGE_LOG_FILE: &str = "gaming-optimizer-changes.json";
const HAGS_PATH: &str = r"SYSTEM\CurrentControlSet\Control\GraphicsDrivers";
const HAGS_VALUE: &str = "HwSchMode";
const GAME_CONFIG_STORE_PATH: &str = r"System\GameConfigStore";
const GAME_DVR_PATH: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\GameDVR";
const GAME_BAR_PATH: &str = r"Software\Microsoft\GameBar";
const APP_COMPAT_LAYERS_PATH: &str =
    r"Software\Microsoft\Windows NT\CurrentVersion\AppCompatFlags\Layers";
const DISABLE_FULLSCREEN_OPT_FLAG: &str = "DISABLEDXMAXIMIZEDWINDOWEDMODE";

#[derive(Debug, Serialize)]
pub struct GamingOptimizerStatus {
    is_administrator: bool,
    hags: OptimizerSettingState,
    game_capture: OptimizerSettingState,
    game_mode: OptimizerSettingState,
    fullscreen_optimization: OptimizerSettingState,
    selected_game_path: Option<String>,
    pending_reboot: bool,
    change_count: usize,
    changes: Vec<OptimizerChangeRecord>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OptimizerSettingState {
    key: String,
    label: String,
    detail: String,
    recommended: String,
    requires_admin: bool,
    requires_restart: bool,
}

#[derive(Debug, Deserialize)]
pub struct GamingActionRequest {
    action: String,
    game_path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GamingPresetRequest {
    game_path: Option<String>,
    include_hags: bool,
    include_game_capture: bool,
    include_fullscreen_optimization: bool,
}

#[derive(Debug, Serialize)]
pub struct GamingOptimizerResult {
    action: String,
    message: String,
    requires_restart: bool,
    output: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizerChangeRecord {
    id: String,
    applied_at: String,
    action: String,
    requires_restart: bool,
    registry_changes: Vec<RegistryChangeRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryChangeRecord {
    hive: String,
    path: String,
    name: String,
    previous_value: Option<RegistryValueSnapshot>,
    new_value: Option<RegistryValueSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RegistryValueSnapshot {
    U32(u32),
    String(String),
}

#[tauri::command]
pub fn gaming_get_optimizer_status(
    app: tauri::AppHandle,
    game_path: Option<String>,
) -> Result<GamingOptimizerStatus, String> {
    let changes = load_change_log(&app)?;
    let selected_game_path = normalize_optional_path(game_path);
    let pending_reboot = changes.iter().any(|change| change.requires_restart);

    Ok(GamingOptimizerStatus {
        is_administrator: is_administrator(),
        hags: hags_state(),
        game_capture: game_capture_state(),
        game_mode: game_mode_state(),
        fullscreen_optimization: fullscreen_optimization_state(selected_game_path.as_deref()),
        selected_game_path,
        pending_reboot,
        change_count: changes.len(),
        changes,
    })
}

#[tauri::command]
pub fn gaming_apply_optimizer_action(
    app: tauri::AppHandle,
    request: GamingActionRequest,
) -> Result<GamingOptimizerResult, String> {
    match request.action.as_str() {
        "enable_hags" => set_hags(&app, Some(2), "启用 HAGS（硬件加速 GPU 调度）"),
        "default_hags" => set_hags(&app, None, "恢复 HAGS 为系统默认"),
        "disable_game_capture" => {
            set_game_capture_disabled(&app, true, "禁用 Xbox Game Bar / Game DVR 捕获")
        }
        "restore_game_capture" => {
            set_game_capture_disabled(&app, false, "恢复 Xbox Game Bar / Game DVR 捕获")
        }
        "disable_game_mode" => set_game_mode_disabled(&app, true, "禁用 Game Mode（实验项）"),
        "restore_game_mode" => set_game_mode_disabled(&app, false, "恢复 Game Mode 为系统默认"),
        "disable_fullscreen_optimization" => set_fullscreen_optimization_disabled(
            &app,
            request.game_path.as_deref(),
            true,
            "禁用所选游戏的全屏优化",
        ),
        "restore_fullscreen_optimization" => set_fullscreen_optimization_disabled(
            &app,
            request.game_path.as_deref(),
            false,
            "恢复所选游戏的全屏优化设置",
        ),
        other => Err(format!("未知竞技优化操作：{other}")),
    }
}

#[tauri::command]
pub fn gaming_apply_competitive_preset(
    app: tauri::AppHandle,
    request: GamingPresetRequest,
) -> Result<GamingOptimizerResult, String> {
    let mut outputs = Vec::new();
    let mut requires_restart = false;

    if request.include_hags {
        let result = set_hags(&app, Some(2), "启用 HAGS（硬件加速 GPU 调度）")?;
        requires_restart |= result.requires_restart;
        outputs.push(result.output);
    }

    if request.include_game_capture {
        let result = set_game_capture_disabled(&app, true, "禁用 Xbox Game Bar / Game DVR 捕获")?;
        requires_restart |= result.requires_restart;
        outputs.push(result.output);
    }

    if request.include_fullscreen_optimization {
        if request
            .game_path
            .as_deref()
            .map(|path| !path.trim().is_empty())
            .unwrap_or(false)
        {
            let result = set_fullscreen_optimization_disabled(
                &app,
                request.game_path.as_deref(),
                true,
                "禁用所选游戏的全屏优化",
            )?;
            requires_restart |= result.requires_restart;
            outputs.push(result.output);
        } else {
            outputs.push("未选择游戏 exe，已跳过全屏优化。".to_string());
        }
    }

    Ok(GamingOptimizerResult {
        action: "competitive_preset".to_string(),
        message: "已应用安全竞技 FPS 预设。".to_string(),
        requires_restart,
        output: join_non_empty(&outputs, "\n"),
    })
}

#[tauri::command]
pub fn gaming_restore_optimizer_changes(
    app: tauri::AppHandle,
) -> Result<GamingOptimizerResult, String> {
    let changes = load_change_log(&app)?;
    if changes.is_empty() {
        return Ok(GamingOptimizerResult {
            action: "restore_gaming_optimizer_changes".to_string(),
            message: "没有需要还原的竞技模式改动。".to_string(),
            requires_restart: false,
            output: String::new(),
        });
    }

    let mut restored = Vec::new();
    let mut requires_restart = false;
    for change in changes.iter().rev() {
        for registry_change in change.registry_changes.iter().rev() {
            restore_registry_value(registry_change)?;
        }
        requires_restart |= change.requires_restart;
        restored.push(change.action.clone());
    }

    save_change_log(&app, &[])?;
    Ok(GamingOptimizerResult {
        action: "restore_gaming_optimizer_changes".to_string(),
        message: "已还原竞技模式记录的所有改动。".to_string(),
        requires_restart,
        output: restored.join("\n"),
    })
}

fn set_hags(
    app: &tauri::AppHandle,
    mode: Option<u32>,
    action_label: &str,
) -> Result<GamingOptimizerResult, String> {
    let hive = RegistryHive::LocalMachine;
    let previous = read_registry_value(hive, HAGS_PATH, HAGS_VALUE)?;
    let new_value = mode.map(RegistryValueSnapshot::U32);
    let change = RegistryChangeRecord {
        hive: hive.as_str().to_string(),
        path: HAGS_PATH.to_string(),
        name: HAGS_VALUE.to_string(),
        previous_value: previous.clone(),
        new_value: new_value.clone(),
    };
    apply_registry_change(&change)?;
    append_change_log(app, action_label, true, vec![change])?;

    Ok(GamingOptimizerResult {
        action: "hags".to_string(),
        message: format!("{action_label} 已写入，重启 Windows 后生效。"),
        requires_restart: true,
        output: format!(
            "HwSchMode: {} -> {}",
            snapshot_label(previous.as_ref()),
            snapshot_label(new_value.as_ref())
        ),
    })
}

fn set_game_capture_disabled(
    app: &tauri::AppHandle,
    disabled: bool,
    action_label: &str,
) -> Result<GamingOptimizerResult, String> {
    let target = if disabled { Some(0) } else { None };
    let changes = vec![
        dword_change(
            RegistryHive::CurrentUser,
            GAME_CONFIG_STORE_PATH,
            "GameDVR_Enabled",
            target,
        )?,
        dword_change(
            RegistryHive::CurrentUser,
            GAME_DVR_PATH,
            "AppCaptureEnabled",
            target,
        )?,
    ];
    for change in &changes {
        apply_registry_change(change)?;
    }
    append_change_log(app, action_label, false, changes)?;

    Ok(GamingOptimizerResult {
        action: "game_capture".to_string(),
        message: format!("{action_label} 已完成。"),
        requires_restart: false,
        output: "GameDVR_Enabled / AppCaptureEnabled 已更新。".to_string(),
    })
}

fn set_game_mode_disabled(
    app: &tauri::AppHandle,
    disabled: bool,
    action_label: &str,
) -> Result<GamingOptimizerResult, String> {
    let target = if disabled { Some(0) } else { None };
    let changes = vec![
        dword_change(
            RegistryHive::CurrentUser,
            GAME_BAR_PATH,
            "AutoGameModeEnabled",
            target,
        )?,
        dword_change(
            RegistryHive::CurrentUser,
            GAME_BAR_PATH,
            "AllowAutoGameMode",
            target,
        )?,
    ];
    for change in &changes {
        apply_registry_change(change)?;
    }
    append_change_log(app, action_label, false, changes)?;

    Ok(GamingOptimizerResult {
        action: "game_mode".to_string(),
        message: format!("{action_label} 已完成。建议重开游戏后 A/B 测试。"),
        requires_restart: false,
        output: "AutoGameModeEnabled / AllowAutoGameMode 已更新。".to_string(),
    })
}

fn set_fullscreen_optimization_disabled(
    app: &tauri::AppHandle,
    game_path: Option<&str>,
    disabled: bool,
    action_label: &str,
) -> Result<GamingOptimizerResult, String> {
    let game_path = game_path
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .ok_or_else(|| "请先选择游戏 exe。".to_string())?;
    let path = PathBuf::from(game_path);
    if !path.exists() {
        return Err("选择的游戏 exe 不存在。".to_string());
    }
    if !path.is_file() {
        return Err("请选择具体的游戏 exe 文件。".to_string());
    }
    if is_process_running_for_path(&path) {
        return Err("检测到所选游戏正在运行，请完全退出游戏后再修改全屏优化。".to_string());
    }

    let previous =
        read_registry_value(RegistryHive::CurrentUser, APP_COMPAT_LAYERS_PATH, game_path)?;
    let previous_text = match previous.as_ref() {
        Some(RegistryValueSnapshot::String(value)) => value.clone(),
        _ => String::new(),
    };
    let next_text = if disabled {
        append_compat_flag(&previous_text, DISABLE_FULLSCREEN_OPT_FLAG)
    } else {
        remove_compat_flag(&previous_text, DISABLE_FULLSCREEN_OPT_FLAG)
    };
    let new_value = if next_text.trim().is_empty() {
        None
    } else {
        Some(RegistryValueSnapshot::String(next_text))
    };
    let change = RegistryChangeRecord {
        hive: RegistryHive::CurrentUser.as_str().to_string(),
        path: APP_COMPAT_LAYERS_PATH.to_string(),
        name: game_path.to_string(),
        previous_value: previous.clone(),
        new_value: new_value.clone(),
    };
    apply_registry_change(&change)?;
    append_change_log(app, action_label, false, vec![change])?;

    Ok(GamingOptimizerResult {
        action: "fullscreen_optimization".to_string(),
        message: format!("{action_label} 已完成。建议重开游戏。"),
        requires_restart: false,
        output: format!(
            "{}: {} -> {}",
            game_path,
            snapshot_label(previous.as_ref()),
            snapshot_label(new_value.as_ref())
        ),
    })
}

fn dword_change(
    hive: RegistryHive,
    path: &str,
    name: &str,
    next: Option<u32>,
) -> Result<RegistryChangeRecord, String> {
    Ok(RegistryChangeRecord {
        hive: hive.as_str().to_string(),
        path: path.to_string(),
        name: name.to_string(),
        previous_value: read_registry_value(hive, path, name)?,
        new_value: next.map(RegistryValueSnapshot::U32),
    })
}

fn hags_state() -> OptimizerSettingState {
    match read_registry_value(RegistryHive::LocalMachine, HAGS_PATH, HAGS_VALUE) {
        Ok(Some(RegistryValueSnapshot::U32(2))) => OptimizerSettingState {
            key: "on".to_string(),
            label: "已启用".to_string(),
            detail: "HwSchMode=2，需要重启后完全生效。".to_string(),
            recommended: "启用".to_string(),
            requires_admin: true,
            requires_restart: true,
        },
        Ok(Some(RegistryValueSnapshot::U32(1))) => OptimizerSettingState {
            key: "off".to_string(),
            label: "已禁用".to_string(),
            detail: "HwSchMode=1。".to_string(),
            recommended: "启用或系统默认后实测".to_string(),
            requires_admin: true,
            requires_restart: true,
        },
        Ok(Some(RegistryValueSnapshot::U32(value))) => OptimizerSettingState {
            key: "unknown".to_string(),
            label: "自定义".to_string(),
            detail: format!("HwSchMode={value}。"),
            recommended: "启用".to_string(),
            requires_admin: true,
            requires_restart: true,
        },
        Ok(_) => OptimizerSettingState {
            key: "unknown".to_string(),
            label: "系统默认".to_string(),
            detail: "未显式设置 HwSchMode。".to_string(),
            recommended: "启用后实测".to_string(),
            requires_admin: true,
            requires_restart: true,
        },
        Err(error) => unknown_state("读取 HAGS 失败", error, true, true, "启用"),
    }
}

fn game_capture_state() -> OptimizerSettingState {
    let game_dvr = read_registry_value(
        RegistryHive::CurrentUser,
        GAME_CONFIG_STORE_PATH,
        "GameDVR_Enabled",
    );
    let app_capture = read_registry_value(
        RegistryHive::CurrentUser,
        GAME_DVR_PATH,
        "AppCaptureEnabled",
    );

    match (game_dvr, app_capture) {
        (Ok(Some(RegistryValueSnapshot::U32(0))), Ok(Some(RegistryValueSnapshot::U32(0)))) => {
            OptimizerSettingState {
                key: "on".to_string(),
                label: "已禁用".to_string(),
                detail: "GameDVR_Enabled=0，AppCaptureEnabled=0。".to_string(),
                recommended: "禁用".to_string(),
                requires_admin: false,
                requires_restart: false,
            }
        }
        (Ok(left), Ok(right)) => OptimizerSettingState {
            key: "off".to_string(),
            label: "未完全禁用".to_string(),
            detail: format!(
                "GameDVR_Enabled={}，AppCaptureEnabled={}。",
                snapshot_label(left.as_ref()),
                snapshot_label(right.as_ref())
            ),
            recommended: "禁用".to_string(),
            requires_admin: false,
            requires_restart: false,
        },
        (left, right) => unknown_state(
            "读取 Game DVR 失败",
            join_non_empty(
                &[
                    left.err().unwrap_or_default(),
                    right.err().unwrap_or_default(),
                ],
                "; ",
            ),
            false,
            false,
            "禁用",
        ),
    }
}

fn game_mode_state() -> OptimizerSettingState {
    let auto = read_registry_value(
        RegistryHive::CurrentUser,
        GAME_BAR_PATH,
        "AutoGameModeEnabled",
    );
    let allow = read_registry_value(
        RegistryHive::CurrentUser,
        GAME_BAR_PATH,
        "AllowAutoGameMode",
    );

    match (auto, allow) {
        (Ok(Some(RegistryValueSnapshot::U32(0))), Ok(Some(RegistryValueSnapshot::U32(0)))) => {
            OptimizerSettingState {
                key: "on".to_string(),
                label: "已禁用".to_string(),
                detail: "AutoGameModeEnabled=0，AllowAutoGameMode=0。".to_string(),
                recommended: "实验项，逐机测试".to_string(),
                requires_admin: false,
                requires_restart: false,
            }
        }
        (Ok(left), Ok(right)) => OptimizerSettingState {
            key: "unknown".to_string(),
            label: "系统默认 / 未禁用".to_string(),
            detail: format!(
                "AutoGameModeEnabled={}，AllowAutoGameMode={}。",
                snapshot_label(left.as_ref()),
                snapshot_label(right.as_ref())
            ),
            recommended: "实验项，逐机测试".to_string(),
            requires_admin: false,
            requires_restart: false,
        },
        (left, right) => unknown_state(
            "读取 Game Mode 失败",
            join_non_empty(
                &[
                    left.err().unwrap_or_default(),
                    right.err().unwrap_or_default(),
                ],
                "; ",
            ),
            false,
            false,
            "实验项，逐机测试",
        ),
    }
}

fn fullscreen_optimization_state(game_path: Option<&str>) -> OptimizerSettingState {
    let Some(game_path) = game_path else {
        return OptimizerSettingState {
            key: "unknown".to_string(),
            label: "未选择游戏".to_string(),
            detail: "选择游戏 exe 后可检测。".to_string(),
            recommended: "按游戏单独禁用".to_string(),
            requires_admin: false,
            requires_restart: false,
        };
    };

    match read_registry_value(RegistryHive::CurrentUser, APP_COMPAT_LAYERS_PATH, game_path) {
        Ok(Some(RegistryValueSnapshot::String(value)))
            if has_compat_flag(&value, DISABLE_FULLSCREEN_OPT_FLAG) =>
        {
            OptimizerSettingState {
                key: "on".to_string(),
                label: "已禁用".to_string(),
                detail: value,
                recommended: "按游戏单独禁用".to_string(),
                requires_admin: false,
                requires_restart: false,
            }
        }
        Ok(Some(value)) => OptimizerSettingState {
            key: "off".to_string(),
            label: "未禁用".to_string(),
            detail: snapshot_label(Some(&value)),
            recommended: "按游戏单独禁用".to_string(),
            requires_admin: false,
            requires_restart: false,
        },
        Ok(None) => OptimizerSettingState {
            key: "off".to_string(),
            label: "未配置".to_string(),
            detail: "没有 AppCompatFlags Layers 记录。".to_string(),
            recommended: "按游戏单独禁用".to_string(),
            requires_admin: false,
            requires_restart: false,
        },
        Err(error) => unknown_state("读取全屏优化失败", error, false, false, "按游戏单独禁用"),
    }
}

fn unknown_state(
    label: &str,
    detail: String,
    requires_admin: bool,
    requires_restart: bool,
    recommended: &str,
) -> OptimizerSettingState {
    OptimizerSettingState {
        key: "unknown".to_string(),
        label: label.to_string(),
        detail,
        recommended: recommended.to_string(),
        requires_admin,
        requires_restart,
    }
}

#[derive(Clone, Copy)]
enum RegistryHive {
    CurrentUser,
    LocalMachine,
}

impl RegistryHive {
    fn as_str(self) -> &'static str {
        match self {
            RegistryHive::CurrentUser => "HKCU",
            RegistryHive::LocalMachine => "HKLM",
        }
    }

    fn from_str(value: &str) -> Result<Self, String> {
        match value {
            "HKCU" => Ok(RegistryHive::CurrentUser),
            "HKLM" => Ok(RegistryHive::LocalMachine),
            other => Err(format!("未知注册表根键：{other}")),
        }
    }
}

#[cfg(windows)]
fn root_key(hive: RegistryHive) -> RegKey {
    match hive {
        RegistryHive::CurrentUser => RegKey::predef(HKEY_CURRENT_USER),
        RegistryHive::LocalMachine => RegKey::predef(HKEY_LOCAL_MACHINE),
    }
}

#[cfg(windows)]
fn read_registry_value(
    hive: RegistryHive,
    path: &str,
    name: &str,
) -> Result<Option<RegistryValueSnapshot>, String> {
    let root = root_key(hive);
    let key = match root.open_subkey(path) {
        Ok(key) => key,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("无法打开注册表 {path}：{error}")),
    };

    match key.get_value::<u32, _>(name) {
        Ok(value) => return Ok(Some(RegistryValueSnapshot::U32(value))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => {}
    }
    match key.get_value::<String, _>(name) {
        Ok(value) => Ok(Some(RegistryValueSnapshot::String(value))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("无法读取注册表值 {path}\\{name}：{error}")),
    }
}

#[cfg(not(windows))]
fn read_registry_value(
    _hive: RegistryHive,
    _path: &str,
    _name: &str,
) -> Result<Option<RegistryValueSnapshot>, String> {
    Ok(None)
}

#[cfg(windows)]
fn apply_registry_change(change: &RegistryChangeRecord) -> Result<(), String> {
    let hive = RegistryHive::from_str(&change.hive)?;
    let root = root_key(hive);
    let (key, _) = root
        .create_subkey(&change.path)
        .map_err(|error| format!("无法创建或打开注册表 {}：{error}", change.path))?;

    match change.new_value.as_ref() {
        Some(RegistryValueSnapshot::U32(value)) => key
            .set_value(&change.name, value)
            .map_err(|error| format!("无法写入注册表值 {}：{error}", change.name)),
        Some(RegistryValueSnapshot::String(value)) => key
            .set_value(&change.name, value)
            .map_err(|error| format!("无法写入注册表值 {}：{error}", change.name)),
        None => match key.delete_value(&change.name) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(format!("无法删除注册表值 {}：{error}", change.name)),
        },
    }
}

#[cfg(not(windows))]
fn apply_registry_change(_change: &RegistryChangeRecord) -> Result<(), String> {
    Err("竞技优化器仅支持 Windows。".to_string())
}

fn restore_registry_value(change: &RegistryChangeRecord) -> Result<(), String> {
    let restore_change = RegistryChangeRecord {
        hive: change.hive.clone(),
        path: change.path.clone(),
        name: change.name.clone(),
        previous_value: change.new_value.clone(),
        new_value: change.previous_value.clone(),
    };
    apply_registry_change(&restore_change)
}

fn change_log_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|error| format!("无法定位应用配置目录：{error}"))?;
    Ok(dir.join(CHANGE_LOG_FILE))
}

fn load_change_log(app: &tauri::AppHandle) -> Result<Vec<OptimizerChangeRecord>, String> {
    let path = change_log_path(app)?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let text =
        fs::read_to_string(path).map_err(|error| format!("无法读取竞技模式变更记录：{error}"))?;
    serde_json::from_str(&text).map_err(|error| format!("竞技模式变更记录格式无效：{error}"))
}

fn save_change_log(
    app: &tauri::AppHandle,
    changes: &[OptimizerChangeRecord],
) -> Result<(), String> {
    let path = change_log_path(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("无法创建应用配置目录：{error}"))?;
    }
    let json = serde_json::to_string_pretty(changes)
        .map_err(|error| format!("无法保存竞技模式变更记录：{error}"))?;
    fs::write(path, json).map_err(|error| format!("无法写入竞技模式变更记录：{error}"))
}

fn append_change_log(
    app: &tauri::AppHandle,
    action: &str,
    requires_restart: bool,
    registry_changes: Vec<RegistryChangeRecord>,
) -> Result<(), String> {
    let mut changes = load_change_log(app)?;
    changes.push(OptimizerChangeRecord {
        id: format!(
            "{}-{}",
            std::process::id(),
            chrono::Local::now()
                .timestamp_nanos_opt()
                .unwrap_or_else(|| chrono::Local::now().timestamp_millis())
        ),
        applied_at: chrono::Local::now().to_rfc3339(),
        action: action.to_string(),
        requires_restart,
        registry_changes,
    });
    save_change_log(app, &changes)
}

fn normalize_optional_path(path: Option<String>) -> Option<String> {
    path.map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn snapshot_label(value: Option<&RegistryValueSnapshot>) -> String {
    match value {
        Some(RegistryValueSnapshot::U32(value)) => value.to_string(),
        Some(RegistryValueSnapshot::String(value)) => value.clone(),
        None => "未设置".to_string(),
    }
}

fn has_compat_flag(value: &str, flag: &str) -> bool {
    value
        .split_whitespace()
        .any(|part| part.eq_ignore_ascii_case(flag))
}

fn append_compat_flag(value: &str, flag: &str) -> String {
    if has_compat_flag(value, flag) {
        return value.trim().to_string();
    }
    join_non_empty(&[value.trim().to_string(), flag.to_string()], " ")
}

fn remove_compat_flag(value: &str, flag: &str) -> String {
    value
        .split_whitespace()
        .filter(|part| !part.eq_ignore_ascii_case(flag))
        .collect::<Vec<_>>()
        .join(" ")
}

fn is_process_running_for_path(path: &Path) -> bool {
    let Some(file_name) = path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
    else {
        return false;
    };
    let output = Command::new("tasklist")
        .args(["/FI", &format!("IMAGENAME eq {file_name}"), "/NH"])
        .output();
    let Ok(output) = output else {
        return false;
    };
    String::from_utf8_lossy(&output.stdout)
        .to_ascii_lowercase()
        .contains(&file_name.to_ascii_lowercase())
}

#[cfg(windows)]
fn is_administrator() -> bool {
    unsafe { IsUserAnAdmin() != 0 }
}

#[cfg(not(windows))]
fn is_administrator() -> bool {
    false
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

    #[test]
    fn compat_flag_append_is_idempotent() {
        assert_eq!(
            append_compat_flag("RUNASADMIN", DISABLE_FULLSCREEN_OPT_FLAG),
            "RUNASADMIN DISABLEDXMAXIMIZEDWINDOWEDMODE"
        );
        assert_eq!(
            append_compat_flag(
                "RUNASADMIN DISABLEDXMAXIMIZEDWINDOWEDMODE",
                DISABLE_FULLSCREEN_OPT_FLAG
            ),
            "RUNASADMIN DISABLEDXMAXIMIZEDWINDOWEDMODE"
        );
    }

    #[test]
    fn compat_flag_remove_preserves_other_flags() {
        assert_eq!(
            remove_compat_flag(
                "RUNASADMIN DISABLEDXMAXIMIZEDWINDOWEDMODE HIGHDPIAWARE",
                DISABLE_FULLSCREEN_OPT_FLAG
            ),
            "RUNASADMIN HIGHDPIAWARE"
        );
    }

    #[test]
    fn registry_snapshot_label_formats_missing_dword_and_string() {
        assert_eq!(snapshot_label(None), "未设置");
        assert_eq!(snapshot_label(Some(&RegistryValueSnapshot::U32(2))), "2");
        assert_eq!(
            snapshot_label(Some(&RegistryValueSnapshot::String("abc".to_string()))),
            "abc"
        );
    }
}
