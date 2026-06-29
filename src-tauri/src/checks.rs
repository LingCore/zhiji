use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use tauri::Manager;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

const CREATE_NO_WINDOW: u32 = 0x08000000;
const DEFAULT_COMMAND_TIMEOUT_SECS: u64 = 120;
const ADMIN_ACTION_TIMEOUT_SECS: u64 = 15 * 60;
static SNAPSHOT_CACHE: OnceLock<Mutex<Option<CheckSnapshot>>> = OnceLock::new();

#[derive(Debug, Serialize)]
pub struct HardwareInfo {
    board: String,
    bios: String,
    cpu: String,
}

#[derive(Debug, Serialize)]
pub struct CheckItem {
    name: String,
    required: String,
    status: String,
    passed: Option<bool>,
    detected: String,
    details: String,
    fix_hint: String,
}

#[derive(Debug, Serialize)]
pub struct CheckReport {
    computer_name: String,
    generated_at: String,
    is_administrator: bool,
    hardware: HardwareInfo,
    feature_states: FeatureStates,
    virtual_memory: VirtualMemoryInfo,
    blue_screen: BlueScreenInfo,
    results: Vec<CheckItem>,
}

#[derive(Debug, Default, Serialize)]
pub struct FeatureStates {
    hyper_v: String,
    virtual_machine_platform: String,
    windows_hypervisor_platform: String,
    hypervisor_launch: String,
    fast_startup: String,
    memory_compression: String,
}

#[derive(Debug, Serialize)]
pub struct ActionResult {
    action: String,
    succeeded: bool,
    requires_restart: bool,
    message: String,
    output: String,
}

#[derive(Debug, Serialize)]
pub struct VirtualMemoryInfo {
    total_physical_memory_mb: u64,
    automatic_managed_pagefile: Option<bool>,
    configured_state: String,
    system_drive: String,
    system_drive_total_mb: Option<u64>,
    system_drive_free_mb: Option<u64>,
    configured_pagefiles: Vec<PageFileConfigInfo>,
    pagefiles: Vec<PageFileInfo>,
    recommendation: PageFileRecommendation,
    details: String,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct PageFileConfigInfo {
    name: String,
    initial_size_mb: Option<u64>,
    maximum_size_mb: Option<u64>,
    source: String,
}

#[derive(Debug, Default, Serialize)]
pub struct PageFileInfo {
    name: String,
    initial_size_mb: Option<u64>,
    maximum_size_mb: Option<u64>,
    allocated_base_size_mb: Option<u64>,
    current_usage_mb: Option<u64>,
    peak_usage_mb: Option<u64>,
    temp_page_file: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct PageFileRecommendation {
    preferred_mode: String,
    recommended_initial_mb: Option<u64>,
    recommended_maximum_mb: Option<u64>,
    system_managed_min_estimate_mb: u64,
    system_managed_max_estimate_mb: u64,
    formula: String,
    reason: String,
}

#[derive(Debug, Serialize)]
pub struct BlueScreenInfo {
    crash_dump_enabled: Option<u32>,
    crash_dump_label: String,
    minidump_dir: String,
    minidump_dir_configured: bool,
    minidump_dir_exists: bool,
    dump_count: usize,
    recent_dumps: Vec<DumpFileInfo>,
    tool_path: Option<String>,
    tool_available: bool,
    collection_ready: bool,
    details: String,
}

#[derive(Debug, Serialize)]
pub struct DumpFileInfo {
    name: String,
    path: String,
    size_kb: u64,
    modified: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct InitialConfig {
    version: u32,
    saved_at: String,
    hyper_v: Option<String>,
    virtual_machine_platform: Option<String>,
    windows_hypervisor_platform: Option<String>,
    hypervisor_launch: Option<String>,
    #[serde(default)]
    hiberboot_enabled_present: Option<bool>,
    #[serde(default)]
    hiberboot_enabled: Option<bool>,
    #[serde(default)]
    hibernation_enabled: Option<bool>,
    #[serde(default)]
    memory_compression_enabled: Option<bool>,
    automatic_managed_pagefile: Option<bool>,
    pagefiles: Vec<SavedPageFileSetting>,
    #[serde(default)]
    crash_dump_enabled_present: Option<bool>,
    #[serde(default)]
    crash_dump_enabled: Option<u32>,
    #[serde(default)]
    minidump_dir_present: Option<bool>,
    #[serde(default)]
    minidump_dir: Option<String>,
    firmware_notice: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct SavedPageFileSetting {
    name: String,
    initial_size_mb: Option<u64>,
    maximum_size_mb: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct PageFileCustomRequest {
    initial_size_mb: u32,
    maximum_size_mb: u32,
}

#[derive(Debug, Default, Clone, Deserialize)]
struct CheckSnapshot {
    computer_name: Option<String>,
    is_administrator: Option<bool>,
    board: Option<String>,
    bios: Option<String>,
    cpu: Option<String>,
    cpu_manufacturer: Option<String>,
    virtualization_firmware_enabled: Option<bool>,
    hypervisor_present: Option<bool>,
    hyperv_state: Option<String>,
    hyperv_error: Option<String>,
    virtual_machine_platform_state: Option<String>,
    virtual_machine_platform_error: Option<String>,
    windows_hypervisor_platform_state: Option<String>,
    windows_hypervisor_platform_error: Option<String>,
    secure_boot_state: Option<String>,
    secure_boot_details: Option<String>,
    firmware_boot_mode: Option<String>,
    firmware_boot_details: Option<String>,
    tpm_state: Option<String>,
    tpm_present: Option<bool>,
    tpm_enabled: Option<bool>,
    tpm_details: Option<String>,
    vtd_available: Option<bool>,
    vtd_details: Option<String>,
    hypervisor_launch_type: Option<String>,
    hypervisor_launch_details: Option<String>,
    hiberboot_enabled_present: Option<bool>,
    hiberboot_enabled: Option<bool>,
    hibernation_enabled: Option<bool>,
    fast_startup_details: Option<String>,
    memory_compression_enabled: Option<bool>,
    memory_compression_details: Option<String>,
    total_physical_memory_mb: Option<u64>,
    automatic_managed_pagefile: Option<bool>,
    system_drive: Option<String>,
    system_drive_total_mb: Option<u64>,
    system_drive_free_mb: Option<u64>,
    pagefile_settings: Option<Vec<PageFileSettingSnapshot>>,
    pagefile_usage: Option<Vec<PageFileUsageSnapshot>>,
    registry_paging_files: Option<Vec<String>>,
    pagefile_details: Option<String>,
    crash_dump_enabled_present: Option<bool>,
    crash_dump_enabled: Option<u32>,
    minidump_dir_present: Option<bool>,
    minidump_dir: Option<String>,
    crash_control_details: Option<String>,
}

#[derive(Debug, Default, Clone, Deserialize)]
struct PageFileSettingSnapshot {
    name: Option<String>,
    initial_size_mb: Option<u64>,
    maximum_size_mb: Option<u64>,
}

#[derive(Debug, Default, Clone, Deserialize)]
struct PageFileUsageSnapshot {
    name: Option<String>,
    allocated_base_size_mb: Option<u64>,
    current_usage_mb: Option<u64>,
    peak_usage_mb: Option<u64>,
    temp_page_file: Option<bool>,
}

struct CommandOutput {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

#[tauri::command]
pub async fn run_checks(
    app: tauri::AppHandle,
    mode: Option<String>,
) -> Result<CheckReport, String> {
    let resource_dir = app.path().resource_dir().ok();
    let check_mode = CheckRunMode::from_option(mode);
    tauri::async_runtime::spawn_blocking(move || build_check_report(resource_dir, check_mode))
        .await
        .map_err(|error| format!("Check task failed: {error}"))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CheckRunMode {
    Fast,
    Full,
}

impl CheckRunMode {
    fn from_option(mode: Option<String>) -> Self {
        if mode
            .as_deref()
            .is_some_and(|value| value.eq_ignore_ascii_case("full"))
        {
            Self::Full
        } else {
            Self::Fast
        }
    }
}

fn build_check_report(resource_dir: Option<PathBuf>, mode: CheckRunMode) -> CheckReport {
    let snapshot = match collect_snapshot_for_mode(mode) {
        Ok(snapshot) => snapshot,
        Err(error) => return failed_snapshot_report(error, resource_dir.as_deref()),
    };

    let hardware = HardwareInfo {
        board: snapshot.board.clone().unwrap_or_else(|| "-".to_string()),
        bios: snapshot.bios.clone().unwrap_or_else(|| "-".to_string()),
        cpu: snapshot.cpu.clone().unwrap_or_else(|| "-".to_string()),
    };

    let results = vec![
        test_virtualization_snapshot(&snapshot),
        test_vtd_snapshot(&snapshot),
        test_csm_snapshot(&snapshot),
        test_hyper_v_snapshot(&snapshot),
        test_secure_boot_snapshot(&snapshot),
        test_tpm_snapshot(&snapshot),
        test_hypervisor_launch_snapshot(&snapshot),
    ];
    let _ = ensure_initial_config_saved(&snapshot);
    let feature_states = feature_states_from_snapshot(&snapshot);
    let virtual_memory = virtual_memory_from_snapshot(&snapshot);
    let blue_screen = blue_screen_from_snapshot(&snapshot, resource_dir.as_deref());

    CheckReport {
        computer_name: snapshot
            .computer_name
            .unwrap_or_else(|| std::env::var("COMPUTERNAME").unwrap_or_else(|_| "-".to_string())),
        generated_at: chrono::Local::now().to_rfc3339(),
        is_administrator: snapshot.is_administrator.unwrap_or(false),
        hardware,
        feature_states,
        virtual_memory,
        blue_screen,
        results,
    }
}

fn failed_snapshot_report(error: String, resource_dir: Option<&Path>) -> CheckReport {
    let results = vec![check_item(
        "System checks",
        "The app must be able to query Windows state",
        None,
        "Could not collect Windows state".to_string(),
        error.clone(),
        "重新检测；如果仍失败，请以管理员身份运行程序。",
    )];

    CheckReport {
        computer_name: std::env::var("COMPUTERNAME").unwrap_or_else(|_| "-".to_string()),
        generated_at: chrono::Local::now().to_rfc3339(),
        is_administrator: false,
        hardware: HardwareInfo {
            board: "-".to_string(),
            bios: "-".to_string(),
            cpu: "-".to_string(),
        },
        feature_states: FeatureStates::default(),
        virtual_memory: fallback_virtual_memory_info(error),
        blue_screen: fallback_blue_screen_info(resource_dir, "System checks failed.".to_string()),
        results,
    }
}

#[tauri::command]
pub fn restart_to_firmware() -> Result<(), String> {
    let output = run_program("shutdown.exe", &["/r", "/fw", "/t", "0"])?;
    if output.code == Some(0) {
        Ok(())
    } else {
        Err(join_non_empty(&[output.stderr, output.stdout], "; "))
    }
}

#[tauri::command]
pub fn set_virtual_memory_system_managed() -> Result<ActionResult, String> {
    ensure_initial_config_saved_from_current()?;
    run_admin_action(
        "set_virtual_memory_system_managed",
        r#"
$computerSystem = Get-CimInstance -ClassName Win32_ComputerSystem -ErrorAction Stop
$computerSystem | Set-CimInstance -Property @{ AutomaticManagedPagefile = $true } -ErrorAction Stop
"AutomaticManagedPagefile=True"
"Windows will choose page file size at startup based on RAM, commit charge, and crash dump settings."
"#,
        true,
        "已设置为系统管理虚拟内存。",
    )
}

#[tauri::command]
pub fn set_virtual_memory_custom(request: PageFileCustomRequest) -> Result<ActionResult, String> {
    ensure_initial_config_saved_from_current()?;
    if request.initial_size_mb == 0 {
        return Err("Initial size must be greater than 0 MB.".to_string());
    }
    if request.maximum_size_mb < request.initial_size_mb {
        return Err("Maximum size must be greater than or equal to initial size.".to_string());
    }

    let initial = request.initial_size_mb;
    let maximum = request.maximum_size_mb;
    let script = format!(
        r#"
$initial = {initial}
$maximum = {maximum}
$systemDrive = [Environment]::GetEnvironmentVariable('SystemDrive')
if ([string]::IsNullOrWhiteSpace($systemDrive)) {{ $systemDrive = 'C:' }}
$pageFile = Join-Path $systemDrive 'pagefile.sys'

$computerSystem = Get-CimInstance -ClassName Win32_ComputerSystem -ErrorAction Stop
$computerSystem | Set-CimInstance -Property @{{ AutomaticManagedPagefile = $false }} -ErrorAction Stop

$settings = @(Get-CimInstance -ClassName Win32_PageFileSetting -ErrorAction SilentlyContinue)
$existing = @($settings | Where-Object {{ $_.Name -ieq $pageFile }})
if ($existing.Count -gt 0) {{
  $existing | Set-CimInstance -Property @{{ InitialSize = $initial; MaximumSize = $maximum }} -ErrorAction Stop
}} else {{
  New-CimInstance -ClassName Win32_PageFileSetting -Property @{{ Name = $pageFile; InitialSize = $initial; MaximumSize = $maximum }} -ErrorAction Stop | Out-Null
}}

"AutomaticManagedPagefile=False"
"PageFile=$pageFile InitialSize=$initial MB MaximumSize=$maximum MB"
"#
    );

    run_admin_action(
        "set_virtual_memory_custom",
        &script,
        true,
        "已设置自定义虚拟内存。",
    )
}

#[tauri::command]
pub fn configure_minidump_collection() -> Result<ActionResult, String> {
    ensure_initial_config_saved_from_current()?;
    run_admin_action(
        "configure_minidump_collection",
        r#"
$crashControl = 'HKLM:\SYSTEM\CurrentControlSet\Control\CrashControl'
$minidumpDir = '%SystemRoot%\Minidump'
$expandedDir = [Environment]::ExpandEnvironmentVariables($minidumpDir)

New-Item -ItemType Directory -Path $expandedDir -Force | Out-Null
New-ItemProperty -LiteralPath $crashControl -Name CrashDumpEnabled -Value 3 -PropertyType DWord -Force | Out-Null
New-ItemProperty -LiteralPath $crashControl -Name MinidumpDir -Value $minidumpDir -PropertyType ExpandString -Force | Out-Null

"CrashDumpEnabled=3 (Small memory dump)"
"MinidumpDir=$minidumpDir"
"ExpandedMinidumpDir=$expandedDir"
"#,
        false,
        "已开启小内存转储并设置 Minidump 路径。",
    )
}

#[tauri::command]
pub fn open_bluescreenview(app: tauri::AppHandle) -> Result<ActionResult, String> {
    let resource_dir = app.path().resource_dir().ok();
    let tool = find_bluescreenview_tool(resource_dir.as_deref()).ok_or_else(|| {
        "找不到 BlueScreenView 工具。请确认 bluescreenview155.exe 已放入程序资源或桌面。"
            .to_string()
    })?;
    let dump_dir = default_minidump_path();

    let mut command = Command::new(&tool);
    command
        .arg("/LoadFrom")
        .arg("1")
        .arg("/MiniDumpFolder")
        .arg(&dump_dir);
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);
    command
        .spawn()
        .map_err(|error| format!("无法启动 BlueScreenView: {error}"))?;

    Ok(ActionResult {
        action: "open_bluescreenview".to_string(),
        succeeded: true,
        requires_restart: false,
        message: "已打开 BlueScreenView。".to_string(),
        output: format!(
            "Tool={}\nMiniDumpFolder={}",
            tool.to_string_lossy(),
            dump_dir.to_string_lossy()
        ),
    })
}

#[tauri::command]
pub fn export_bluescreen_report(app: tauri::AppHandle) -> Result<ActionResult, String> {
    let resource_dir = app.path().resource_dir().ok();
    let tool = find_bluescreenview_tool(resource_dir.as_deref()).ok_or_else(|| {
        "找不到 BlueScreenView 工具。请确认 bluescreenview155.exe 已放入程序资源或桌面。"
            .to_string()
    })?;
    let dump_dir = default_minidump_path();
    let export_path = std::env::temp_dir().join(format!(
        "pc_requirements_bluescreen_{}.csv",
        chrono::Local::now().format("%Y%m%d_%H%M%S")
    ));

    let mut command = Command::new(&tool);
    command
        .arg("/LoadFrom")
        .arg("1")
        .arg("/MiniDumpFolder")
        .arg(&dump_dir)
        .arg("/scomma")
        .arg(&export_path);
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);
    let output = command
        .output()
        .map_err(|error| format!("无法导出 BlueScreenView 报告: {error}"))?;

    if output.status.success() && export_path.exists() {
        Ok(ActionResult {
            action: "export_bluescreen_report".to_string(),
            succeeded: true,
            requires_restart: false,
            message: "已导出蓝屏 CSV 报告。".to_string(),
            output: export_path.to_string_lossy().to_string(),
        })
    } else {
        Err(join_non_empty(
            &[
                String::from_utf8_lossy(&output.stderr).trim().to_string(),
                String::from_utf8_lossy(&output.stdout).trim().to_string(),
                format!("Exit code: {:?}", output.status.code()),
            ],
            "\n",
        ))
    }
}

#[tauri::command]
pub fn restore_initial_config() -> Result<ActionResult, String> {
    let config = load_initial_config()?;
    let config_json = serde_json::to_string(&config)
        .map_err(|error| format!("Failed to encode initial config: {error}"))?;
    let config_json = escape_ps_single(&config_json);

    let script = format!(
        r#"
$restore = '{config_json}' | ConvertFrom-Json
$messages = New-Object System.Collections.Generic.List[string]

function Add-Message([string]$message) {{
  if (-not [string]::IsNullOrWhiteSpace($message)) {{
    $messages.Add($message)
  }}
}}

function Restore-Feature([string]$label, [string]$enableName, [string]$disableName, [string]$state, [bool]$enableAll) {{
  if ([string]::IsNullOrWhiteSpace($state) -or $state -match '^Unknown') {{
    Add-Message "$label skipped: initial state unknown"
    return
  }}

  if ($state -match 'Enabled') {{
    if ($enableAll) {{
      Enable-WindowsOptionalFeature -Online -FeatureName $enableName -All -NoRestart -ErrorAction Stop | Out-Null
    }} else {{
      Enable-WindowsOptionalFeature -Online -FeatureName $enableName -NoRestart -ErrorAction Stop | Out-Null
    }}
    Add-Message "$label restored to Enabled"
  }} elseif ($state -match 'Disabled') {{
    Disable-WindowsOptionalFeature -Online -FeatureName $disableName -NoRestart -ErrorAction Stop | Out-Null
    Add-Message "$label restored to Disabled"
  }} else {{
    Add-Message "$label skipped: unsupported initial state $state"
  }}
}}

Restore-Feature 'Microsoft Hyper-V' 'Microsoft-Hyper-V' 'Microsoft-Hyper-V-All' ([string]$restore.hyper_v) $true
Restore-Feature 'Virtual Machine Platform' 'VirtualMachinePlatform' 'VirtualMachinePlatform' ([string]$restore.virtual_machine_platform) $false
Restore-Feature 'Windows Hypervisor Platform' 'HypervisorPlatform' 'HypervisorPlatform' ([string]$restore.windows_hypervisor_platform) $false

$launchType = [string]$restore.hypervisor_launch
if (-not [string]::IsNullOrWhiteSpace($launchType) -and $launchType -match '^(?i:auto|off)$') {{
  function Get-BcdTarget {{
    $current = & bcdedit.exe /enum '{{current}}' 2>&1
    if ($LASTEXITCODE -eq 0) {{ return '{{current}}' }}
    $default = & bcdedit.exe /enum '{{default}}' 2>&1
    if ($LASTEXITCODE -eq 0) {{ return '{{default}}' }}
    throw "Could not find a usable BCD boot loader entry. current=$($current | Out-String) default=$($default | Out-String)"
  }}
  $bcdTarget = Get-BcdTarget
  $bcd = & bcdedit.exe /set $bcdTarget hypervisorlaunchtype $launchType 2>&1
  if ($LASTEXITCODE -ne 0) {{ throw (($bcd | Out-String).Trim()) }}
  Add-Message "Hypervisor launch restored to $launchType on $bcdTarget"
}} else {{
  Add-Message "Hypervisor launch skipped: initial state unknown or unsupported"
}}

$hasHibernateState = $null -ne $restore.hibernation_enabled
if ($hasHibernateState) {{
  $hibernateShouldBeEnabled = [System.Convert]::ToBoolean($restore.hibernation_enabled)
  $powercfgArgs = if ($hibernateShouldBeEnabled) {{ @('/hibernate', 'on') }} else {{ @('/hibernate', 'off') }}
  $hibernate = & powercfg.exe @powercfgArgs 2>&1
  if ($LASTEXITCODE -ne 0) {{ throw (($hibernate | Out-String).Trim()) }}
  Add-Message "Hibernation restored to $(if ($hibernateShouldBeEnabled) {{ 'Enabled' }} else {{ 'Disabled' }})"
}} else {{
  Add-Message "Hibernation skipped: initial state unknown"
}}

$hasHiberbootState = $null -ne $restore.hiberboot_enabled_present
if ($hasHiberbootState) {{
  $sessionPower = 'HKLM:\SYSTEM\CurrentControlSet\Control\Session Manager\Power'
  New-Item -Path $sessionPower -Force -ErrorAction Stop | Out-Null
  if ([System.Convert]::ToBoolean($restore.hiberboot_enabled_present)) {{
    $hiberbootValue = if ([System.Convert]::ToBoolean($restore.hiberboot_enabled)) {{ 1 }} else {{ 0 }}
    New-ItemProperty -LiteralPath $sessionPower -Name HiberbootEnabled -Value $hiberbootValue -PropertyType DWord -Force -ErrorAction Stop | Out-Null
    Add-Message "Fast startup restored to $(if ($hiberbootValue -eq 1) {{ 'Enabled' }} else {{ 'Disabled' }})"
  }} else {{
    Remove-ItemProperty -LiteralPath $sessionPower -Name HiberbootEnabled -ErrorAction SilentlyContinue
    Add-Message "Fast startup registry value removed to match the initial state"
  }}
}} else {{
  Add-Message "Fast startup skipped: initial state unknown"
}}

if ($null -ne $restore.memory_compression_enabled) {{
  $memoryCompression = [System.Convert]::ToBoolean($restore.memory_compression_enabled)
  if ($memoryCompression) {{
    Enable-MMAgent -MemoryCompression -ErrorAction Stop
    Add-Message "Memory Compression restored to Enabled"
  }} else {{
    Disable-MMAgent -MemoryCompression -ErrorAction Stop
    Add-Message "Memory Compression restored to Disabled"
  }}
}} else {{
  Add-Message "Memory Compression skipped: initial state unknown"
}}

if ($null -ne $restore.automatic_managed_pagefile) {{
  $auto = [System.Convert]::ToBoolean($restore.automatic_managed_pagefile)
  $computerSystem = Get-CimInstance -ClassName Win32_ComputerSystem -ErrorAction Stop
  $computerSystem | Set-CimInstance -Property @{{ AutomaticManagedPagefile = $auto }} -ErrorAction Stop

  if ($auto) {{
    Add-Message "Virtual memory restored to system managed"
  }} else {{
    $targets = @($restore.pagefiles)
    $current = @(Get-CimInstance -ClassName Win32_PageFileSetting -ErrorAction SilentlyContinue)
    foreach ($item in $current) {{
      $match = @($targets | Where-Object {{ [string]$_.name -ieq [string]$item.Name }})
      if ($match.Count -eq 0) {{
        $item | Remove-CimInstance -ErrorAction Stop
      }}
    }}

    foreach ($target in $targets) {{
      $name = [string]$target.name
      if ([string]::IsNullOrWhiteSpace($name)) {{ continue }}
      $initial = if ($null -eq $target.initial_size_mb) {{ 0 }} else {{ [uint32]$target.initial_size_mb }}
      $maximum = if ($null -eq $target.maximum_size_mb) {{ 0 }} else {{ [uint32]$target.maximum_size_mb }}
      $existing = @(Get-CimInstance -ClassName Win32_PageFileSetting -ErrorAction SilentlyContinue | Where-Object {{ $_.Name -ieq $name }})
      if ($existing.Count -gt 0) {{
        $existing | Set-CimInstance -Property @{{ InitialSize = $initial; MaximumSize = $maximum }} -ErrorAction Stop
      }} else {{
        New-CimInstance -ClassName Win32_PageFileSetting -Property @{{ Name = $name; InitialSize = $initial; MaximumSize = $maximum }} -ErrorAction Stop | Out-Null
      }}
    }}
    Add-Message "Virtual memory restored to initial custom pagefile settings"
  }}
}} else {{
  Add-Message "Virtual memory skipped: initial state unknown"
}}

$hasCrashDumpEnabledState = $null -ne $restore.crash_dump_enabled_present
$hasMinidumpDirState = $null -ne $restore.minidump_dir_present
if ($hasCrashDumpEnabledState -or $hasMinidumpDirState) {{
  $crashControl = 'HKLM:\SYSTEM\CurrentControlSet\Control\CrashControl'
  New-Item -Path $crashControl -Force -ErrorAction Stop | Out-Null

  if ($hasCrashDumpEnabledState) {{
    if ([System.Convert]::ToBoolean($restore.crash_dump_enabled_present)) {{
      $enabled = if ($null -eq $restore.crash_dump_enabled) {{ 0 }} else {{ [uint32]$restore.crash_dump_enabled }}
      New-ItemProperty -LiteralPath $crashControl -Name CrashDumpEnabled -Value $enabled -PropertyType DWord -Force -ErrorAction Stop | Out-Null
    }} else {{
      Remove-ItemProperty -LiteralPath $crashControl -Name CrashDumpEnabled -ErrorAction SilentlyContinue
    }}
  }}

  if ($hasMinidumpDirState) {{
    if ([System.Convert]::ToBoolean($restore.minidump_dir_present)) {{
      $minidumpDir = [string]$restore.minidump_dir
      New-ItemProperty -LiteralPath $crashControl -Name MinidumpDir -Value $minidumpDir -PropertyType ExpandString -Force -ErrorAction Stop | Out-Null
      if (-not [string]::IsNullOrWhiteSpace($minidumpDir)) {{
        $expandedDir = [Environment]::ExpandEnvironmentVariables($minidumpDir)
        New-Item -ItemType Directory -Path $expandedDir -Force -ErrorAction SilentlyContinue | Out-Null
      }}
    }} else {{
      Remove-ItemProperty -LiteralPath $crashControl -Name MinidumpDir -ErrorAction SilentlyContinue
    }}
  }}

  Add-Message "DMP collection restored to initial CrashControl settings"
}} else {{
  Add-Message "DMP collection skipped: initial state unknown"
}}

Add-Message ([string]$restore.firmware_notice)
$messages -join "`n"
"#
    );

    run_admin_action(
        "restore_initial_config",
        &script,
        true,
        "已恢复本软件支持的初始配置。",
    )
}

#[tauri::command]
pub fn apply_requirement_action(action: String) -> Result<ActionResult, String> {
    if action != "restart_windows" {
        ensure_initial_config_saved_from_current()?;
    }

    match action.as_str() {
        "enable_hyper_v" => run_admin_action(
            "enable_hyper_v",
            r#"
$feature = Enable-WindowsOptionalFeature -Online -FeatureName Microsoft-Hyper-V -All -NoRestart -ErrorAction Stop
"Hyper-V RestartNeeded=$($feature.RestartNeeded)"
"#,
            true,
            "已执行：启用 Hyper-V。",
        ),
        "disable_hyper_v" => run_admin_action(
            "disable_hyper_v",
            r#"
$feature = Disable-WindowsOptionalFeature -Online -FeatureName Microsoft-Hyper-V-All -NoRestart -ErrorAction Stop
"Hyper-V RestartNeeded=$($feature.RestartNeeded)"
"#,
            true,
            "已执行：禁用 Hyper-V。",
        ),
        "enable_virtual_machine_platform" => run_admin_action(
            "enable_virtual_machine_platform",
            r#"
$feature = Enable-WindowsOptionalFeature -Online -FeatureName VirtualMachinePlatform -NoRestart -ErrorAction Stop
"VirtualMachinePlatform RestartNeeded=$($feature.RestartNeeded)"
"#,
            true,
            "已执行：启用 Virtual Machine Platform。",
        ),
        "disable_virtual_machine_platform" => run_admin_action(
            "disable_virtual_machine_platform",
            r#"
$feature = Disable-WindowsOptionalFeature -Online -FeatureName VirtualMachinePlatform -NoRestart -ErrorAction Stop
"VirtualMachinePlatform RestartNeeded=$($feature.RestartNeeded)"
"#,
            true,
            "已执行：禁用 Virtual Machine Platform。",
        ),
        "enable_windows_hypervisor_platform" => run_admin_action(
            "enable_windows_hypervisor_platform",
            r#"
$feature = Enable-WindowsOptionalFeature -Online -FeatureName HypervisorPlatform -NoRestart -ErrorAction Stop
"HypervisorPlatform RestartNeeded=$($feature.RestartNeeded)"
"#,
            true,
            "已执行：启用 Windows Hypervisor Platform。",
        ),
        "disable_windows_hypervisor_platform" => run_admin_action(
            "disable_windows_hypervisor_platform",
            r#"
$feature = Disable-WindowsOptionalFeature -Online -FeatureName HypervisorPlatform -NoRestart -ErrorAction Stop
"HypervisorPlatform RestartNeeded=$($feature.RestartNeeded)"
"#,
            true,
            "已执行：禁用 Windows Hypervisor Platform。",
        ),
        "set_hypervisor_auto" => run_admin_action(
            "set_hypervisor_auto",
            r#"
function Get-BcdTarget {
  $current = & bcdedit.exe /enum '{current}' 2>&1
  if ($LASTEXITCODE -eq 0) { return '{current}' }
  $default = & bcdedit.exe /enum '{default}' 2>&1
  if ($LASTEXITCODE -eq 0) { return '{default}' }
  throw "Could not find a usable BCD boot loader entry. current=$($current | Out-String) default=$($default | Out-String)"
}
$bcdTarget = Get-BcdTarget
$bcd = & bcdedit.exe /set $bcdTarget hypervisorlaunchtype auto 2>&1
if ($LASTEXITCODE -ne 0) { throw (($bcd | Out-String).Trim()) }
"Target=$bcdTarget"
($bcd | Out-String).Trim()
"#,
            true,
            "已执行：hypervisorlaunchtype 设置为 auto。",
        ),
        "set_hypervisor_off" => run_admin_action(
            "set_hypervisor_off",
            r#"
function Get-BcdTarget {
  $current = & bcdedit.exe /enum '{current}' 2>&1
  if ($LASTEXITCODE -eq 0) { return '{current}' }
  $default = & bcdedit.exe /enum '{default}' 2>&1
  if ($LASTEXITCODE -eq 0) { return '{default}' }
  throw "Could not find a usable BCD boot loader entry. current=$($current | Out-String) default=$($default | Out-String)"
}
$bcdTarget = Get-BcdTarget
$bcd = & bcdedit.exe /set $bcdTarget hypervisorlaunchtype off 2>&1
if ($LASTEXITCODE -ne 0) { throw (($bcd | Out-String).Trim()) }
"Target=$bcdTarget"
($bcd | Out-String).Trim()
"#,
            true,
            "已执行：hypervisorlaunchtype 设置为 off。",
        ),
        "enable_fast_startup" => run_admin_action(
            "enable_fast_startup",
            r#"
$sessionPower = 'HKLM:\SYSTEM\CurrentControlSet\Control\Session Manager\Power'
New-Item -Path $sessionPower -Force -ErrorAction Stop | Out-Null
$messages = New-Object System.Collections.Generic.List[string]

function Add-Message([string]$message) {
  if (-not [string]::IsNullOrWhiteSpace($message)) {
    $messages.Add($message)
  }
}

function Get-HibernationEnabled {
  try {
    $controlPower = Get-ItemProperty -LiteralPath 'HKLM:\SYSTEM\CurrentControlSet\Control\Power' -ErrorAction Stop
    $names = @($controlPower.PSObject.Properties.Name)
    if ($names -contains 'HibernateEnabled') {
      return ([uint32]$controlPower.HibernateEnabled) -ne 0
    }
    if ($names -contains 'HibernateEnabledDefault') {
      return ([uint32]$controlPower.HibernateEnabledDefault) -ne 0
    }
  } catch {
    Add-Message "Could not read hibernation registry state: $($_.Exception.Message)"
  }
  return $null
}

$hibernationEnabled = Get-HibernationEnabled
if ($hibernationEnabled -eq $false) {
  $reduced = & powercfg.exe /h /type reduced 2>&1
  if ($LASTEXITCODE -ne 0) {
    Add-Message "powercfg /h /type reduced failed: $(($reduced | Out-String).Trim())"
    $size = & powercfg.exe /h /size 0 2>&1
    if ($LASTEXITCODE -ne 0) {
      Add-Message "powercfg /h /size 0 failed: $(($size | Out-String).Trim())"
    } else {
      $reduced = & powercfg.exe /h /type reduced 2>&1
    }
  }

  if ($LASTEXITCODE -ne 0) {
    $full = & powercfg.exe /h on 2>&1
    if ($LASTEXITCODE -ne 0) {
      throw "Could not enable hibernation for Fast Startup: $(($full | Out-String).Trim())"
    }
    Add-Message "Hibernation enabled for Fast Startup."
  } else {
    Add-Message "Reduced hibernation file enabled for Fast Startup."
  }
} elseif ($null -eq $hibernationEnabled) {
  $hibernate = & powercfg.exe /h on 2>&1
  if ($LASTEXITCODE -ne 0) {
    throw "Could not enable hibernation for Fast Startup: $(($hibernate | Out-String).Trim())"
  }
  Add-Message "Hibernation enabled for Fast Startup."
} else {
  Add-Message "Hibernation is already enabled."
}

New-ItemProperty -LiteralPath $sessionPower -Name HiberbootEnabled -Value 1 -PropertyType DWord -Force -ErrorAction Stop | Out-Null
Add-Message "HiberbootEnabled=1"
$messages -join "`n"
"#,
            false,
            "已执行：开启快速启动。",
        ),
        "disable_fast_startup" => run_admin_action(
            "disable_fast_startup",
            r#"
$sessionPower = 'HKLM:\SYSTEM\CurrentControlSet\Control\Session Manager\Power'
New-Item -Path $sessionPower -Force -ErrorAction Stop | Out-Null
New-ItemProperty -LiteralPath $sessionPower -Name HiberbootEnabled -Value 0 -PropertyType DWord -Force -ErrorAction Stop | Out-Null
"HiberbootEnabled=0"
"Hibernation was left unchanged."
"#,
            false,
            "已执行：关闭快速启动。",
        ),
        "enable_memory_compression" => run_admin_action(
            "enable_memory_compression",
            r#"
$before = Get-MMAgent -ErrorAction Stop
"Before MemoryCompression=$($before.MemoryCompression)"
Enable-MMAgent -MemoryCompression -ErrorAction Stop
$after = Get-MMAgent -ErrorAction Stop
"After MemoryCompression=$($after.MemoryCompression)"
"Restart Windows to make sure the setting is fully applied."
"#,
            true,
            "已执行：启用内存压缩。",
        ),
        "disable_memory_compression" => run_admin_action(
            "disable_memory_compression",
            r#"
$before = Get-MMAgent -ErrorAction Stop
"Before MemoryCompression=$($before.MemoryCompression)"
Disable-MMAgent -MemoryCompression -ErrorAction Stop
$after = Get-MMAgent -ErrorAction Stop
"After MemoryCompression=$($after.MemoryCompression)"
"Restart Windows to make sure the setting is fully applied."
"#,
            true,
            "已执行：禁用内存压缩。",
        ),
        "restart_windows" => {
            let output = run_program("shutdown.exe", &["/r", "/t", "0"])?;
            if output.code == Some(0) {
                Ok(ActionResult {
                    action,
                    succeeded: true,
                    requires_restart: false,
                    message: "已请求立即重启。".to_string(),
                    output: join_non_empty(&[output.stdout, output.stderr], "\n"),
                })
            } else {
                Err(join_non_empty(&[output.stderr, output.stdout], "\n"))
            }
        }
        other => Err(format!("Unknown action: {other}")),
    }
}

fn check_item(
    name: &str,
    required: &str,
    passed: Option<bool>,
    detected: String,
    details: String,
    fix_hint: &str,
) -> CheckItem {
    let status = match passed {
        Some(true) => "PASS",
        Some(false) => "FAIL",
        None => "UNKNOWN",
    };

    CheckItem {
        name: name.to_string(),
        required: required.to_string(),
        status: status.to_string(),
        passed,
        detected,
        details,
        fix_hint: fix_hint.to_string(),
    }
}

fn ensure_initial_config_saved_from_current() -> Result<(), String> {
    let path = initial_config_path()?;
    if path.exists() {
        return Ok(());
    }

    let snapshot = collect_snapshot()?;
    ensure_initial_config_saved(&snapshot)
}

fn ensure_initial_config_saved(snapshot: &CheckSnapshot) -> Result<(), String> {
    let path = initial_config_path()?;
    if path.exists() {
        return ensure_initial_config_has_current_fields(&path, snapshot);
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Failed to create initial config directory: {error}"))?;
    }

    let config = initial_config_from_snapshot(snapshot);
    let json = serde_json::to_string_pretty(&config)
        .map_err(|error| format!("Failed to serialize initial config: {error}"))?;
    let temp_path = path.with_extension("json.tmp");

    fs::write(&temp_path, json)
        .map_err(|error| format!("Failed to write initial config: {error}"))?;

    match fs::rename(&temp_path, &path) {
        Ok(()) => Ok(()),
        Err(error) if path.exists() => {
            let _ = fs::remove_file(&temp_path);
            let _ = error;
            Ok(())
        }
        Err(error) => Err(format!("Failed to save initial config: {error}")),
    }
}

fn ensure_initial_config_has_current_fields(
    path: &Path,
    snapshot: &CheckSnapshot,
) -> Result<(), String> {
    let json = fs::read_to_string(path).map_err(|error| {
        format!(
            "Initial config exists but could not be read: {error}. Path: {}",
            path.display()
        )
    })?;
    let mut value: serde_json::Value = serde_json::from_str(&json).map_err(|error| {
        format!(
            "Initial config exists but is unreadable: {error}. Path: {}",
            path.display()
        )
    })?;

    let Some(object) = value.as_object_mut() else {
        return Err(format!(
            "Initial config is not a JSON object. Path: {}",
            path.display()
        ));
    };

    let mut changed = false;
    changed |= insert_json_field_if_missing(
        object,
        "crash_dump_enabled_present",
        serde_json::to_value(snapshot.crash_dump_enabled_present)
            .unwrap_or(serde_json::Value::Null),
    );
    changed |= insert_json_field_if_missing(
        object,
        "crash_dump_enabled",
        serde_json::to_value(snapshot.crash_dump_enabled).unwrap_or(serde_json::Value::Null),
    );
    changed |= insert_json_field_if_missing(
        object,
        "minidump_dir_present",
        serde_json::to_value(snapshot.minidump_dir_present).unwrap_or(serde_json::Value::Null),
    );
    changed |= insert_json_field_if_missing(
        object,
        "minidump_dir",
        serde_json::to_value(snapshot.minidump_dir.clone()).unwrap_or(serde_json::Value::Null),
    );
    changed |= insert_json_field_if_missing(
        object,
        "hiberboot_enabled_present",
        serde_json::to_value(snapshot.hiberboot_enabled_present).unwrap_or(serde_json::Value::Null),
    );
    changed |= insert_json_field_if_missing(
        object,
        "hiberboot_enabled",
        serde_json::to_value(snapshot.hiberboot_enabled).unwrap_or(serde_json::Value::Null),
    );
    changed |= insert_json_field_if_missing(
        object,
        "hibernation_enabled",
        serde_json::to_value(snapshot.hibernation_enabled).unwrap_or(serde_json::Value::Null),
    );
    changed |= insert_json_field_if_missing(
        object,
        "memory_compression_enabled",
        serde_json::to_value(snapshot.memory_compression_enabled)
            .unwrap_or(serde_json::Value::Null),
    );
    let firmware_notice =
        "BIOS/UEFI firmware items were not restored automatically: CPU virtualization, VT-d/IOMMU, Secure Boot, and TPM/PTT/fTPM must be changed in firmware setup.";
    if object
        .get("firmware_notice")
        .and_then(|value| value.as_str())
        .map(|notice| !notice.contains("VT-d/IOMMU"))
        .unwrap_or(true)
    {
        object.insert(
            "firmware_notice".to_string(),
            serde_json::Value::String(firmware_notice.to_string()),
        );
        changed = true;
    }

    if !changed {
        return Ok(());
    }

    let updated = serde_json::to_string_pretty(&value)
        .map_err(|error| format!("Failed to serialize initial config: {error}"))?;
    let temp_path = path.with_extension("json.tmp");
    fs::write(&temp_path, updated)
        .map_err(|error| format!("Failed to update initial config: {error}"))?;
    fs::rename(&temp_path, path)
        .map_err(|error| format!("Failed to replace initial config: {error}"))
}

fn insert_json_field_if_missing(
    object: &mut serde_json::Map<String, serde_json::Value>,
    key: &str,
    value: serde_json::Value,
) -> bool {
    if object.get(key).is_some_and(|current| !current.is_null()) {
        return false;
    }

    object.insert(key.to_string(), value);
    true
}

fn snapshot_cache() -> &'static Mutex<Option<CheckSnapshot>> {
    SNAPSHOT_CACHE.get_or_init(|| Mutex::new(None))
}

fn cached_snapshot() -> Option<CheckSnapshot> {
    snapshot_cache()
        .lock()
        .ok()
        .and_then(|snapshot| snapshot.clone())
}

fn store_cached_snapshot(snapshot: &CheckSnapshot) {
    if let Ok(mut cached) = snapshot_cache().lock() {
        *cached = Some(snapshot.clone());
    }
}

fn collect_snapshot_for_mode(mode: CheckRunMode) -> Result<CheckSnapshot, String> {
    match mode {
        CheckRunMode::Full => {
            let snapshot = collect_snapshot()?;
            store_cached_snapshot(&snapshot);
            Ok(snapshot)
        }
        CheckRunMode::Fast => {
            let Some(cached) = cached_snapshot() else {
                let snapshot = collect_snapshot()?;
                store_cached_snapshot(&snapshot);
                return Ok(snapshot);
            };

            let fast = collect_fast_snapshot()?;
            let merged = merge_fast_snapshot(cached, fast);
            store_cached_snapshot(&merged);
            Ok(merged)
        }
    }
}

fn merge_fast_snapshot(mut cached: CheckSnapshot, fast: CheckSnapshot) -> CheckSnapshot {
    cached.computer_name = fast.computer_name.or(cached.computer_name);
    cached.is_administrator = fast.is_administrator.or(cached.is_administrator);
    cached.hypervisor_present = fast.hypervisor_present.or(cached.hypervisor_present);
    cached.hyperv_state = fast.hyperv_state.or(cached.hyperv_state);
    cached.hyperv_error = fast.hyperv_error.or(cached.hyperv_error);
    cached.virtual_machine_platform_state = fast
        .virtual_machine_platform_state
        .or(cached.virtual_machine_platform_state);
    cached.virtual_machine_platform_error = fast
        .virtual_machine_platform_error
        .or(cached.virtual_machine_platform_error);
    cached.windows_hypervisor_platform_state = fast
        .windows_hypervisor_platform_state
        .or(cached.windows_hypervisor_platform_state);
    cached.windows_hypervisor_platform_error = fast
        .windows_hypervisor_platform_error
        .or(cached.windows_hypervisor_platform_error);
    cached.hypervisor_launch_type = fast
        .hypervisor_launch_type
        .or(cached.hypervisor_launch_type);
    cached.hypervisor_launch_details = fast
        .hypervisor_launch_details
        .or(cached.hypervisor_launch_details);
    cached.hiberboot_enabled_present = fast
        .hiberboot_enabled_present
        .or(cached.hiberboot_enabled_present);
    cached.hiberboot_enabled = fast.hiberboot_enabled.or(cached.hiberboot_enabled);
    cached.hibernation_enabled = fast.hibernation_enabled.or(cached.hibernation_enabled);
    cached.fast_startup_details = fast.fast_startup_details.or(cached.fast_startup_details);
    cached.memory_compression_enabled = fast
        .memory_compression_enabled
        .or(cached.memory_compression_enabled);
    cached.memory_compression_details = fast
        .memory_compression_details
        .or(cached.memory_compression_details);
    cached.total_physical_memory_mb = fast
        .total_physical_memory_mb
        .or(cached.total_physical_memory_mb);
    cached.automatic_managed_pagefile = fast
        .automatic_managed_pagefile
        .or(cached.automatic_managed_pagefile);
    cached.system_drive = fast.system_drive.or(cached.system_drive);
    cached.system_drive_total_mb = fast.system_drive_total_mb.or(cached.system_drive_total_mb);
    cached.system_drive_free_mb = fast.system_drive_free_mb.or(cached.system_drive_free_mb);
    cached.pagefile_settings = fast.pagefile_settings.or(cached.pagefile_settings);
    cached.pagefile_usage = fast.pagefile_usage.or(cached.pagefile_usage);
    cached.registry_paging_files = fast.registry_paging_files.or(cached.registry_paging_files);
    cached.pagefile_details = fast.pagefile_details.or(cached.pagefile_details);
    cached.crash_dump_enabled_present = fast
        .crash_dump_enabled_present
        .or(cached.crash_dump_enabled_present);
    cached.crash_dump_enabled = fast.crash_dump_enabled.or(cached.crash_dump_enabled);
    cached.minidump_dir_present = fast.minidump_dir_present.or(cached.minidump_dir_present);
    cached.minidump_dir = fast.minidump_dir.or(cached.minidump_dir);
    cached.crash_control_details = fast.crash_control_details.or(cached.crash_control_details);
    cached
}

fn load_initial_config() -> Result<InitialConfig, String> {
    let path = initial_config_path()?;
    let json = fs::read_to_string(&path).map_err(|error| {
        format!(
            "Initial config was not found. Open the app once before making changes so it can save the original state. Path: {}. Error: {error}",
            path.display()
        )
    })?;

    serde_json::from_str(&json).map_err(|error| {
        format!(
            "Initial config is unreadable: {error}. Path: {}",
            path.display()
        )
    })
}

fn initial_config_path() -> Result<PathBuf, String> {
    let base = std::env::var_os("LOCALAPPDATA")
        .or_else(|| std::env::var_os("APPDATA"))
        .ok_or_else(|| {
            "Could not locate LOCALAPPDATA or APPDATA for initial config storage.".to_string()
        })?;

    Ok(PathBuf::from(base).join("知机").join("initial_config.json"))
}

fn initial_config_from_snapshot(snapshot: &CheckSnapshot) -> InitialConfig {
    let pagefiles = configured_pagefiles_from_snapshot(snapshot)
        .into_iter()
        .map(|setting| SavedPageFileSetting {
            name: setting.name,
            initial_size_mb: setting.initial_size_mb,
            maximum_size_mb: setting.maximum_size_mb,
        })
        .collect();

    InitialConfig {
        version: 1,
        saved_at: chrono::Local::now().to_rfc3339(),
        hyper_v: Some(summarize_hyper_v_state(snapshot)),
        virtual_machine_platform: snapshot.virtual_machine_platform_state.clone(),
        windows_hypervisor_platform: snapshot.windows_hypervisor_platform_state.clone(),
        hypervisor_launch: snapshot.hypervisor_launch_type.clone(),
        hiberboot_enabled_present: snapshot.hiberboot_enabled_present,
        hiberboot_enabled: snapshot.hiberboot_enabled,
        hibernation_enabled: snapshot.hibernation_enabled,
        memory_compression_enabled: snapshot.memory_compression_enabled,
        automatic_managed_pagefile: snapshot.automatic_managed_pagefile,
        pagefiles,
        crash_dump_enabled_present: snapshot.crash_dump_enabled_present,
        crash_dump_enabled: snapshot.crash_dump_enabled,
        minidump_dir_present: snapshot.minidump_dir_present,
        minidump_dir: snapshot.minidump_dir.clone(),
        firmware_notice:
            "BIOS/UEFI firmware items were not restored automatically: CPU virtualization, VT-d/IOMMU, Secure Boot, and TPM/PTT/fTPM must be changed in firmware setup."
                .to_string(),
    }
}

fn collect_snapshot() -> Result<CheckSnapshot, String> {
    let script = r#"
$ErrorActionPreference = 'SilentlyContinue'
$ProgressPreference = 'SilentlyContinue'
$result = [ordered]@{}

function Add-Value($name, $value) {
  $result[$name] = $value
}

Add-Value 'computer_name' $env:COMPUTERNAME

try {
  $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
  $principal = New-Object Security.Principal.WindowsPrincipal($identity)
  Add-Value 'is_administrator' ([bool]$principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator))
} catch {
  Add-Value 'is_administrator' $false
}

try {
  $board = Get-CimInstance Win32_BaseBoard | Select-Object -First 1
  Add-Value 'board' ("$($board.Manufacturer) $($board.Product)".Trim())
} catch {
  Add-Value 'board' ''
}

try {
  $bios = Get-CimInstance Win32_BIOS | Select-Object -First 1
  Add-Value 'bios' ("$($bios.Manufacturer) $($bios.SMBIOSBIOSVersion) $($bios.ReleaseDate)".Trim())
} catch {
  Add-Value 'bios' ''
}

try {
  $processor = Get-CimInstance Win32_Processor | Select-Object -First 1
  Add-Value 'cpu' ([string]$processor.Name)
  Add-Value 'cpu_manufacturer' ([string]$processor.Manufacturer)
  if ($processor.PSObject.Properties.Name -contains 'VirtualizationFirmwareEnabled') {
    Add-Value 'virtualization_firmware_enabled' ([bool]$processor.VirtualizationFirmwareEnabled)
  } else {
    Add-Value 'virtualization_firmware_enabled' $null
  }
} catch {
  Add-Value 'cpu' ''
  Add-Value 'cpu_manufacturer' ''
  Add-Value 'virtualization_firmware_enabled' $null
}

try {
  $computerSystem = Get-CimInstance Win32_ComputerSystem | Select-Object -First 1
  if ($computerSystem.PSObject.Properties.Name -contains 'HypervisorPresent') {
    Add-Value 'hypervisor_present' ([bool]$computerSystem.HypervisorPresent)
  } else {
    Add-Value 'hypervisor_present' $null
  }
  if ($computerSystem.PSObject.Properties.Name -contains 'TotalPhysicalMemory') {
    Add-Value 'total_physical_memory_mb' ([uint64][math]::Ceiling($computerSystem.TotalPhysicalMemory / 1MB))
  } else {
    Add-Value 'total_physical_memory_mb' $null
  }
  if ($computerSystem.PSObject.Properties.Name -contains 'AutomaticManagedPagefile') {
    Add-Value 'automatic_managed_pagefile' ([bool]$computerSystem.AutomaticManagedPagefile)
  } else {
    Add-Value 'automatic_managed_pagefile' $null
  }
} catch {
  Add-Value 'hypervisor_present' $null
  Add-Value 'total_physical_memory_mb' $null
  Add-Value 'automatic_managed_pagefile' $null
}

$featureStates = @()
$featureErrors = @()
foreach ($name in @('Microsoft-Hyper-V','Microsoft-Hyper-V-All','Microsoft-Hyper-V-Hypervisor')) {
  try {
    $feature = Get-WindowsOptionalFeature -Online -FeatureName $name -ErrorAction Stop
    $featureStates += "$($feature.FeatureName)=$($feature.State)"
  } catch {
    $featureErrors += "$name=$($_.Exception.Message)"
  }
}
Add-Value 'hyperv_state' ($featureStates -join '; ')
Add-Value 'hyperv_error' ($featureErrors -join '; ')

try {
  $feature = Get-WindowsOptionalFeature -Online -FeatureName VirtualMachinePlatform -ErrorAction Stop
  Add-Value 'virtual_machine_platform_state' ([string]$feature.State)
  Add-Value 'virtual_machine_platform_error' ''
} catch {
  Add-Value 'virtual_machine_platform_state' ''
  Add-Value 'virtual_machine_platform_error' $_.Exception.Message
}

try {
  $feature = Get-WindowsOptionalFeature -Online -FeatureName HypervisorPlatform -ErrorAction Stop
  Add-Value 'windows_hypervisor_platform_state' ([string]$feature.State)
  Add-Value 'windows_hypervisor_platform_error' ''
} catch {
  Add-Value 'windows_hypervisor_platform_state' ''
  Add-Value 'windows_hypervisor_platform_error' $_.Exception.Message
}

try {
  $firmwareDetails = @()
  $firmwareBootMode = $null

  try {
    $control = Get-ItemProperty -LiteralPath 'HKLM:\SYSTEM\CurrentControlSet\Control' -ErrorAction Stop
    if ($control.PSObject.Properties.Name -contains 'PEFirmwareType') {
      $peFirmwareType = [uint32]$control.PEFirmwareType
      $firmwareDetails += "PEFirmwareType=$peFirmwareType"
      if ($peFirmwareType -eq 2) {
        $firmwareBootMode = 'UEFI'
      } elseif ($peFirmwareType -eq 1) {
        $firmwareBootMode = 'Legacy'
      }
    } else {
      $firmwareDetails += 'PEFirmwareType registry value not present'
    }
  } catch {
    $firmwareDetails += "PEFirmwareType query failed: $($_.Exception.Message)"
  }

  if ($null -eq $firmwareBootMode) {
    try {
      $computerInfo = Get-ComputerInfo -Property BiosFirmwareType -ErrorAction Stop
      if ($null -ne $computerInfo.BiosFirmwareType) {
        $firmwareDetails += "BiosFirmwareType=$($computerInfo.BiosFirmwareType)"
        if ([string]$computerInfo.BiosFirmwareType) {
          $firmwareBootMode = [string]$computerInfo.BiosFirmwareType
        }
      }
    } catch {
      $firmwareDetails += "Get-ComputerInfo BiosFirmwareType failed: $($_.Exception.Message)"
    }
  } else {
    $firmwareDetails += 'BiosFirmwareType query skipped because PEFirmwareType resolved boot mode'
  }

  Add-Value 'firmware_boot_mode' $firmwareBootMode
  Add-Value 'firmware_boot_details' ($firmwareDetails -join '; ')
} catch {
  Add-Value 'firmware_boot_mode' $null
  Add-Value 'firmware_boot_details' $_.Exception.Message
}

try {
  if (Confirm-SecureBootUEFI -ErrorAction Stop) {
    Add-Value 'secure_boot_state' 'Enabled'
  } else {
    Add-Value 'secure_boot_state' 'Disabled'
  }
  Add-Value 'secure_boot_details' ''
} catch {
  $message = $_.Exception.Message
  if ($message -match 'not supported|unsupported') {
    Add-Value 'secure_boot_state' 'NotSupported'
  } else {
    Add-Value 'secure_boot_state' 'Unknown'
  }
  Add-Value 'secure_boot_details' $message
}

$tpmDetails = @()
$tpmPresent = $null
$tpmEnabled = $null
try {
  $tpm = Get-Tpm -ErrorAction Stop
  $tpmPresent = [bool]$tpm.TpmPresent
  if (-not $tpmPresent) { $tpmEnabled = $false }
  $tpmDetails += "Get-Tpm: TpmPresent=$($tpm.TpmPresent), TpmReady=$($tpm.TpmReady)"
} catch {
  $tpmDetails += "Get-Tpm failed: $($_.Exception.Message)"
}
try {
  $win32Tpm = @(Get-CimInstance -Namespace root\CIMV2\Security\MicrosoftTpm -ClassName Win32_Tpm -ErrorAction Stop)
  if ($win32Tpm.Count -gt 0) {
    $tpmPresent = $true
    $enabledValues = @($win32Tpm | ForEach-Object { $_.IsEnabled_InitialValue })
    $activatedValues = @($win32Tpm | ForEach-Object { $_.IsActivated_InitialValue })
    $tpmEnabled = ($enabledValues -contains $true)
    $tpmDetails += "Win32_Tpm present; IsEnabled_InitialValue=$($enabledValues -join ', '); IsActivated_InitialValue=$($activatedValues -join ', ')"
  }
} catch {
  $tpmDetails += "Win32_Tpm query failed: $($_.Exception.Message)"
}
Add-Value 'tpm_state' ($tpmDetails -join '; ')
Add-Value 'tpm_present' $tpmPresent
Add-Value 'tpm_enabled' $tpmEnabled
Add-Value 'tpm_details' ($tpmDetails -join '; ')

try {
  $deviceGuard = Get-CimInstance -Namespace root\Microsoft\Windows\DeviceGuard -ClassName Win32_DeviceGuard -ErrorAction Stop
  $availableProperties = @($deviceGuard.AvailableSecurityProperties)
  Add-Value 'vtd_available' ([bool]($availableProperties -contains 3))
  Add-Value 'vtd_details' ("Win32_DeviceGuard AvailableSecurityProperties=$($availableProperties -join ', '); value 3 means DMA protection/IOMMU is available")
} catch {
  Add-Value 'vtd_available' $null
  Add-Value 'vtd_details' "Win32_DeviceGuard query failed: $($_.Exception.Message)"
}

try {
  function Get-BcdEnum {
    foreach ($target in @('{current}', '{default}')) {
      $output = & bcdedit.exe /enum $target 2>&1
      if ($LASTEXITCODE -eq 0) {
        return [pscustomobject]@{ Target = $target; Output = $output }
      }
    }

    $output = & bcdedit.exe /enum 2>&1
    if ($LASTEXITCODE -eq 0) {
      return [pscustomobject]@{ Target = 'all'; Output = $output }
    }

    return [pscustomobject]@{ Target = ''; Output = $output }
  }

  $bcdEnum = Get-BcdEnum
  $bcdOutput = $bcdEnum.Output
  if (-not [string]::IsNullOrWhiteSpace($bcdEnum.Target)) {
    $launchType = $null
    foreach ($line in @($bcdOutput)) {
      if ($line -match '^\s*hypervisorlaunchtype\s+(\S+)\s*$') {
        $launchType = $Matches[1]
        break
      }
    }
    if ($null -eq $launchType) {
      $launchType = 'Auto'
      $bcdOutput = @($bcdOutput) + 'hypervisorlaunchtype is not listed; Windows default is Auto.'
    }
    Add-Value 'hypervisor_launch_type' $launchType
    Add-Value 'hypervisor_launch_details' ("Target=$($bcdEnum.Target); " + (($bcdOutput | Out-String).Trim()))
  } else {
    Add-Value 'hypervisor_launch_type' $null
    Add-Value 'hypervisor_launch_details' (($bcdOutput | Out-String).Trim())
  }
} catch {
  Add-Value 'hypervisor_launch_type' $null
  Add-Value 'hypervisor_launch_details' $_.Exception.Message
}

try {
  $sessionPowerPath = 'HKLM:\SYSTEM\CurrentControlSet\Control\Session Manager\Power'
  $sessionPower = Get-ItemProperty -LiteralPath $sessionPowerPath -ErrorAction Stop
  $sessionPowerNames = @($sessionPower.PSObject.Properties.Name)
  $hasHiberboot = [bool]($sessionPowerNames -contains 'HiberbootEnabled')
  $hiberbootValue = if ($hasHiberboot -and $null -ne $sessionPower.HiberbootEnabled) { [uint32]$sessionPower.HiberbootEnabled } else { $null }

  $hibernateEnabled = $null
  $hibernateDetails = @()
  try {
    $controlPower = Get-ItemProperty -LiteralPath 'HKLM:\SYSTEM\CurrentControlSet\Control\Power' -ErrorAction Stop
    $controlPowerNames = @($controlPower.PSObject.Properties.Name)
    if ($controlPowerNames -contains 'HibernateEnabled') {
      $hibernateEnabled = ([uint32]$controlPower.HibernateEnabled) -ne 0
      $hibernateDetails += "HibernateEnabled=$($controlPower.HibernateEnabled)"
    } elseif ($controlPowerNames -contains 'HibernateEnabledDefault') {
      $hibernateEnabled = ([uint32]$controlPower.HibernateEnabledDefault) -ne 0
      $hibernateDetails += "HibernateEnabledDefault=$($controlPower.HibernateEnabledDefault)"
    } else {
      $hibernateDetails += "HibernateEnabled value not present"
    }
    if ($controlPowerNames -contains 'HiberFileType') {
      $hibernateDetails += "HiberFileType=$($controlPower.HiberFileType)"
    }
    if ($controlPowerNames -contains 'HiberFileSizePercent') {
      $hibernateDetails += "HiberFileSizePercent=$($controlPower.HiberFileSizePercent)"
    }
  } catch {
    $hibernateDetails += "Control\Power registry query failed: $($_.Exception.Message)"
  }

  Add-Value 'hiberboot_enabled_present' $hasHiberboot
  Add-Value 'hiberboot_enabled' $(if ($hasHiberboot -and $null -ne $hiberbootValue) { [bool]($hiberbootValue -ne 0) } else { $null })
  Add-Value 'hibernation_enabled' $hibernateEnabled

  $fastStartupDetails = @()
  if ($hasHiberboot -and $null -ne $hiberbootValue) {
    $fastStartupDetails += "HiberbootEnabled=$hiberbootValue"
  } else {
    $fastStartupDetails += "HiberbootEnabled not present"
  }
  $fastStartupDetails += $hibernateDetails
  Add-Value 'fast_startup_details' ($fastStartupDetails -join '; ')
} catch {
  Add-Value 'hiberboot_enabled_present' $null
  Add-Value 'hiberboot_enabled' $null
  Add-Value 'hibernation_enabled' $null
  Add-Value 'fast_startup_details' "Fast startup registry query failed: $($_.Exception.Message)"
}

try {
  $mma = Get-MMAgent -ErrorAction Stop
  if ($mma.PSObject.Properties.Name -contains 'MemoryCompression') {
    Add-Value 'memory_compression_enabled' ([bool]$mma.MemoryCompression)
    Add-Value 'memory_compression_details' "MemoryCompression=$($mma.MemoryCompression)"
  } else {
    Add-Value 'memory_compression_enabled' $null
    Add-Value 'memory_compression_details' 'Get-MMAgent did not return MemoryCompression'
  }
} catch {
  Add-Value 'memory_compression_enabled' $null
  Add-Value 'memory_compression_details' "Get-MMAgent failed: $($_.Exception.Message)"
}

$pagefileDetails = @()
$systemDrive = [Environment]::GetEnvironmentVariable('SystemDrive')
if ([string]::IsNullOrWhiteSpace($systemDrive)) { $systemDrive = 'C:' }
Add-Value 'system_drive' $systemDrive
try {
  $disk = Get-CimInstance Win32_LogicalDisk -Filter "DeviceID='$systemDrive'" -ErrorAction Stop | Select-Object -First 1
  Add-Value 'system_drive_total_mb' ([uint64][math]::Floor($disk.Size / 1MB))
  Add-Value 'system_drive_free_mb' ([uint64][math]::Floor($disk.FreeSpace / 1MB))
} catch {
  Add-Value 'system_drive_total_mb' $null
  Add-Value 'system_drive_free_mb' $null
  $pagefileDetails += "Win32_LogicalDisk failed: $($_.Exception.Message)"
}

try {
  $settings = @(Get-CimInstance Win32_PageFileSetting -ErrorAction Stop | ForEach-Object {
    [ordered]@{
      name = [string]$_.Name
      initial_size_mb = if ($null -ne $_.InitialSize) { [uint64]$_.InitialSize } else { $null }
      maximum_size_mb = if ($null -ne $_.MaximumSize) { [uint64]$_.MaximumSize } else { $null }
    }
  })
  Add-Value 'pagefile_settings' $settings
} catch {
  Add-Value 'pagefile_settings' @()
  $pagefileDetails += "Win32_PageFileSetting failed: $($_.Exception.Message)"
}

try {
  $usage = @(Get-CimInstance Win32_PageFileUsage -ErrorAction Stop | ForEach-Object {
    [ordered]@{
      name = [string]$_.Name
      allocated_base_size_mb = if ($null -ne $_.AllocatedBaseSize) { [uint64]$_.AllocatedBaseSize } else { $null }
      current_usage_mb = if ($null -ne $_.CurrentUsage) { [uint64]$_.CurrentUsage } else { $null }
      peak_usage_mb = if ($null -ne $_.PeakUsage) { [uint64]$_.PeakUsage } else { $null }
      temp_page_file = if ($null -ne $_.TempPageFile) { [bool]$_.TempPageFile } else { $null }
    }
  })
  Add-Value 'pagefile_usage' $usage
} catch {
  Add-Value 'pagefile_usage' @()
  $pagefileDetails += "Win32_PageFileUsage failed: $($_.Exception.Message)"
}

try {
  $memoryKey = Get-ItemProperty -LiteralPath 'HKLM:\SYSTEM\CurrentControlSet\Control\Session Manager\Memory Management' -ErrorAction Stop
  Add-Value 'registry_paging_files' @($memoryKey.PagingFiles | Where-Object { $null -ne $_ } | ForEach-Object { [string]$_ })
  Add-Value 'registry_existing_pagefiles' @($memoryKey.ExistingPageFiles | Where-Object { $null -ne $_ } | ForEach-Object { [string]$_ })
  if ($null -ne $memoryKey.TempPageFile) {
    Add-Value 'registry_temp_pagefile' ([bool]$memoryKey.TempPageFile)
  } else {
    Add-Value 'registry_temp_pagefile' $null
  }
} catch {
  Add-Value 'registry_paging_files' @()
  Add-Value 'registry_existing_pagefiles' @()
  Add-Value 'registry_temp_pagefile' $null
  $pagefileDetails += "Memory Management registry failed: $($_.Exception.Message)"
}
Add-Value 'pagefile_details' ($pagefileDetails -join '; ')

try {
  $crashControl = Get-ItemProperty -LiteralPath 'HKLM:\SYSTEM\CurrentControlSet\Control\CrashControl' -ErrorAction Stop
  $crashNames = @($crashControl.PSObject.Properties.Name)
  $hasCrashDumpEnabled = [bool]($crashNames -contains 'CrashDumpEnabled')
  $hasMinidumpDir = [bool]($crashNames -contains 'MinidumpDir')
  Add-Value 'crash_dump_enabled_present' $hasCrashDumpEnabled
  Add-Value 'crash_dump_enabled' $(if ($hasCrashDumpEnabled -and $null -ne $crashControl.CrashDumpEnabled) { [uint32]$crashControl.CrashDumpEnabled } else { $null })
  Add-Value 'minidump_dir_present' $hasMinidumpDir
  Add-Value 'minidump_dir' $(if ($hasMinidumpDir -and $null -ne $crashControl.MinidumpDir) { [string]$crashControl.MinidumpDir } else { '' })
  Add-Value 'crash_control_details' "CrashDumpEnabled=$($crashControl.CrashDumpEnabled); MinidumpDir=$($crashControl.MinidumpDir)"
} catch {
  Add-Value 'crash_dump_enabled_present' $null
  Add-Value 'crash_dump_enabled' $null
  Add-Value 'minidump_dir_present' $null
  Add-Value 'minidump_dir' ''
  Add-Value 'crash_control_details' "CrashControl query failed: $($_.Exception.Message)"
}

$result | ConvertTo-Json -Compress -Depth 5
"#;

    run_snapshot_script(script)
}

fn collect_fast_snapshot() -> Result<CheckSnapshot, String> {
    let script = r#"
$ErrorActionPreference = 'SilentlyContinue'
$ProgressPreference = 'SilentlyContinue'
$result = [ordered]@{}

function Add-Value($name, $value) {
  $result[$name] = $value
}

Add-Value 'computer_name' $env:COMPUTERNAME

try {
  $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
  $principal = New-Object Security.Principal.WindowsPrincipal($identity)
  Add-Value 'is_administrator' ([bool]$principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator))
} catch {
  Add-Value 'is_administrator' $false
}

try {
  $computerSystem = Get-CimInstance Win32_ComputerSystem | Select-Object -First 1
  if ($computerSystem.PSObject.Properties.Name -contains 'HypervisorPresent') {
    Add-Value 'hypervisor_present' ([bool]$computerSystem.HypervisorPresent)
  } else {
    Add-Value 'hypervisor_present' $null
  }
  if ($computerSystem.PSObject.Properties.Name -contains 'TotalPhysicalMemory') {
    Add-Value 'total_physical_memory_mb' ([uint64][math]::Ceiling($computerSystem.TotalPhysicalMemory / 1MB))
  } else {
    Add-Value 'total_physical_memory_mb' $null
  }
  if ($computerSystem.PSObject.Properties.Name -contains 'AutomaticManagedPagefile') {
    Add-Value 'automatic_managed_pagefile' ([bool]$computerSystem.AutomaticManagedPagefile)
  } else {
    Add-Value 'automatic_managed_pagefile' $null
  }
} catch {
  Add-Value 'hypervisor_present' $null
  Add-Value 'total_physical_memory_mb' $null
  Add-Value 'automatic_managed_pagefile' $null
}

$featureStates = @()
$featureErrors = @()
foreach ($name in @('Microsoft-Hyper-V','Microsoft-Hyper-V-All','Microsoft-Hyper-V-Hypervisor')) {
  try {
    $feature = Get-WindowsOptionalFeature -Online -FeatureName $name -ErrorAction Stop
    $featureStates += "$($feature.FeatureName)=$($feature.State)"
  } catch {
    $featureErrors += "$name=$($_.Exception.Message)"
  }
}
Add-Value 'hyperv_state' ($featureStates -join '; ')
Add-Value 'hyperv_error' ($featureErrors -join '; ')

try {
  $feature = Get-WindowsOptionalFeature -Online -FeatureName VirtualMachinePlatform -ErrorAction Stop
  Add-Value 'virtual_machine_platform_state' ([string]$feature.State)
  Add-Value 'virtual_machine_platform_error' ''
} catch {
  Add-Value 'virtual_machine_platform_state' ''
  Add-Value 'virtual_machine_platform_error' $_.Exception.Message
}

try {
  $feature = Get-WindowsOptionalFeature -Online -FeatureName HypervisorPlatform -ErrorAction Stop
  Add-Value 'windows_hypervisor_platform_state' ([string]$feature.State)
  Add-Value 'windows_hypervisor_platform_error' ''
} catch {
  Add-Value 'windows_hypervisor_platform_state' ''
  Add-Value 'windows_hypervisor_platform_error' $_.Exception.Message
}

try {
  function Get-BcdEnum {
    foreach ($target in @('{current}', '{default}')) {
      $output = & bcdedit.exe /enum $target 2>&1
      if ($LASTEXITCODE -eq 0) {
        return [pscustomobject]@{ Target = $target; Output = $output }
      }
    }

    $output = & bcdedit.exe /enum 2>&1
    if ($LASTEXITCODE -eq 0) {
      return [pscustomobject]@{ Target = 'all'; Output = $output }
    }

    return [pscustomobject]@{ Target = ''; Output = $output }
  }

  $bcdEnum = Get-BcdEnum
  $bcdOutput = $bcdEnum.Output
  if (-not [string]::IsNullOrWhiteSpace($bcdEnum.Target)) {
    $launchType = $null
    foreach ($line in @($bcdOutput)) {
      if ($line -match '^\s*hypervisorlaunchtype\s+(\S+)\s*$') {
        $launchType = $Matches[1]
        break
      }
    }
    if ($null -eq $launchType) {
      $launchType = 'Auto'
      $bcdOutput = @($bcdOutput) + 'hypervisorlaunchtype is not listed; Windows default is Auto.'
    }
    Add-Value 'hypervisor_launch_type' $launchType
    Add-Value 'hypervisor_launch_details' ("Target=$($bcdEnum.Target); " + (($bcdOutput | Out-String).Trim()))
  } else {
    Add-Value 'hypervisor_launch_type' $null
    Add-Value 'hypervisor_launch_details' (($bcdOutput | Out-String).Trim())
  }
} catch {
  Add-Value 'hypervisor_launch_type' $null
  Add-Value 'hypervisor_launch_details' $_.Exception.Message
}

try {
  $sessionPowerPath = 'HKLM:\SYSTEM\CurrentControlSet\Control\Session Manager\Power'
  $sessionPower = Get-ItemProperty -LiteralPath $sessionPowerPath -ErrorAction Stop
  $sessionPowerNames = @($sessionPower.PSObject.Properties.Name)
  $hasHiberboot = [bool]($sessionPowerNames -contains 'HiberbootEnabled')
  $hiberbootValue = if ($hasHiberboot -and $null -ne $sessionPower.HiberbootEnabled) { [uint32]$sessionPower.HiberbootEnabled } else { $null }

  $hibernateEnabled = $null
  $hibernateDetails = @()
  try {
    $controlPower = Get-ItemProperty -LiteralPath 'HKLM:\SYSTEM\CurrentControlSet\Control\Power' -ErrorAction Stop
    $controlPowerNames = @($controlPower.PSObject.Properties.Name)
    if ($controlPowerNames -contains 'HibernateEnabled') {
      $hibernateEnabled = ([uint32]$controlPower.HibernateEnabled) -ne 0
      $hibernateDetails += "HibernateEnabled=$($controlPower.HibernateEnabled)"
    } elseif ($controlPowerNames -contains 'HibernateEnabledDefault') {
      $hibernateEnabled = ([uint32]$controlPower.HibernateEnabledDefault) -ne 0
      $hibernateDetails += "HibernateEnabledDefault=$($controlPower.HibernateEnabledDefault)"
    } else {
      $hibernateDetails += "HibernateEnabled value not present"
    }
    if ($controlPowerNames -contains 'HiberFileType') {
      $hibernateDetails += "HiberFileType=$($controlPower.HiberFileType)"
    }
    if ($controlPowerNames -contains 'HiberFileSizePercent') {
      $hibernateDetails += "HiberFileSizePercent=$($controlPower.HiberFileSizePercent)"
    }
  } catch {
    $hibernateDetails += "Control\Power registry query failed: $($_.Exception.Message)"
  }

  Add-Value 'hiberboot_enabled_present' $hasHiberboot
  Add-Value 'hiberboot_enabled' $(if ($hasHiberboot -and $null -ne $hiberbootValue) { [bool]($hiberbootValue -ne 0) } else { $null })
  Add-Value 'hibernation_enabled' $hibernateEnabled

  $fastStartupDetails = @()
  if ($hasHiberboot -and $null -ne $hiberbootValue) {
    $fastStartupDetails += "HiberbootEnabled=$hiberbootValue"
  } else {
    $fastStartupDetails += "HiberbootEnabled not present"
  }
  $fastStartupDetails += $hibernateDetails
  Add-Value 'fast_startup_details' ($fastStartupDetails -join '; ')
} catch {
  Add-Value 'hiberboot_enabled_present' $null
  Add-Value 'hiberboot_enabled' $null
  Add-Value 'hibernation_enabled' $null
  Add-Value 'fast_startup_details' "Fast startup registry query failed: $($_.Exception.Message)"
}

try {
  $mma = Get-MMAgent -ErrorAction Stop
  if ($mma.PSObject.Properties.Name -contains 'MemoryCompression') {
    Add-Value 'memory_compression_enabled' ([bool]$mma.MemoryCompression)
    Add-Value 'memory_compression_details' "MemoryCompression=$($mma.MemoryCompression)"
  } else {
    Add-Value 'memory_compression_enabled' $null
    Add-Value 'memory_compression_details' 'Get-MMAgent did not return MemoryCompression'
  }
} catch {
  Add-Value 'memory_compression_enabled' $null
  Add-Value 'memory_compression_details' "Get-MMAgent failed: $($_.Exception.Message)"
}

$pagefileDetails = @()
$systemDrive = [Environment]::GetEnvironmentVariable('SystemDrive')
if ([string]::IsNullOrWhiteSpace($systemDrive)) { $systemDrive = 'C:' }
Add-Value 'system_drive' $systemDrive
try {
  $disk = Get-CimInstance Win32_LogicalDisk -Filter "DeviceID='$systemDrive'" -ErrorAction Stop | Select-Object -First 1
  Add-Value 'system_drive_total_mb' ([uint64][math]::Floor($disk.Size / 1MB))
  Add-Value 'system_drive_free_mb' ([uint64][math]::Floor($disk.FreeSpace / 1MB))
} catch {
  Add-Value 'system_drive_total_mb' $null
  Add-Value 'system_drive_free_mb' $null
  $pagefileDetails += "Win32_LogicalDisk failed: $($_.Exception.Message)"
}

try {
  $settings = @(Get-CimInstance Win32_PageFileSetting -ErrorAction Stop | ForEach-Object {
    [ordered]@{
      name = [string]$_.Name
      initial_size_mb = if ($null -ne $_.InitialSize) { [uint64]$_.InitialSize } else { $null }
      maximum_size_mb = if ($null -ne $_.MaximumSize) { [uint64]$_.MaximumSize } else { $null }
    }
  })
  Add-Value 'pagefile_settings' $settings
} catch {
  Add-Value 'pagefile_settings' @()
  $pagefileDetails += "Win32_PageFileSetting failed: $($_.Exception.Message)"
}

try {
  $usage = @(Get-CimInstance Win32_PageFileUsage -ErrorAction Stop | ForEach-Object {
    [ordered]@{
      name = [string]$_.Name
      allocated_base_size_mb = if ($null -ne $_.AllocatedBaseSize) { [uint64]$_.AllocatedBaseSize } else { $null }
      current_usage_mb = if ($null -ne $_.CurrentUsage) { [uint64]$_.CurrentUsage } else { $null }
      peak_usage_mb = if ($null -ne $_.PeakUsage) { [uint64]$_.PeakUsage } else { $null }
      temp_page_file = if ($null -ne $_.TempPageFile) { [bool]$_.TempPageFile } else { $null }
    }
  })
  Add-Value 'pagefile_usage' $usage
} catch {
  Add-Value 'pagefile_usage' @()
  $pagefileDetails += "Win32_PageFileUsage failed: $($_.Exception.Message)"
}

try {
  $memoryKey = Get-ItemProperty -LiteralPath 'HKLM:\SYSTEM\CurrentControlSet\Control\Session Manager\Memory Management' -ErrorAction Stop
  Add-Value 'registry_paging_files' @($memoryKey.PagingFiles | Where-Object { $null -ne $_ } | ForEach-Object { [string]$_ })
} catch {
  Add-Value 'registry_paging_files' @()
  $pagefileDetails += "Memory Management registry failed: $($_.Exception.Message)"
}
Add-Value 'pagefile_details' ($pagefileDetails -join '; ')

try {
  $crashControl = Get-ItemProperty -LiteralPath 'HKLM:\SYSTEM\CurrentControlSet\Control\CrashControl' -ErrorAction Stop
  $crashNames = @($crashControl.PSObject.Properties.Name)
  $hasCrashDumpEnabled = [bool]($crashNames -contains 'CrashDumpEnabled')
  $hasMinidumpDir = [bool]($crashNames -contains 'MinidumpDir')
  Add-Value 'crash_dump_enabled_present' $hasCrashDumpEnabled
  Add-Value 'crash_dump_enabled' $(if ($hasCrashDumpEnabled -and $null -ne $crashControl.CrashDumpEnabled) { [uint32]$crashControl.CrashDumpEnabled } else { $null })
  Add-Value 'minidump_dir_present' $hasMinidumpDir
  Add-Value 'minidump_dir' $(if ($hasMinidumpDir -and $null -ne $crashControl.MinidumpDir) { [string]$crashControl.MinidumpDir } else { '' })
  Add-Value 'crash_control_details' "CrashDumpEnabled=$($crashControl.CrashDumpEnabled); MinidumpDir=$($crashControl.MinidumpDir)"
} catch {
  Add-Value 'crash_dump_enabled_present' $null
  Add-Value 'crash_dump_enabled' $null
  Add-Value 'minidump_dir_present' $null
  Add-Value 'minidump_dir' ''
  Add-Value 'crash_control_details' "CrashControl query failed: $($_.Exception.Message)"
}

$result | ConvertTo-Json -Compress -Depth 5
"#;

    run_snapshot_script(script)
}

fn run_snapshot_script(script: &str) -> Result<CheckSnapshot, String> {
    let output = run_program(
        "powershell.exe",
        &[
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            script,
        ],
    )?;

    if output.code != Some(0) {
        return Err(join_non_empty(&[output.stderr, output.stdout], "; "));
    }

    serde_json::from_str(output.stdout.trim()).map_err(|error| {
        format!(
            "Failed to parse check output: {error}. Raw output: {}",
            output.stdout
        )
    })
}

fn test_virtualization_snapshot(snapshot: &CheckSnapshot) -> CheckItem {
    let cpu = snapshot.cpu.as_deref().unwrap_or("-");
    let cpu_text = format!(
        "{} {}",
        snapshot.cpu_manufacturer.as_deref().unwrap_or(""),
        cpu
    );
    let fix_hint =
        "在 BIOS/UEFI 中启用 Intel Virtualization Technology / VT-x 或 AMD SVM / AMD-V，然后完全重启。";
    let required = if cpu_text.to_ascii_lowercase().contains("amd") {
        "AMD SVM / AMD-V must be enabled in firmware"
    } else if cpu_text.to_ascii_lowercase().contains("intel") {
        "Intel VT-x must be enabled in firmware"
    } else {
        "CPU virtualization must be enabled in firmware"
    };

    match snapshot.virtualization_firmware_enabled {
        Some(true) => check_item(
            "CPU virtualization",
            required,
            Some(true),
            format!("VirtualizationFirmwareEnabled=True; CPU={cpu}"),
            String::new(),
            fix_hint,
        ),
        Some(false) if snapshot.hypervisor_present.unwrap_or(false) => check_item(
            "CPU virtualization",
            required,
            Some(true),
            format!("HypervisorPresent=True; hardware virtualization is already in use. CPU={cpu}"),
            "Windows CPU virtualization fields can report False after a hypervisor starts."
                .to_string(),
            fix_hint,
        ),
        Some(false) => check_item(
            "CPU virtualization",
            required,
            Some(false),
            format!("VirtualizationFirmwareEnabled=False; CPU={cpu}"),
            String::new(),
            fix_hint,
        ),
        None if snapshot.hypervisor_present.unwrap_or(false) => check_item(
            "CPU virtualization",
            required,
            Some(true),
            format!("HypervisorPresent=True; CPU={cpu}"),
            "Hardware virtualization is already in use by a hypervisor.".to_string(),
            fix_hint,
        ),
        None => check_item(
            "CPU virtualization",
            required,
            None,
            format!("Could not determine CPU virtualization state; CPU={cpu}"),
            String::new(),
            fix_hint,
        ),
    }
}

fn test_vtd_snapshot(snapshot: &CheckSnapshot) -> CheckItem {
    let cpu = snapshot.cpu.clone().unwrap_or_else(|| "-".to_string());
    let required = if snapshot
        .cpu_manufacturer
        .as_deref()
        .unwrap_or_default()
        .to_lowercase()
        .contains("intel")
    {
        "Intel VT-d / IOMMU must be enabled in firmware"
    } else if snapshot
        .cpu_manufacturer
        .as_deref()
        .unwrap_or_default()
        .to_lowercase()
        .contains("amd")
    {
        "AMD IOMMU must be enabled in firmware"
    } else {
        "IOMMU / DMA remapping must be enabled in firmware"
    };
    let details = snapshot.vtd_details.clone().unwrap_or_default();
    let fix_hint = "在 BIOS/UEFI 中开启 VT-d、IOMMU 或 DMA Remapping。不同主板可能位于 Advanced、System Agent 或 Chipset 菜单。";

    match snapshot.vtd_available {
        Some(true) => check_item(
            "VT-d / IOMMU",
            required,
            Some(true),
            format!("DMAProtectionAvailable=True; CPU={cpu}"),
            details,
            fix_hint,
        ),
        Some(false) => check_item(
            "VT-d / IOMMU",
            required,
            Some(false),
            format!("DMAProtectionAvailable=False; CPU={cpu}"),
            details,
            fix_hint,
        ),
        None => check_item(
            "VT-d / IOMMU",
            required,
            None,
            format!("Could not determine VT-d/IOMMU state; CPU={cpu}"),
            details,
            fix_hint,
        ),
    }
}

fn test_csm_snapshot(snapshot: &CheckSnapshot) -> CheckItem {
    let mode = snapshot
        .firmware_boot_mode
        .as_deref()
        .unwrap_or("Unknown")
        .trim();
    let details = join_non_empty(
        &[
            snapshot.firmware_boot_details.clone().unwrap_or_default(),
            "Windows can detect the current boot mode, not every firmware menu's CSM toggle state."
                .to_string(),
        ],
        "; ",
    );
    let fix_hint = "Change BIOS/UEFI boot mode from Legacy/CSM to UEFI. If the system disk is MBR, convert it to GPT first with Microsoft's MBR2GPT tool or Windows may not boot.";

    if mode.eq_ignore_ascii_case("UEFI") {
        check_item(
            "CSM / Legacy Boot",
            "Current boot mode must be UEFI, not Legacy/CSM",
            Some(true),
            format!("BootMode={mode}"),
            details,
            fix_hint,
        )
    } else if mode.eq_ignore_ascii_case("Legacy") || mode.eq_ignore_ascii_case("BIOS") {
        check_item(
            "CSM / Legacy Boot",
            "Current boot mode must be UEFI, not Legacy/CSM",
            Some(false),
            format!("BootMode={mode}"),
            details,
            fix_hint,
        )
    } else {
        check_item(
            "CSM / Legacy Boot",
            "Current boot mode must be UEFI, not Legacy/CSM",
            None,
            "Could not determine current firmware boot mode".to_string(),
            details,
            fix_hint,
        )
    }
}

fn test_hyper_v_snapshot(snapshot: &CheckSnapshot) -> CheckItem {
    let text = snapshot.hyperv_state.clone().unwrap_or_default();
    let details = snapshot.hyperv_error.clone().unwrap_or_default();
    let fix_hint = "以管理员身份运行程序，点击 Microsoft Hyper-V 的“开启”，然后重启。";

    if text.trim().is_empty() {
        return check_item(
            "Microsoft Hyper-V",
            "Hyper-V feature/role must be enabled",
            None,
            "Could not read Hyper-V feature state".to_string(),
            details,
            fix_hint,
        );
    }

    let enabled = text.contains("Microsoft-Hyper-V=Enabled")
        || text.contains("Microsoft-Hyper-V-All=Enabled")
        || text.contains("Microsoft-Hyper-V-Hypervisor=Enabled");
    check_item(
        "Microsoft Hyper-V",
        "Hyper-V feature/role must be enabled",
        Some(enabled),
        text,
        details,
        fix_hint,
    )
}

fn test_secure_boot_snapshot(snapshot: &CheckSnapshot) -> CheckItem {
    let state = snapshot.secure_boot_state.as_deref().unwrap_or("Unknown");
    let fix_hint = "在 BIOS/UEFI 中关闭 Secure Boot。若启用了 BitLocker，先保存恢复密钥。";

    match state {
        "Disabled" | "NotSupported" => check_item(
            "Secure Boot",
            "Secure Boot must be disabled",
            Some(true),
            format!("SecureBoot={state}"),
            snapshot.secure_boot_details.clone().unwrap_or_default(),
            fix_hint,
        ),
        "Enabled" => check_item(
            "Secure Boot",
            "Secure Boot must be disabled",
            Some(false),
            "SecureBoot=Enabled".to_string(),
            snapshot.secure_boot_details.clone().unwrap_or_default(),
            fix_hint,
        ),
        _ => check_item(
            "Secure Boot",
            "Secure Boot must be disabled",
            None,
            "Could not determine Secure Boot state".to_string(),
            snapshot.secure_boot_details.clone().unwrap_or_default(),
            fix_hint,
        ),
    }
}

fn test_tpm_snapshot(snapshot: &CheckSnapshot) -> CheckItem {
    let detected = snapshot
        .tpm_state
        .clone()
        .unwrap_or_else(|| "Could not determine TPM state".to_string());
    let fix_hint =
        "在 BIOS/UEFI 中关闭 TPM、Intel PTT、AMD fTPM 或 Security Device。若启用了 BitLocker，先保存恢复密钥。";
    let passed = match (snapshot.tpm_enabled, snapshot.tpm_present) {
        (Some(enabled), _) => Some(!enabled),
        (None, Some(present)) => Some(!present),
        (None, None) => None,
    };

    check_item(
        "TPM",
        "TPM must be disabled or not visible to Windows",
        passed,
        detected,
        snapshot.tpm_details.clone().unwrap_or_default(),
        fix_hint,
    )
}

fn test_hypervisor_launch_snapshot(snapshot: &CheckSnapshot) -> CheckItem {
    let fix_hint = "以管理员身份运行程序，点击 Hypervisor 开机启动的“开启”，然后重启。";
    match snapshot.hypervisor_launch_type.as_deref() {
        Some(value) => check_item(
            "Hypervisor launch",
            "hypervisorlaunchtype must be Auto",
            Some(value.eq_ignore_ascii_case("auto")),
            format!("hypervisorlaunchtype={value}"),
            String::new(),
            fix_hint,
        ),
        None => check_item(
            "Hypervisor launch",
            "hypervisorlaunchtype must be Auto",
            None,
            "hypervisorlaunchtype is not listed in the current BCD entry".to_string(),
            snapshot
                .hypervisor_launch_details
                .clone()
                .unwrap_or_default(),
            fix_hint,
        ),
    }
}

fn configured_pagefiles_from_snapshot(snapshot: &CheckSnapshot) -> Vec<PageFileConfigInfo> {
    let mut configs = BTreeMap::<String, PageFileConfigInfo>::new();

    for setting in snapshot.pagefile_settings.as_deref().unwrap_or(&[]) {
        let Some(name) = setting
            .name
            .as_ref()
            .map(|name| name.trim())
            .filter(|name| !name.is_empty())
        else {
            continue;
        };

        configs.insert(
            name.to_string(),
            PageFileConfigInfo {
                name: name.to_string(),
                initial_size_mb: setting.initial_size_mb,
                maximum_size_mb: setting.maximum_size_mb,
                source: "WMI".to_string(),
            },
        );
    }

    for registry_config in
        registry_pagefile_configs(snapshot.registry_paging_files.as_deref().unwrap_or(&[]))
    {
        configs
            .entry(registry_config.name.clone())
            .and_modify(|existing| {
                if existing.initial_size_mb.is_none() {
                    existing.initial_size_mb = registry_config.initial_size_mb;
                }
                if existing.maximum_size_mb.is_none() {
                    existing.maximum_size_mb = registry_config.maximum_size_mb;
                }
                existing.source = "WMI + 注册表".to_string();
            })
            .or_insert(registry_config);
    }

    configs.into_values().collect()
}

fn registry_pagefile_configs(entries: &[String]) -> Vec<PageFileConfigInfo> {
    entries
        .iter()
        .filter_map(|entry| parse_registry_pagefile_entry(entry))
        .collect()
}

fn parse_registry_pagefile_entry(entry: &str) -> Option<PageFileConfigInfo> {
    let trimmed = entry.trim();
    if trimmed.is_empty() {
        return None;
    }

    let parts: Vec<&str> = trimmed.split_whitespace().collect();
    let name = parts.first()?.trim();
    if name.is_empty() {
        return None;
    }

    Some(PageFileConfigInfo {
        name: name.to_string(),
        initial_size_mb: parts.get(1).and_then(|value| value.parse::<u64>().ok()),
        maximum_size_mb: parts.get(2).and_then(|value| value.parse::<u64>().ok()),
        source: "注册表".to_string(),
    })
}

fn pagefile_configured_state(
    snapshot: &CheckSnapshot,
    configured_pagefiles: &[PageFileConfigInfo],
) -> String {
    if snapshot.automatic_managed_pagefile == Some(true) {
        return "系统托管".to_string();
    }

    if configured_pagefiles.is_empty() {
        return match snapshot.automatic_managed_pagefile {
            Some(false) => "未配置页面文件".to_string(),
            Some(true) => "系统托管".to_string(),
            None => "未知".to_string(),
        };
    }

    let all_system_managed = configured_pagefiles.iter().all(|pagefile| {
        matches!(pagefile.initial_size_mb, None | Some(0))
            && matches!(pagefile.maximum_size_mb, None | Some(0))
    });

    if all_system_managed {
        "系统托管".to_string()
    } else {
        "自定义".to_string()
    }
}

fn virtual_memory_from_snapshot(snapshot: &CheckSnapshot) -> VirtualMemoryInfo {
    let total_physical_memory_mb = snapshot.total_physical_memory_mb.unwrap_or(0);
    let system_drive = snapshot
        .system_drive
        .clone()
        .unwrap_or_else(|| "C:".to_string());
    let configured_pagefiles = configured_pagefiles_from_snapshot(snapshot);
    let configured_state = pagefile_configured_state(snapshot, &configured_pagefiles);

    let mut pagefile_map = BTreeMap::<String, PageFileInfo>::new();

    for setting in snapshot.pagefile_settings.as_deref().unwrap_or(&[]) {
        if let Some(name) = setting.name.as_ref().filter(|name| !name.trim().is_empty()) {
            let entry = pagefile_map
                .entry(name.clone())
                .or_insert_with(|| PageFileInfo {
                    name: name.clone(),
                    ..PageFileInfo::default()
                });
            entry.initial_size_mb = setting.initial_size_mb;
            entry.maximum_size_mb = setting.maximum_size_mb;
        }
    }

    for usage in snapshot.pagefile_usage.as_deref().unwrap_or(&[]) {
        if let Some(name) = usage.name.as_ref().filter(|name| !name.trim().is_empty()) {
            let entry = pagefile_map
                .entry(name.clone())
                .or_insert_with(|| PageFileInfo {
                    name: name.clone(),
                    ..PageFileInfo::default()
                });
            entry.allocated_base_size_mb = usage.allocated_base_size_mb;
            entry.current_usage_mb = usage.current_usage_mb;
            entry.peak_usage_mb = usage.peak_usage_mb;
            entry.temp_page_file = usage.temp_page_file;
        }
    }

    VirtualMemoryInfo {
        total_physical_memory_mb,
        automatic_managed_pagefile: snapshot.automatic_managed_pagefile,
        configured_state,
        system_drive,
        system_drive_total_mb: snapshot.system_drive_total_mb,
        system_drive_free_mb: snapshot.system_drive_free_mb,
        configured_pagefiles,
        pagefiles: pagefile_map.into_values().collect(),
        recommendation: pagefile_recommendation(
            total_physical_memory_mb,
            snapshot.system_drive_total_mb,
        ),
        details: snapshot.pagefile_details.clone().unwrap_or_default(),
    }
}

fn fallback_virtual_memory_info(details: String) -> VirtualMemoryInfo {
    VirtualMemoryInfo {
        total_physical_memory_mb: 0,
        automatic_managed_pagefile: None,
        configured_state: "未知".to_string(),
        system_drive: "C:".to_string(),
        system_drive_total_mb: None,
        system_drive_free_mb: None,
        configured_pagefiles: Vec::new(),
        pagefiles: Vec::new(),
        recommendation: pagefile_recommendation(0, None),
        details,
    }
}

fn blue_screen_from_snapshot(
    snapshot: &CheckSnapshot,
    resource_dir: Option<&Path>,
) -> BlueScreenInfo {
    let minidump_dir_raw = snapshot
        .minidump_dir
        .clone()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "%SystemRoot%\\Minidump".to_string());
    let minidump_path = expand_windows_path(&minidump_dir_raw);
    let minidump_dir_exists = minidump_path.is_dir();
    let recent_dumps = recent_dump_files(&minidump_path);
    let tool_path = find_bluescreenview_tool(resource_dir);
    let minidump_dir_configured = minidump_dir_raw.eq_ignore_ascii_case("%SystemRoot%\\Minidump")
        || minidump_path == default_minidump_path();
    let collection_ready =
        snapshot.crash_dump_enabled == Some(3) && minidump_dir_configured && minidump_dir_exists;

    BlueScreenInfo {
        crash_dump_enabled: snapshot.crash_dump_enabled,
        crash_dump_label: crash_dump_label(snapshot.crash_dump_enabled),
        minidump_dir: minidump_path.to_string_lossy().to_string(),
        minidump_dir_configured,
        minidump_dir_exists,
        dump_count: recent_dumps.len(),
        recent_dumps,
        tool_path: tool_path
            .as_ref()
            .map(|path| path.to_string_lossy().to_string()),
        tool_available: tool_path.is_some(),
        collection_ready,
        details: snapshot.crash_control_details.clone().unwrap_or_default(),
    }
}

fn fallback_blue_screen_info(resource_dir: Option<&Path>, details: String) -> BlueScreenInfo {
    let minidump_path = default_minidump_path();
    let recent_dumps = recent_dump_files(&minidump_path);
    let tool_path = find_bluescreenview_tool(resource_dir);

    BlueScreenInfo {
        crash_dump_enabled: None,
        crash_dump_label: "未知".to_string(),
        minidump_dir: minidump_path.to_string_lossy().to_string(),
        minidump_dir_configured: false,
        minidump_dir_exists: minidump_path.is_dir(),
        dump_count: recent_dumps.len(),
        recent_dumps,
        tool_path: tool_path
            .as_ref()
            .map(|path| path.to_string_lossy().to_string()),
        tool_available: tool_path.is_some(),
        collection_ready: false,
        details,
    }
}

fn crash_dump_label(value: Option<u32>) -> String {
    match value {
        Some(0) => "关闭".to_string(),
        Some(1) => "完整内存转储".to_string(),
        Some(2) => "内核内存转储".to_string(),
        Some(3) => "小内存转储".to_string(),
        Some(7) => "自动内存转储".to_string(),
        Some(other) => format!("模式 {other}"),
        None => "未知".to_string(),
    }
}

fn default_minidump_path() -> PathBuf {
    let system_root = std::env::var("SystemRoot")
        .or_else(|_| std::env::var("windir"))
        .unwrap_or_else(|_| "C:\\Windows".to_string());
    PathBuf::from(system_root).join("Minidump")
}

fn expand_windows_path(value: &str) -> PathBuf {
    let mut expanded = value.to_string();
    for (key, fallback) in [
        ("SystemRoot", "C:\\Windows"),
        ("windir", "C:\\Windows"),
        ("USERPROFILE", ""),
    ] {
        let replacement = std::env::var(key).unwrap_or_else(|_| fallback.to_string());
        expanded = expanded.replace(&format!("%{key}%"), &replacement);
        expanded = expanded.replace(&format!("%{}%", key.to_ascii_uppercase()), &replacement);
    }
    PathBuf::from(expanded)
}

fn recent_dump_files(dir: &Path) -> Vec<DumpFileInfo> {
    let mut files = match fs::read_dir(dir) {
        Ok(entries) => entries
            .filter_map(Result::ok)
            .filter_map(|entry| {
                let path = entry.path();
                if path
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| !ext.eq_ignore_ascii_case("dmp"))
                    .unwrap_or(true)
                {
                    return None;
                }

                let metadata = entry.metadata().ok()?;
                let modified_system = metadata.modified().ok();
                let modified = modified_system
                    .map(|time| chrono::DateTime::<chrono::Local>::from(time).to_rfc3339())
                    .unwrap_or_default();
                let modified_sort = modified_system
                    .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|duration| duration.as_secs())
                    .unwrap_or(0);

                Some((
                    modified_sort,
                    DumpFileInfo {
                        name: path
                            .file_name()
                            .map(|name| name.to_string_lossy().to_string())
                            .unwrap_or_else(|| "-".to_string()),
                        path: path.to_string_lossy().to_string(),
                        size_kb: metadata.len().saturating_add(1023) / 1024,
                        modified,
                    },
                ))
            })
            .collect::<Vec<_>>(),
        Err(_) => Vec::new(),
    };

    files.sort_by_key(|file| Reverse(file.0));
    files.into_iter().take(8).map(|(_, file)| file).collect()
}

fn find_bluescreenview_tool(resource_dir: Option<&Path>) -> Option<PathBuf> {
    let mut candidates = Vec::<PathBuf>::new();

    if let Some(resource_dir) = resource_dir {
        candidates.push(resource_dir.join("resources/tools/BlueScreenView.exe"));
        candidates.push(resource_dir.join("resources/tools/bluescreenview155.exe"));
        candidates.push(resource_dir.join("tools/BlueScreenView.exe"));
        candidates.push(resource_dir.join("tools/bluescreenview155.exe"));
        candidates.push(resource_dir.join("BlueScreenView.exe"));
        candidates.push(resource_dir.join("bluescreenview155.exe"));
    }

    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            candidates.push(exe_dir.join("resources/tools/BlueScreenView.exe"));
            candidates.push(exe_dir.join("resources/tools/bluescreenview155.exe"));
            candidates.push(exe_dir.join("tools/BlueScreenView.exe"));
            candidates.push(exe_dir.join("tools/bluescreenview155.exe"));
            candidates.push(exe_dir.join("BlueScreenView.exe"));
            candidates.push(exe_dir.join("bluescreenview155.exe"));
        }
    }

    candidates
        .push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/tools/BlueScreenView.exe"));
    candidates.push(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/tools/bluescreenview155.exe"),
    );

    if let Some(profile) = std::env::var_os("USERPROFILE") {
        candidates.push(
            PathBuf::from(&profile)
                .join("Desktop")
                .join("BlueScreenView.exe"),
        );
        candidates.push(
            PathBuf::from(profile)
                .join("Desktop")
                .join("bluescreenview155.exe"),
        );
    }

    candidates.into_iter().find(|path| path.is_file())
}

fn pagefile_recommendation(
    total_physical_memory_mb: u64,
    _system_drive_total_mb: Option<u64>,
) -> PageFileRecommendation {
    if total_physical_memory_mb == 0 {
        return PageFileRecommendation {
            preferred_mode: "系统托管".to_string(),
            recommended_initial_mb: None,
            recommended_maximum_mb: None,
            system_managed_min_estimate_mb: 0,
            system_managed_max_estimate_mb: 0,
            formula: "需要先读取物理内存容量，才能计算自定义页面文件值。".to_string(),
            reason: "未能读取物理内存，保守建议保持系统托管。".to_string(),
        };
    }

    let system_min_estimate = std::cmp::min(div_ceil(total_physical_memory_mb, 8), 32 * 1024);
    let system_max_estimate = std::cmp::max(total_physical_memory_mb.saturating_mul(3), 4096);

    if total_physical_memory_mb <= 32 * 1024 {
        let dump_safe_size = total_physical_memory_mb.saturating_add(300);
        PageFileRecommendation {
            preferred_mode: "固定自定义".to_string(),
            recommended_initial_mb: Some(dump_safe_size),
            recommended_maximum_mb: Some(dump_safe_size),
            system_managed_min_estimate_mb: system_min_estimate,
            system_managed_max_estimate_mb: system_max_estimate,
            formula: "固定值 = 物理内存 + 300 MB；系统托管上限估算 = min(max(3 x 物理内存, 4096 MB), 系统盘容量 / 8)。".to_string(),
            reason: "物理内存不超过 32 GB 时，这个固定值便于支持完整内存转储，并保留 300 MB 安全余量。".to_string(),
        }
    } else {
        PageFileRecommendation {
            preferred_mode: "系统托管".to_string(),
            recommended_initial_mb: None,
            recommended_maximum_mb: None,
            system_managed_min_estimate_mb: system_min_estimate,
            system_managed_max_estimate_mb: system_max_estimate,
            formula: "推荐值 = 系统托管；官方上限规则是 max(3 x 物理内存, 4096 MB)，这是按需增长边界，不是一次性预留空间。".to_string(),
            reason: "物理内存超过 32 GB 时，不需要手动设置几十 GB 的固定页面文件，推荐由 Windows 自动按需管理。".to_string(),
        }
    }
}

fn div_ceil(value: u64, divisor: u64) -> u64 {
    if divisor == 0 {
        0
    } else {
        value.div_ceil(divisor)
    }
}

fn feature_states_from_snapshot(snapshot: &CheckSnapshot) -> FeatureStates {
    FeatureStates {
        hyper_v: summarize_hyper_v_state(snapshot),
        virtual_machine_platform: state_or_error(
            snapshot.virtual_machine_platform_state.as_deref(),
            snapshot.virtual_machine_platform_error.as_deref(),
        ),
        windows_hypervisor_platform: state_or_error(
            snapshot.windows_hypervisor_platform_state.as_deref(),
            snapshot.windows_hypervisor_platform_error.as_deref(),
        ),
        hypervisor_launch: snapshot
            .hypervisor_launch_type
            .clone()
            .unwrap_or_else(|| "Unknown".to_string()),
        fast_startup: summarize_fast_startup_state(snapshot),
        memory_compression: summarize_memory_compression_state(snapshot),
    }
}

fn state_or_error(state: Option<&str>, error: Option<&str>) -> String {
    if let Some(state) = state {
        if !state.trim().is_empty() {
            return state.to_string();
        }
    }

    if let Some(error) = error {
        if !error.trim().is_empty() {
            return format!("Unknown ({error})");
        }
    }

    "Unknown".to_string()
}

fn summarize_hyper_v_state(snapshot: &CheckSnapshot) -> String {
    let state = snapshot.hyperv_state.clone().unwrap_or_default();
    if state.contains("=Enabled") {
        "Enabled".to_string()
    } else if state.contains("=Disabled") {
        "Disabled".to_string()
    } else if state.trim().is_empty() {
        "Unknown".to_string()
    } else {
        state
    }
}

fn summarize_fast_startup_state(snapshot: &CheckSnapshot) -> String {
    match (snapshot.hiberboot_enabled, snapshot.hibernation_enabled) {
        (Some(true), Some(false)) => "Disabled (hibernation unavailable)".to_string(),
        (Some(true), _) => "Enabled".to_string(),
        (Some(false), _) => "Disabled".to_string(),
        (None, Some(false)) => "Disabled (hibernation unavailable)".to_string(),
        (None, _) => state_or_error(None, snapshot.fast_startup_details.as_deref()),
    }
}

fn summarize_memory_compression_state(snapshot: &CheckSnapshot) -> String {
    match snapshot.memory_compression_enabled {
        Some(true) => "Enabled".to_string(),
        Some(false) => "Disabled".to_string(),
        None => state_or_error(None, snapshot.memory_compression_details.as_deref()),
    }
}

#[allow(dead_code)]
fn hardware_info() -> HardwareInfo {
    let board = ps_trim("$b=Get-CimInstance Win32_BaseBoard; \"$($b.Manufacturer) $($b.Product)\"")
        .unwrap_or_else(|_| "-".to_string());

    let bios = ps_trim(
        "$b=Get-CimInstance Win32_BIOS; \"$($b.Manufacturer) $($b.SMBIOSBIOSVersion) $($b.ReleaseDate)\"",
    )
    .unwrap_or_else(|_| "-".to_string());

    let cpu = ps_trim("(Get-CimInstance Win32_Processor | Select-Object -First 1).Name")
        .unwrap_or_else(|_| "-".to_string());

    HardwareInfo { board, bios, cpu }
}

#[allow(dead_code)]
fn test_virtualization(cpu: &str) -> CheckItem {
    let fix_hint = "在 BIOS/UEFI 中启用 Intel Virtualization Technology / VT-x 或 AMD SVM / AMD-V，然后完全重启。";
    let required = if cpu.to_ascii_lowercase().contains("amd") {
        "AMD SVM / AMD-V must be enabled in firmware"
    } else if cpu.to_ascii_lowercase().contains("intel") {
        "Intel VT-x must be enabled in firmware"
    } else {
        "CPU virtualization must be enabled in firmware"
    };

    let virt = ps_bool(
        "(Get-CimInstance Win32_Processor | Select-Object -First 1).VirtualizationFirmwareEnabled",
    );
    let hypervisor_present =
        ps_bool("(Get-CimInstance Win32_ComputerSystem).HypervisorPresent").unwrap_or(false);

    match virt {
        Ok(true) => check_item(
            "CPU virtualization",
            required,
            Some(true),
            format!("VirtualizationFirmwareEnabled=True; CPU={cpu}"),
            String::new(),
            fix_hint,
        ),
        Ok(false) if hypervisor_present => check_item(
            "CPU virtualization",
            required,
            Some(true),
            format!("HypervisorPresent=True; hardware virtualization is already in use. CPU={cpu}"),
            "Windows CPU virtualization fields can report False after a hypervisor starts."
                .to_string(),
            fix_hint,
        ),
        Ok(false) => check_item(
            "CPU virtualization",
            required,
            Some(false),
            format!("VirtualizationFirmwareEnabled=False; CPU={cpu}"),
            String::new(),
            fix_hint,
        ),
        Err(error) if hypervisor_present => check_item(
            "CPU virtualization",
            required,
            Some(true),
            format!("HypervisorPresent=True; CPU={cpu}"),
            error,
            fix_hint,
        ),
        Err(error) => check_item(
            "CPU virtualization",
            required,
            None,
            format!("Could not determine CPU virtualization state; CPU={cpu}"),
            error,
            fix_hint,
        ),
    }
}

#[allow(dead_code)]
fn test_hyper_v() -> CheckItem {
    let script = r#"
$names='Microsoft-Hyper-V','Microsoft-Hyper-V-All','Microsoft-Hyper-V-Hypervisor'
$items = foreach ($name in $names) {
  try {
    $f = Get-WindowsOptionalFeature -Online -FeatureName $name -ErrorAction Stop
    "$($f.FeatureName)=$($f.State)"
  } catch {
    "$name=ERROR:$($_.Exception.Message)"
  }
}
$items -join '; '
"#;
    let fix_hint = "以管理员身份运行：Enable-WindowsOptionalFeature -Online -FeatureName Microsoft-Hyper-V -All，然后重启。";

    match ps_trim(script) {
        Ok(text) => {
            let enabled = text.contains("Microsoft-Hyper-V=Enabled")
                || text.contains("Microsoft-Hyper-V-All=Enabled")
                || text.contains("Microsoft-Hyper-V-Hypervisor=Enabled");
            check_item(
                "Microsoft Hyper-V",
                "Hyper-V feature/role must be enabled",
                Some(enabled),
                text,
                String::new(),
                fix_hint,
            )
        }
        Err(error) => check_item(
            "Microsoft Hyper-V",
            "Hyper-V feature/role must be enabled",
            None,
            "Could not read Hyper-V feature state".to_string(),
            error,
            fix_hint,
        ),
    }
}

#[allow(dead_code)]
fn test_secure_boot() -> CheckItem {
    let script = r#"
try {
  if (Confirm-SecureBootUEFI -ErrorAction Stop) { 'Enabled' } else { 'Disabled' }
} catch {
  if ($_.Exception.Message -match 'not supported|unsupported') { 'NotSupported' } else { "ERROR:$($_.Exception.Message)" }
}
"#;
    let fix_hint = "在 BIOS/UEFI 中关闭 Secure Boot。若启用了 BitLocker，先保存恢复密钥。";

    match ps_trim(script) {
        Ok(value) if value == "Disabled" || value == "NotSupported" => check_item(
            "Secure Boot",
            "Secure Boot must be disabled",
            Some(true),
            format!("SecureBoot={value}"),
            String::new(),
            fix_hint,
        ),
        Ok(value) if value == "Enabled" => check_item(
            "Secure Boot",
            "Secure Boot must be disabled",
            Some(false),
            "SecureBoot=Enabled".to_string(),
            String::new(),
            fix_hint,
        ),
        Ok(value) => check_item(
            "Secure Boot",
            "Secure Boot must be disabled",
            None,
            "Could not determine Secure Boot state".to_string(),
            value,
            fix_hint,
        ),
        Err(error) => check_item(
            "Secure Boot",
            "Secure Boot must be disabled",
            None,
            "Could not determine Secure Boot state".to_string(),
            error,
            fix_hint,
        ),
    }
}

#[allow(dead_code)]
fn test_tpm() -> CheckItem {
    let script = r#"
try {
  $t = Get-Tpm -ErrorAction Stop
  "TpmPresent=$($t.TpmPresent); TpmReady=$($t.TpmReady)"
} catch {
  try {
    $w = Get-CimInstance -Namespace root\CIMV2\Security\MicrosoftTpm -ClassName Win32_Tpm -ErrorAction Stop
    if ($null -eq $w) { 'TpmPresent=False' } else { "Win32_Tpm present; IsEnabled_InitialValue=$($w.IsEnabled_InitialValue)" }
  } catch {
    "ERROR:$($_.Exception.Message)"
  }
}
"#;
    let fix_hint = "在 BIOS/UEFI 中关闭 TPM、Intel PTT、AMD fTPM 或 Security Device。若启用了 BitLocker，先保存恢复密钥。";

    match ps_trim(script) {
        Ok(text)
            if text.contains("TpmPresent=False")
                || text.contains("IsEnabled_InitialValue=False") =>
        {
            check_item(
                "TPM",
                "TPM must be disabled or not visible to Windows",
                Some(true),
                text,
                String::new(),
                fix_hint,
            )
        }
        Ok(text)
            if text.contains("TpmPresent=True") || text.contains("IsEnabled_InitialValue=True") =>
        {
            check_item(
                "TPM",
                "TPM must be disabled or not visible to Windows",
                Some(false),
                text,
                String::new(),
                fix_hint,
            )
        }
        Ok(text) => check_item(
            "TPM",
            "TPM must be disabled or not visible to Windows",
            None,
            "Could not determine TPM state".to_string(),
            text,
            fix_hint,
        ),
        Err(error) => check_item(
            "TPM",
            "TPM must be disabled or not visible to Windows",
            None,
            "Could not determine TPM state".to_string(),
            error,
            fix_hint,
        ),
    }
}

#[allow(dead_code)]
fn test_hypervisor_launch() -> CheckItem {
    let fix_hint = "以管理员身份运行：bcdedit /set {default} hypervisorlaunchtype auto，然后重启。";
    match run_program("bcdedit.exe", &["/enum", "{current}"]) {
        Ok(output) if output.code == Some(0) => {
            let value = output.stdout.lines().find_map(|line| {
                let line = line.trim();
                if line
                    .to_ascii_lowercase()
                    .starts_with("hypervisorlaunchtype")
                {
                    line.split_whitespace().last().map(str::to_string)
                } else {
                    None
                }
            });

            match value {
                Some(value) => check_item(
                    "Hypervisor launch",
                    "hypervisorlaunchtype must be Auto",
                    Some(value.eq_ignore_ascii_case("auto")),
                    format!("hypervisorlaunchtype={value}"),
                    String::new(),
                    fix_hint,
                ),
                None => check_item(
                    "Hypervisor launch",
                    "hypervisorlaunchtype must be Auto",
                    None,
                    "hypervisorlaunchtype is not listed in the current BCD entry".to_string(),
                    "Set it explicitly if this requirement must be enforced.".to_string(),
                    fix_hint,
                ),
            }
        }
        Ok(output) => check_item(
            "Hypervisor launch",
            "hypervisorlaunchtype must be Auto",
            None,
            format!("bcdedit exited with code {:?}", output.code),
            join_non_empty(&[output.stderr, output.stdout], "; "),
            fix_hint,
        ),
        Err(error) => check_item(
            "Hypervisor launch",
            "hypervisorlaunchtype must be Auto",
            None,
            "Could not run bcdedit.exe".to_string(),
            error,
            fix_hint,
        ),
    }
}

fn run_admin_action(
    action: &str,
    script_body: &str,
    requires_restart: bool,
    success_message: &str,
) -> Result<ActionResult, String> {
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
            Duration::from_secs(ADMIN_ACTION_TIMEOUT_SECS),
        )
    } else {
        run_elevated_powershell_encoded(&encoded_script)
    };

    let output = fs::read_to_string(&log_path).unwrap_or_else(|_| String::new());
    let _ = fs::remove_file(&log_path);
    let _ = fs::remove_dir(&action_dir);

    let command_output = run_result?;
    if command_output.code == Some(0) {
        Ok(ActionResult {
            action: action.to_string(),
            succeeded: true,
            requires_restart,
            message: success_message.to_string(),
            output: join_non_empty(
                &[output, command_output.stdout, command_output.stderr],
                "\n",
            ),
        })
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
    let base_dir = std::env::temp_dir().join("zhiji");
    fs::create_dir_all(&base_dir)
        .map_err(|error| format!("Failed to create action temp directory: {error}"))?;

    let stamp = chrono::Local::now().timestamp_millis();
    let safe_action = action.replace(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_', "_");
    let action_dir = base_dir.join(format!("{safe_action}_{}_{}", std::process::id(), stamp));
    fs::create_dir(&action_dir)
        .map_err(|error| format!("Failed to create action workspace: {error}"))?;
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
        Duration::from_secs(ADMIN_ACTION_TIMEOUT_SECS),
    )
}

fn is_administrator() -> bool {
    ps_bool(
        r#"$p=New-Object Security.Principal.WindowsPrincipal([Security.Principal.WindowsIdentity]::GetCurrent()); $p.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)"#,
    )
    .unwrap_or(false)
}

fn ps_bool(script: &str) -> Result<bool, String> {
    let value = ps_trim(script)?;
    match value.to_ascii_lowercase().as_str() {
        "true" => Ok(true),
        "false" => Ok(false),
        other => Err(format!("Expected True or False, got {other}")),
    }
}

fn ps_trim(script: &str) -> Result<String, String> {
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

    if output.code == Some(0) {
        Ok(output.stdout.trim().to_string())
    } else {
        Err(join_non_empty(&[output.stderr, output.stdout], "; "))
    }
}

fn escape_ps_single(value: &str) -> String {
    value.replace('\'', "''")
}

fn run_program(program: &str, args: &[&str]) -> Result<CommandOutput, String> {
    run_program_with_timeout(
        program,
        args,
        Duration::from_secs(DEFAULT_COMMAND_TIMEOUT_SECS),
    )
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
        .map_err(|error| format!("Failed to run {program}: {error}"))?;
    let started_at = Instant::now();

    loop {
        if let Some(_status) = child
            .try_wait()
            .map_err(|error| format!("Failed to wait for {program}: {error}"))?
        {
            let output = child
                .wait_with_output()
                .map_err(|error| format!("Failed to read {program} output: {error}"))?;

            return Ok(CommandOutput {
                code: output.status.code(),
                stdout: String::from_utf8_lossy(&output.stdout).trim().to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            });
        }

        if started_at.elapsed() >= timeout {
            let _ = child.kill();
            let output = child.wait_with_output().ok();
            let stdout = output
                .as_ref()
                .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
                .unwrap_or_default();
            let stderr = output
                .as_ref()
                .map(|output| String::from_utf8_lossy(&output.stderr).trim().to_string())
                .unwrap_or_default();
            return Err(join_non_empty(
                &[
                    format!(
                        "Timed out after {} seconds running {program}",
                        timeout.as_secs()
                    ),
                    stderr,
                    stdout,
                ],
                "\n",
            ));
        }

        std::thread::sleep(Duration::from_millis(50));
    }
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
    fn parses_registry_pagefile_entry() {
        let parsed = parse_registry_pagefile_entry("C:\\pagefile.sys 4096 8192").unwrap();
        assert_eq!(parsed.name, "C:\\pagefile.sys");
        assert_eq!(parsed.initial_size_mb, Some(4096));
        assert_eq!(parsed.maximum_size_mb, Some(8192));
        assert_eq!(parsed.source, "注册表");
    }

    #[test]
    fn configured_pagefiles_merge_wmi_and_registry_sources() {
        let snapshot = CheckSnapshot {
            pagefile_settings: Some(vec![PageFileSettingSnapshot {
                name: Some("C:\\pagefile.sys".to_string()),
                initial_size_mb: Some(0),
                maximum_size_mb: Some(0),
            }]),
            registry_paging_files: Some(vec![
                "C:\\pagefile.sys 4096 8192".to_string(),
                "D:\\pagefile.sys 1024 2048".to_string(),
            ]),
            ..CheckSnapshot::default()
        };

        let configs = configured_pagefiles_from_snapshot(&snapshot);
        let c = configs
            .iter()
            .find(|config| config.name == "C:\\pagefile.sys")
            .unwrap();
        let d = configs
            .iter()
            .find(|config| config.name == "D:\\pagefile.sys")
            .unwrap();

        assert_eq!(configs.len(), 2);
        assert_eq!(c.initial_size_mb, Some(0));
        assert_eq!(c.maximum_size_mb, Some(0));
        assert_eq!(c.source, "WMI + 注册表");
        assert_eq!(d.initial_size_mb, Some(1024));
        assert_eq!(d.maximum_size_mb, Some(2048));
        assert_eq!(d.source, "注册表");
    }

    #[test]
    fn pagefile_configured_state_distinguishes_system_managed_custom_and_missing() {
        let auto = CheckSnapshot {
            automatic_managed_pagefile: Some(true),
            ..CheckSnapshot::default()
        };
        assert_eq!(pagefile_configured_state(&auto, &[]), "系统托管");

        let disabled_empty = CheckSnapshot {
            automatic_managed_pagefile: Some(false),
            ..CheckSnapshot::default()
        };
        assert_eq!(
            pagefile_configured_state(&disabled_empty, &[]),
            "未配置页面文件"
        );

        let custom = CheckSnapshot {
            automatic_managed_pagefile: Some(false),
            ..CheckSnapshot::default()
        };
        assert_eq!(
            pagefile_configured_state(
                &custom,
                &[PageFileConfigInfo {
                    name: "C:\\pagefile.sys".to_string(),
                    initial_size_mb: Some(4096),
                    maximum_size_mb: Some(8192),
                    source: "注册表".to_string(),
                }]
            ),
            "自定义"
        );
    }

    #[test]
    fn div_ceil_handles_zero_divisor_and_rounding() {
        assert_eq!(div_ceil(0, 0), 0);
        assert_eq!(div_ceil(1, 8), 1);
        assert_eq!(div_ceil(16, 8), 2);
        assert_eq!(div_ceil(17, 8), 3);
    }

    #[test]
    fn pagefile_recommendation_prefers_fixed_size_until_32gb_then_system_managed() {
        let small = pagefile_recommendation(16 * 1024, Some(512_000));
        assert_eq!(small.preferred_mode, "固定自定义");
        assert_eq!(small.recommended_initial_mb, Some(16 * 1024 + 300));
        assert_eq!(small.recommended_maximum_mb, Some(16 * 1024 + 300));

        let large = pagefile_recommendation(64 * 1024, Some(1_024_000));
        assert_eq!(large.preferred_mode, "系统托管");
        assert_eq!(large.recommended_initial_mb, None);
        assert_eq!(large.recommended_maximum_mb, None);
        assert_eq!(large.system_managed_max_estimate_mb, 64 * 1024 * 3);
    }

    #[test]
    fn summarize_fast_startup_accounts_for_hibernation_state() {
        let enabled = CheckSnapshot {
            hiberboot_enabled: Some(true),
            hibernation_enabled: Some(true),
            ..CheckSnapshot::default()
        };
        assert_eq!(summarize_fast_startup_state(&enabled), "Enabled");

        let unavailable = CheckSnapshot {
            hiberboot_enabled: Some(true),
            hibernation_enabled: Some(false),
            ..CheckSnapshot::default()
        };
        assert_eq!(
            summarize_fast_startup_state(&unavailable),
            "Disabled (hibernation unavailable)"
        );

        let unknown = CheckSnapshot {
            fast_startup_details: Some("registry failed".to_string()),
            ..CheckSnapshot::default()
        };
        assert_eq!(
            summarize_fast_startup_state(&unknown),
            "Unknown (registry failed)"
        );
    }

    #[test]
    fn summarizes_memory_compression_state() {
        let enabled = CheckSnapshot {
            memory_compression_enabled: Some(true),
            ..CheckSnapshot::default()
        };
        assert_eq!(summarize_memory_compression_state(&enabled), "Enabled");

        let disabled = CheckSnapshot {
            memory_compression_enabled: Some(false),
            ..CheckSnapshot::default()
        };
        assert_eq!(summarize_memory_compression_state(&disabled), "Disabled");

        let unknown = CheckSnapshot {
            memory_compression_details: Some("Get-MMAgent failed".to_string()),
            ..CheckSnapshot::default()
        };
        assert_eq!(
            summarize_memory_compression_state(&unknown),
            "Unknown (Get-MMAgent failed)"
        );
    }

    #[test]
    fn expand_windows_path_replaces_known_environment_tokens() {
        let expanded = expand_windows_path("%SystemRoot%\\Minidump")
            .to_string_lossy()
            .to_string();
        assert!(
            expanded.ends_with("\\Minidump"),
            "expanded path should preserve the requested suffix: {expanded}"
        );
        assert!(
            !expanded.contains("%SystemRoot%"),
            "SystemRoot token should be expanded: {expanded}"
        );
    }

    #[test]
    fn check_run_mode_defaults_to_fast_and_accepts_full() {
        assert_eq!(CheckRunMode::from_option(None), CheckRunMode::Fast);
        assert_eq!(
            CheckRunMode::from_option(Some("fast".to_string())),
            CheckRunMode::Fast
        );
        assert_eq!(
            CheckRunMode::from_option(Some("FULL".to_string())),
            CheckRunMode::Full
        );
    }

    #[test]
    fn fast_snapshot_merge_keeps_stable_fields_and_updates_dynamic_fields() {
        let cached = CheckSnapshot {
            board: Some("Cached board".to_string()),
            secure_boot_state: Some("Disabled".to_string()),
            tpm_present: Some(false),
            hyperv_state: Some("Microsoft-Hyper-V=Disabled".to_string()),
            hypervisor_launch_type: Some("Off".to_string()),
            memory_compression_enabled: Some(false),
            total_physical_memory_mb: Some(32768),
            crash_dump_enabled: Some(0),
            ..CheckSnapshot::default()
        };
        let fast = CheckSnapshot {
            hyperv_state: Some("Microsoft-Hyper-V=Enabled".to_string()),
            hypervisor_launch_type: Some("Auto".to_string()),
            memory_compression_enabled: Some(true),
            total_physical_memory_mb: Some(49152),
            crash_dump_enabled: Some(3),
            ..CheckSnapshot::default()
        };

        let merged = merge_fast_snapshot(cached, fast);

        assert_eq!(merged.board.as_deref(), Some("Cached board"));
        assert_eq!(merged.secure_boot_state.as_deref(), Some("Disabled"));
        assert_eq!(merged.tpm_present, Some(false));
        assert_eq!(
            merged.hyperv_state.as_deref(),
            Some("Microsoft-Hyper-V=Enabled")
        );
        assert_eq!(merged.hypervisor_launch_type.as_deref(), Some("Auto"));
        assert_eq!(merged.memory_compression_enabled, Some(true));
        assert_eq!(merged.total_physical_memory_mb, Some(49152));
        assert_eq!(merged.crash_dump_enabled, Some(3));
    }
}
