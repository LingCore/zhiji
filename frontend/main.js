const THEME_STORAGE_KEY = "pc-requirements-theme";
const PROJECT_REPOSITORY_URL = "https://github.com/LingCore/zhiji";
const hasTauri = Boolean(window.__TAURI__?.core?.invoke);

function storedThemePreference() {
  try {
    return window.localStorage?.getItem(THEME_STORAGE_KEY);
  } catch {
    return null;
  }
}

function systemThemePreference() {
  return window.matchMedia?.("(prefers-color-scheme: light)")?.matches ? "light" : "dark";
}

function preferredTheme() {
  const stored = storedThemePreference();
  return stored === "light" || stored === "dark" ? stored : systemThemePreference();
}

document.documentElement.dataset.theme = preferredTheme();

function isEditableTarget(target) {
  return Boolean(target?.closest?.('input, textarea, select, [contenteditable=""], [contenteditable="true"]'));
}

function installDesktopShellGuards() {
  document.addEventListener("contextmenu", (event) => {
    event.preventDefault();
  });

  document.addEventListener("dragstart", (event) => {
    event.preventDefault();
  });

  document.addEventListener("selectstart", (event) => {
    if (!isEditableTarget(event.target)) {
      event.preventDefault();
    }
  });

  window.addEventListener(
    "keydown",
    (event) => {
      const key = event.key.toLowerCase();
      const isReload = event.key === "F5" || ((event.ctrlKey || event.metaKey) && key === "r");
      const isHistoryNavigation = event.altKey && (event.key === "ArrowLeft" || event.key === "ArrowRight");
      if (isReload || isHistoryNavigation) {
        event.preventDefault();
      }
    },
    { capture: true }
  );
}

const previewReport = {
  computer_name: "PREVIEW",
  generated_at: new Date().toISOString(),
  is_administrator: true,
  hardware: {
    board: "GIGABYTE Z790M AORUS ELITE AX ICE",
    bios: "American Megatrends F7a",
    cpu: "Intel(R) Core(TM) i5-14600KF"
  },
  feature_states: {
    hyper_v: "Enabled",
    virtual_machine_platform: "Disabled",
    windows_hypervisor_platform: "Disabled",
    hypervisor_launch: "Auto",
    fast_startup: "Enabled",
    memory_compression: "Enabled"
  },
  virtual_memory: {
    total_physical_memory_mb: 49152,
    automatic_managed_pagefile: true,
    configured_state: "系统托管",
    system_drive: "C:",
    system_drive_total_mb: 953000,
    system_drive_free_mb: 420000,
    configured_pagefiles: [
      {
        name: "C:\\pagefile.sys",
        initial_size_mb: 0,
        maximum_size_mb: 0,
        source: "注册表"
      }
    ],
    pagefiles: [
      {
        name: "C:\\pagefile.sys",
        initial_size_mb: null,
        maximum_size_mb: null,
        allocated_base_size_mb: 4864,
        current_usage_mb: 312,
        peak_usage_mb: 980,
        temp_page_file: false
      }
    ],
    recommendation: {
      preferred_mode: "系统托管",
      recommended_initial_mb: null,
      recommended_maximum_mb: null,
      system_managed_min_estimate_mb: 6144,
      system_managed_max_estimate_mb: 119125,
      formula: "推荐值 = 系统托管；系统托管上限估算 = min(max(3 x RAM, 4096 MB), 系统盘容量 / 8)。",
      reason: "物理内存超过 32 GB 时，固定页面文件会占用过多磁盘空间，推荐由 Windows 自动管理。"
    },
    details: ""
  },
  blue_screen: {
    crash_dump_enabled: 3,
    crash_dump_label: "小内存转储",
    minidump_dir: "C:\\Windows\\Minidump",
    minidump_dir_configured: true,
    minidump_dir_exists: true,
    dump_count: 1,
    recent_dumps: [
      {
        name: "052926-12345-01.dmp",
        path: "C:\\Windows\\Minidump\\052926-12345-01.dmp",
        size_kb: 256,
        modified: new Date().toISOString()
      }
    ],
    tool_path: "C:\\Users\\bob\\Desktop\\bluescreenview155.exe",
    tool_available: true,
    collection_ready: true,
    details: "CrashDumpEnabled=3; MinidumpDir=%SystemRoot%\\Minidump"
  },
  results: [
    {
      name: "CPU virtualization",
      required: "Intel VT-x must be enabled in firmware",
      status: "PASS",
      passed: true,
      detected: "HypervisorPresent=True; hardware virtualization is already in use.",
      details: "Preview data",
      fix_hint: ""
    },
    {
      name: "VT-d / IOMMU",
      required: "Intel VT-d / IOMMU must be enabled in firmware",
      status: "PASS",
      passed: true,
      detected: "DMAProtectionAvailable=True",
      details: "Win32_DeviceGuard AvailableSecurityProperties=1, 3, 4, 5, 6, 7, 8",
      fix_hint: ""
    },
    {
      name: "CSM / Legacy Boot",
      required: "Current boot mode must be UEFI, not Legacy/CSM",
      status: "PASS",
      passed: true,
      detected: "BootMode=UEFI",
      details: "Preview data",
      fix_hint: ""
    },
    {
      name: "Microsoft Hyper-V",
      required: "Hyper-V feature/role must be enabled",
      status: "PASS",
      passed: true,
      detected: "Microsoft-Hyper-V=Enabled",
      details: "",
      fix_hint: ""
    },
    {
      name: "Secure Boot",
      required: "Secure Boot must be disabled",
      status: "PASS",
      passed: true,
      detected: "SecureBoot=Disabled",
      details: "",
      fix_hint: ""
    },
    {
      name: "TPM",
      required: "TPM must be disabled or not visible to Windows",
      status: "PASS",
      passed: true,
      detected: "TpmPresent=False",
      details: "",
      fix_hint: ""
    },
    {
      name: "Hypervisor launch",
      required: "hypervisorlaunchtype must be Auto",
      status: "PASS",
      passed: true,
      detected: "hypervisorlaunchtype=Auto",
      details: "",
      fix_hint: ""
    }
  ]
};

const actionMeta = {
  enable_hyper_v: {
    busy: "正在启用 Hyper-V（微软虚拟机监控程序）...",
    confirm: "将启用 Hyper-V（微软虚拟机监控程序）。可能会弹出 UAC，执行后需要重启。继续？"
  },
  disable_hyper_v: {
    busy: "正在禁用 Hyper-V（微软虚拟机监控程序）...",
    confirm: "将禁用 Hyper-V（微软虚拟机监控程序）。依赖 Hyper-V 的功能会受影响，执行后需要重启。继续？"
  },
  enable_virtual_machine_platform: {
    busy: "正在启用虚拟机平台（Virtual Machine Platform）...",
    confirm: "将启用虚拟机平台（Virtual Machine Platform）。WSL2、部分容器和虚拟化组件可能需要它，执行后需要重启。继续？"
  },
  disable_virtual_machine_platform: {
    busy: "正在禁用虚拟机平台（Virtual Machine Platform）...",
    confirm: "将禁用虚拟机平台（Virtual Machine Platform）。WSL2、部分容器和虚拟化组件可能受影响，执行后需要重启。继续？"
  },
  enable_windows_hypervisor_platform: {
    busy: "正在启用 Windows 虚拟机监控平台（Windows Hypervisor Platform）...",
    confirm: "将启用 Windows 虚拟机监控平台（Windows Hypervisor Platform）。第三方虚拟机、模拟器可能需要它，执行后需要重启。继续？"
  },
  disable_windows_hypervisor_platform: {
    busy: "正在禁用 Windows 虚拟机监控平台（Windows Hypervisor Platform）...",
    confirm: "将禁用 Windows 虚拟机监控平台（Windows Hypervisor Platform）。第三方虚拟机、模拟器可能受影响，执行后需要重启。继续？"
  },
  set_hypervisor_auto: {
    busy: "正在设置 hypervisorlaunchtype=auto...",
    confirm: "将把虚拟机监控程序启动项（hypervisorlaunchtype）设置为 auto。可能会弹出 UAC，执行后需要重启。继续？"
  },
  set_hypervisor_off: {
    busy: "正在设置 hypervisorlaunchtype=off...",
    confirm: "将把虚拟机监控程序启动项（hypervisorlaunchtype）设置为 off。Hyper-V 相关功能会停止随系统启动，执行后需要重启。继续？"
  },
  enable_fast_startup: {
    busy: "正在开启 Windows 快速启动...",
    confirm: "将开启 Windows 快速启动。若休眠不可用，会先启用用于快速启动的休眠文件；设置会影响下一次关机后的启动。继续？",
    details: "需要管理员权限；通常不需要立即重启，但下一次关机和开机时才会体现差异。"
  },
  disable_fast_startup: {
    busy: "正在关闭 Windows 快速启动...",
    confirm: "将关闭 Windows 快速启动，但不会禁用休眠功能；下一次关机将更接近完整关机。继续？",
    details: "需要管理员权限；通常不需要立即重启，下一次关机后生效。"
  },
  enable_memory_compression: {
    busy: "正在启用内存压缩（Memory Compression）...",
    confirm: "将启用 Windows 内存压缩。8GB 或更低内存通常建议保持开启，执行后建议重启。继续？",
    details: "需要管理员权限；Windows 会把一部分冷数据压缩在内存里，减少写入页面文件的机会。"
  },
  disable_memory_compression: {
    busy: "正在禁用内存压缩（Memory Compression）...",
    confirm: "将禁用 Windows 内存压缩。16GB+ 游戏电脑有时会通过关闭它来降低少量 CPU 开销，执行后建议重启。继续？",
    details: "需要管理员权限；禁用后内存压力较高时可能更依赖页面文件。"
  },
  restart_windows: {
    busy: "正在请求重启...",
    confirm: "电脑会立即重启。确认继续？"
  }
};

const viewTitles = {
  overview: ["Overview", "概览"],
  controls: ["Controls", "快速控制"],
  gaming: ["Gaming optimizer", "竞技模式"],
  memory: ["Virtual memory", "虚拟内存"],
  bsod: ["Blue screen", "蓝屏分析"],
  checks: ["State details", "状态明细"],
  hardware: ["Hardware", "硬件信息"]
};

viewTitles.monitorIdentity = ["显示器身份", "显示器身份"];

const gamingPreviewExePath = "C:\\Games\\Example\\game.exe";

const monitorIdentityPreviewStatus = {
  is_administrator: true,
  active_monitor_count: 1,
  pending_confirmation: null,
  change_count: 0,
  monitors: [
    {
      instance_name: "DISPLAY\\LHC906A\\PREVIEW_0",
      device_instance_id: "DISPLAY\\LHC906A\\PREVIEW",
      hardware_id: "MONITOR\\LHC906A",
      registry_path: "SYSTEM\\CurrentControlSet\\Enum\\DISPLAY\\LHC906A\\PREVIEW\\Device Parameters",
      active: true,
      edid_present: true,
      override_present: false,
      windows_reported: {
        manufacturer_id: "LHC",
        product_code: "906A",
        serial_number: "P2710VMAX01",
        monitor_name: "P2710V MAX",
        hardware_id: "MONITOR\\LHC906A"
      },
      current: {
        manufacturer_id: "LHC",
        product_code_hex: "906A",
        numeric_serial: 100200300,
        serial_number: "P2710VMAX01",
        monitor_name: "P2710V MAX",
        checksum_valid: true,
        windows_hardware_id: "MONITOR\\LHC906A"
      },
      original: {
        manufacturer_id: "LHC",
        product_code_hex: "906A",
        numeric_serial: 100200300,
        serial_number: "P2710VMAX01",
        monitor_name: "P2710V MAX",
        checksum_valid: true,
        windows_hardware_id: "MONITOR\\LHC906A"
      }
    }
  ],
  changes: []
};

let gamingSelectedGamePath = "";
let gamingOptimizerStatus = null;
let monitorIdentityStatus = null;
let monitorIdentityInitPromise = null;
let monitorIdentityCountdownTimer = null;

const invoke = () => {
  if (hasTauri) {
    return window.__TAURI__.core.invoke;
  }

  return async (command, args) => {
    if (command === "run_checks") {
      return previewReport;
    }

    if (command === "gaming_get_optimizer_status") {
      return {
        is_administrator: true,
        hags: {
          key: "unknown",
          label: "系统默认",
          detail: "未显式设置 HwSchMode。",
          recommended: "启用后实测",
          requires_admin: true,
          requires_restart: true
        },
        game_capture: {
          key: "off",
          label: "未完全禁用",
          detail: "GameDVR_Enabled=1，AppCaptureEnabled=1。",
          recommended: "禁用",
          requires_admin: false,
          requires_restart: false
        },
        game_mode: {
          key: "unknown",
          label: "系统默认 / 未禁用",
          detail: "实验项，建议逐机测试。",
          recommended: "实验项，逐机测试",
          requires_admin: false,
          requires_restart: false
        },
        fullscreen_optimization: {
          key: args?.gamePath ? "off" : "unknown",
          label: args?.gamePath ? "未配置" : "未选择游戏",
          detail: args?.gamePath ? "没有 AppCompatFlags Layers 记录。" : "选择游戏 exe 后可检测。",
          recommended: "按游戏单独禁用",
          requires_admin: false,
          requires_restart: false
        },
        selected_game_path: args?.gamePath || gamingSelectedGamePath || null,
        pending_reboot: false,
        change_count: 0,
        changes: []
      };
    }

    if (command === "gaming_apply_optimizer_action") {
      return {
        action: args?.request?.action || "preview",
        message: "浏览器预览模式：这里会写入对应的竞技优化注册表项。",
        requires_restart: args?.request?.action === "enable_hags" || args?.request?.action === "default_hags",
        output: args?.request?.game_path || "Preview only"
      };
    }

    if (command === "gaming_apply_competitive_preset") {
      return {
        action: "competitive_preset",
        message: "浏览器预览模式：这里会应用安全竞技 FPS 预设。",
        requires_restart: Boolean(args?.request?.include_hags),
        output: args?.request?.game_path || "Preview only"
      };
    }

    if (command === "gaming_restore_optimizer_changes") {
      return {
        action: "restore_gaming_optimizer_changes",
        message: "浏览器预览模式：这里会按记录还原竞技模式改动。",
        requires_restart: true,
        output: "Preview only"
      };
    }

    if (command === "monitor_identity_get_status") {
      return monitorIdentityPreviewStatus;
    }

    if (command === "monitor_identity_apply_override" || command === "monitor_identity_install_inf_override") {
      const now = Date.now();
      const isInfInstall = command === "monitor_identity_install_inf_override";
      const pending = {
        token: `preview-${now}`,
        change_id: `preview-change-${now}`,
        monitor_device_instance_id: args?.request?.monitor_device_instance_id || "DISPLAY\\LHC906A\\PREVIEW",
        expires_at: new Date(now + 30_000).toISOString(),
        seconds_remaining: 30
      };
      monitorIdentityPreviewStatus.pending_confirmation = pending;
      monitorIdentityPreviewStatus.change_count += 1;
      monitorIdentityPreviewStatus.changes.unshift({
        id: pending.change_id,
        status: "pending",
        apply_mode: isInfInstall ? "inf_driver" : "registry_override",
        applied_at: new Date(now).toISOString(),
        confirmed_at: null,
        rolled_back_at: null,
        rollback_token: pending.token,
        expires_at: pending.expires_at,
        monitor_device_instance_id: pending.monitor_device_instance_id,
        original_hardware_id: "MONITOR\\LHC906A",
        target_hardware_id: `MONITOR\\${args?.request?.manufacturer_id || "DEL"}${args?.request?.product_code_hex || "A123"}`,
        registry_path: "预览模式注册表路径",
        generated_inf_path: "预览模式 INF",
        published_driver_inf: isInfInstall ? "oem42.inf" : null,
        output: isInfInstall ? "仅预览：这里会执行 pnputil /add-driver /install" : "仅预览"
      });
      return {
        action: command,
        succeeded: true,
        message: isInfInstall ? "预览模式：已应用显示器身份修改。" : "预览模式：已写入注册表覆盖。",
        output: "仅预览",
        pending_confirmation: pending
      };
    }

    if (command === "monitor_identity_confirm_override") {
      monitorIdentityPreviewStatus.pending_confirmation = null;
      for (const change of monitorIdentityPreviewStatus.changes) {
        if (change.rollback_token === args?.token) {
          change.status = "confirmed";
          change.confirmed_at = new Date().toISOString();
        }
      }
      return {
        action: "monitor_identity_confirm_override",
        succeeded: true,
        message: "预览模式：已保留显示器身份覆盖。",
        output: "仅预览",
        pending_confirmation: null
      };
    }

    if (command === "monitor_identity_reenumerate_device") {
      const now = Date.now();
      const pending = {
        token: `preview-reenum-${now}`,
        change_id: `preview-reenum-change-${now}`,
        monitor_device_instance_id: args?.request?.monitor_device_instance_id || "DISPLAY\\LHC906A\\PREVIEW",
        expires_at: new Date(now + 30_000).toISOString(),
        seconds_remaining: 30
      };
      monitorIdentityPreviewStatus.pending_confirmation = pending;
      monitorIdentityPreviewStatus.change_count += 1;
      monitorIdentityPreviewStatus.changes.unshift({
        id: pending.change_id,
        status: "pending",
        apply_mode: "reenumerate_device",
        applied_at: new Date(now).toISOString(),
        confirmed_at: null,
        rolled_back_at: null,
        rollback_token: pending.token,
        expires_at: pending.expires_at,
        monitor_device_instance_id: pending.monitor_device_instance_id,
        original_hardware_id: "MONITOR\\LHC906A",
        target_hardware_id: "MONITOR\\BUX0F04",
        registry_path: "预览模式注册表路径",
        generated_inf_path: "",
        published_driver_inf: "oem42.inf",
        output: "仅预览：这里会执行 pnputil /remove-device 和 /scan-devices"
      });
      return {
        action: "monitor_identity_reenumerate_device",
        succeeded: true,
        message: "预览模式：已强制重枚举显示器。",
        output: "仅预览",
        pending_confirmation: pending
      };
    }

    if (command === "monitor_identity_restore_change") {
      monitorIdentityPreviewStatus.pending_confirmation = null;
      for (const change of monitorIdentityPreviewStatus.changes) {
        if (change.status !== "rolled_back") {
          change.status = "rolled_back";
          change.rolled_back_at = new Date().toISOString();
          break;
        }
      }
      return {
        action: "monitor_identity_restore_change",
        succeeded: true,
        message: "预览模式：已还原显示器身份覆盖。",
        output: "仅预览",
        pending_confirmation: null
      };
    }

    if (command === "apply_requirement_action") {
      return {
        action: args?.action || "preview",
        succeeded: true,
        requires_restart: true,
        message: "浏览器预览模式：这里会执行对应的系统命令。",
        output: "Preview only"
      };
    }

    if (command === "set_virtual_memory_system_managed") {
      return {
        action: "set_virtual_memory_system_managed",
        succeeded: true,
        requires_restart: true,
        message: "浏览器预览模式：这里会设置为系统管理虚拟内存。",
        output: "Preview only"
      };
    }

    if (command === "set_virtual_memory_custom") {
      return {
        action: "set_virtual_memory_custom",
        succeeded: true,
        requires_restart: true,
        message: `浏览器预览模式：这里会设置虚拟内存为 ${args?.request?.initial_size_mb || "-"}-${args?.request?.maximum_size_mb || "-"} MB。`,
        output: "Preview only"
      };
    }

    if (command === "restore_initial_config") {
      return {
        action: "restore_initial_config",
        succeeded: true,
        requires_restart: true,
        message: "浏览器预览模式：这里会恢复第一次保存的初始配置。",
        output:
          "Hyper-V（微软虚拟机监控程序）已恢复\n内存压缩（Memory Compression）已恢复\n虚拟内存已恢复\n固件项（BIOS/UEFI）不会自动恢复：CPU 虚拟化（VT-x / SVM）、设备直通（VT-d / IOMMU）、安全启动（Secure Boot）、可信平台模块（TPM / PTT / fTPM）需要在固件设置里手动修改。"
      };
    }

    if (command === "configure_minidump_collection") {
      return {
        action: "configure_minidump_collection",
        succeeded: true,
        requires_restart: false,
        message: "浏览器预览模式：这里会开启小内存转储。",
        output: "CrashDumpEnabled=3\nMinidumpDir=%SystemRoot%\\Minidump"
      };
    }

    if (command === "open_bluescreenview") {
      return {
        action: "open_bluescreenview",
        succeeded: true,
        requires_restart: false,
        message: "浏览器预览模式：这里会打开蓝屏查看器（BlueScreenView）。",
        output: "Preview only"
      };
    }

    if (command === "open_project_repository") {
      window.__lastOpenedProjectUrl = PROJECT_REPOSITORY_URL;
      return {
        action: "open_project_repository",
        succeeded: true,
        requires_restart: false,
        message: "浏览器预览模式：这里会用默认浏览器打开 GitHub。",
        output: PROJECT_REPOSITORY_URL
      };
    }

    if (command === "export_bluescreen_report") {
      return {
        action: "export_bluescreen_report",
        succeeded: true,
        requires_restart: false,
        message: "浏览器预览模式：这里会导出蓝屏 CSV 报告。",
        output: "C:\\Temp\\pc_requirements_bluescreen_preview.csv"
      };
    }

    if (command === "restart_to_firmware") {
      throw new Error("浏览器预览模式不能重启进入 BIOS");
    }

    throw new Error(`Unknown command: ${command}`);
  };
};

const checksListEl = document.querySelector("#checksList");
const messageEl = document.querySelector("#message");
const titlebarEl = document.querySelector("#appTitlebar");
const windowMinimizeButton = document.querySelector("#windowMinimizeButton");
const windowMaximizeButton = document.querySelector("#windowMaximizeButton");
const windowMaximizeIcon = document.querySelector("#windowMaximizeIcon");
const windowCloseButton = document.querySelector("#windowCloseButton");
const themeToggleButton = document.querySelector("#themeToggleButton");
const themeToggleLabel = document.querySelector("#themeToggleLabel");
const themeToggleIconUse = document.querySelector("#themeToggleIcon use");
const aboutButton = document.querySelector("#aboutButton");
const refreshButton = document.querySelector("#refreshButton");
const restoreInitialButton = document.querySelector("#restoreInitialButton");
const actionButtons = Array.from(document.querySelectorAll(".action-button"));
const firmwareButtons = Array.from(document.querySelectorAll(".firmware-button"));
const navItems = Array.from(document.querySelectorAll(".nav-item"));
const panes = Array.from(document.querySelectorAll(".pane"));
const pagefileInitialInput = document.querySelector("#pagefileInitialInput");
const pagefileMaximumInput = document.querySelector("#pagefileMaximumInput");
const applyCustomPagefileButton = document.querySelector("#applyCustomPagefile");
const configureDumpButton = document.querySelector("#configureDumpButton");
const openBlueScreenViewButton = document.querySelector("#openBlueScreenViewButton");
const exportBlueScreenReportButton = document.querySelector("#exportBlueScreenReportButton");
const formulaOptionsEl = document.querySelector("#formulaOptions");
const gamingAdminStateEl = document.querySelector("#gamingAdminState");
const gamingRebootStateEl = document.querySelector("#gamingRebootState");
const gamingChangeStateEl = document.querySelector("#gamingChangeState");
const gamingActionMessageEl = document.querySelector("#gamingActionMessage");
const gamingSelectedGamePathEl = document.querySelector("#gamingSelectedGamePath");
const gamingPresetHags = document.querySelector("#gamingPresetHags");
const gamingPresetCapture = document.querySelector("#gamingPresetCapture");
const gamingPresetFullscreen = document.querySelector("#gamingPresetFullscreen");
const gamingApplyPresetButton = document.querySelector("#gamingApplyPresetButton");
const gamingRestoreChangesButton = document.querySelector("#gamingRestoreChangesButton");
const gamingRefreshButton = document.querySelector("#gamingRefreshButton");
const gamingBrowseGameButton = document.querySelector("#gamingBrowseGameButton");
const gamingActionButtons = Array.from(document.querySelectorAll(".gaming-action-button"));
const monitorIdentitySelect = document.querySelector("#monitorIdentitySelect");
const monitorManufacturerInput = document.querySelector("#monitorManufacturerInput");
const monitorProductInput = document.querySelector("#monitorProductInput");
const monitorNumericSerialInput = document.querySelector("#monitorNumericSerialInput");
const monitorSerialInput = document.querySelector("#monitorSerialInput");
const monitorNameInput = document.querySelector("#monitorNameInput");
const monitorIdentityApplyButton = document.querySelector("#monitorIdentityApplyButton");
const monitorIdentityInstallInfButton = document.querySelector("#monitorIdentityInstallInfButton");
const monitorIdentityReenumerateButton = document.querySelector("#monitorIdentityReenumerateButton");
const monitorIdentityRandomButton = document.querySelector("#monitorIdentityRandomButton");
const monitorIdentityRefreshButton = document.querySelector("#monitorIdentityRefreshButton");
const monitorIdentityConfirmButton = document.querySelector("#monitorIdentityConfirmButton");
const monitorIdentityRollbackButton = document.querySelector("#monitorIdentityRollbackButton");
const monitorIdentityMessageEl = document.querySelector("#monitorIdentityMessage");
const monitorIdentityLogEl = document.querySelector("#monitorIdentityLog");
const monitorIdentityCurrentEl = document.querySelector("#monitorIdentityCurrent");
const monitorIdentityAdminStateEl = document.querySelector("#monitorIdentityAdminState");
const monitorIdentityActiveStateEl = document.querySelector("#monitorIdentityActiveState");
const monitorIdentityPendingStateEl = document.querySelector("#monitorIdentityPendingState");
const dialogBackdrop = document.querySelector("#appDialog");
const dialogAccent = document.querySelector("#dialogAccent");
const dialogEyebrow = document.querySelector("#dialogEyebrow");
const dialogTitle = document.querySelector("#dialogTitle");
const dialogBody = document.querySelector("#dialogBody");
const dialogDetails = document.querySelector("#dialogDetails");
const dialogRiskList = document.querySelector("#dialogRiskList");
const dialogIcon = document.querySelector("#dialogIcon");
const dialogIconUse = document.querySelector("#dialogIcon use");
const dialogLinkAction = document.querySelector("#dialogLinkAction");
const dialogLinkActionLabel = document.querySelector("#dialogLinkActionLabel");
const dialogLinkActionIconUse = document.querySelector("#dialogLinkAction use");
const dialogCancel = document.querySelector("#dialogCancel");
const dialogConfirm = document.querySelector("#dialogConfirm");
const dialogAcknowledgeWrap = document.querySelector("#dialogAcknowledgeWrap");
const dialogAcknowledge = document.querySelector("#dialogAcknowledge");
const pagefileActionButtons = [applyCustomPagefileButton].filter(Boolean);
const bsodActionButtons = [
  configureDumpButton,
  openBlueScreenViewButton,
  exportBlueScreenReportButton
].filter(Boolean);
const gamingPanelButtons = [
  gamingApplyPresetButton,
  gamingRestoreChangesButton,
  gamingRefreshButton,
  gamingBrowseGameButton,
  ...gamingActionButtons
].filter(Boolean);
const monitorIdentityPanelButtons = [
  monitorIdentityApplyButton,
  monitorIdentityInstallInfButton,
  monitorIdentityReenumerateButton,
  monitorIdentityRandomButton,
  monitorIdentityRefreshButton,
  monitorIdentityConfirmButton,
  monitorIdentityRollbackButton
].filter(Boolean);

function currentAppWindow() {
  const windowApi = window.__TAURI__?.window;
  if (!windowApi?.getCurrentWindow) {
    return null;
  }
  try {
    return windowApi.getCurrentWindow();
  } catch {
    return null;
  }
}

async function updateWindowMaximizeIcon(appWindow = currentAppWindow()) {
  if (!appWindow || !windowMaximizeButton || typeof appWindow.isMaximized !== "function") {
    return;
  }
  try {
    const isMaximized = await appWindow.isMaximized();
    document.documentElement.dataset.windowMaximized = String(isMaximized);
    windowMaximizeButton.setAttribute("aria-label", isMaximized ? "还原窗口" : "最大化窗口");
    windowMaximizeButton.title = isMaximized ? "还原窗口" : "最大化窗口";
    if (windowMaximizeIcon) {
      windowMaximizeIcon.classList.toggle("is-restore", isMaximized);
    }
  } catch {
    // Window state is non-critical; keep the titlebar usable if the query fails.
  }
}

function bindWindowControls() {
  const appWindow = currentAppWindow();
  if (!appWindow) {
    return;
  }

  windowMinimizeButton?.addEventListener("click", async () => {
    try {
      await appWindow.minimize?.();
    } catch {
      // Ignore window-control failures so the rest of the UI stays responsive.
    }
  });

  windowMaximizeButton?.addEventListener("click", async () => {
    try {
      await appWindow.toggleMaximize?.();
      await updateWindowMaximizeIcon(appWindow);
    } catch {
      // Ignore window-control failures so the rest of the UI stays responsive.
    }
  });

  windowCloseButton?.addEventListener("click", async () => {
    try {
      await appWindow.close?.();
    } catch {
      // Ignore window-control failures so the rest of the UI stays responsive.
    }
  });

  titlebarEl?.addEventListener("dblclick", async (event) => {
    if (event.target?.closest?.(".titlebar-button")) {
      return;
    }
    try {
      await appWindow.toggleMaximize?.();
      await updateWindowMaximizeIcon(appWindow);
    } catch {
      // Ignore window-control failures so the rest of the UI stays responsive.
    }
  });

  if (typeof appWindow.onResized === "function") {
    Promise.resolve(appWindow.onResized(() => updateWindowMaximizeIcon(appWindow))).catch(() => {});
  }
  void updateWindowMaximizeIcon(appWindow);
}
let latestReport = previewReport;
let selectedFormulaId = null;
let dialogResolver = null;
let gamingOptimizerInitPromise = null;
let runChecksPromise = null;
let runChecksPromiseMode = null;
let formulaOptionButtons = [];

function waitForNextPaint() {
  return new Promise((resolve) => {
    if (typeof window.requestAnimationFrame === "function") {
      window.requestAnimationFrame(() => resolve());
      return;
    }
    window.setTimeout(resolve, 0);
  });
}

function setText(selector, value) {
  const element = document.querySelector(selector);
  if (element) {
    element.textContent = value || "-";
  }
}

function setIconHref(useElement, symbolId) {
  if (useElement) {
    useElement.setAttribute("href", `#${symbolId}`);
  }
}

function setStateText(selector, state) {
  const element = document.querySelector(selector);
  if (!element) {
    return;
  }
  element.textContent = state?.label || "-";
  element.className = `feature-state state-text state-${state?.key || "unknown"}`;
}

function statusClassForKey(key) {
  if (key === "on" || key === "managed") {
    return "state-on";
  }
  if (key === "off") {
    return "state-off";
  }
  if (key === "auto") {
    return "state-auto";
  }
  return "state-unknown";
}

function setGamingBadge(selector, state) {
  const element = document.querySelector(selector);
  if (!element) {
    return;
  }
  element.textContent = state?.label || "-";
  element.className = `status-pill ${statusClassForKey(state?.key)}`;
}

function renderGamingOptimizerStatus(status = gamingOptimizerStatus) {
  gamingOptimizerStatus = status;
  if (!status) {
    return;
  }

  if (gamingAdminStateEl) {
    gamingAdminStateEl.textContent = `管理员：${status.is_administrator ? "是" : "否"}`;
  }
  if (gamingRebootStateEl) {
    gamingRebootStateEl.textContent = `待重启：${status.pending_reboot ? "是" : "否"}`;
  }
  if (gamingChangeStateEl) {
    gamingChangeStateEl.textContent = `变更记录：${status.change_count || 0}`;
  }
  if (status.selected_game_path) {
    gamingSelectedGamePath = status.selected_game_path;
  }
  if (gamingSelectedGamePathEl) {
    gamingSelectedGamePathEl.textContent = gamingSelectedGamePath || "未选择游戏 exe";
  }

  setGamingBadge("#gamingHagsBadge", status.hags);
  setText("#gamingHagsDetail", status.hags?.detail || "-");
  setGamingBadge("#gamingCaptureBadge", status.game_capture);
  setText("#gamingCaptureDetail", status.game_capture?.detail || "-");
  setGamingBadge("#gamingGameModeBadge", status.game_mode);
  setText("#gamingGameModeDetail", status.game_mode?.detail || "-");
  setGamingBadge("#gamingFullscreenBadge", status.fullscreen_optimization);
  setText("#gamingFullscreenDetail", status.fullscreen_optimization?.detail || "-");
}

async function refreshGamingOptimizerStatus() {
  try {
    const status = await invoke()("gaming_get_optimizer_status", {
      gamePath: gamingSelectedGamePath || null
    });
    renderGamingOptimizerStatus(status);
    if (gamingActionMessageEl) {
      gamingActionMessageEl.textContent = "竞技模式状态已刷新。";
    }
  } catch (error) {
    if (gamingActionMessageEl) {
      gamingActionMessageEl.textContent = `读取竞技模式状态失败：${error}`;
    }
  }
}

function loadStoredGamingGamePath() {
  try {
    gamingSelectedGamePath = window.localStorage?.getItem("pc-requirements-gaming-game-path") || "";
  } catch {
    gamingSelectedGamePath = "";
  }
  if (gamingSelectedGamePathEl) {
    gamingSelectedGamePathEl.textContent = gamingSelectedGamePath || "未选择游戏 exe";
  }
}

async function setGamingSelectedGamePath(path) {
  gamingSelectedGamePath = path || "";
  try {
    window.localStorage?.setItem("pc-requirements-gaming-game-path", gamingSelectedGamePath);
  } catch {
    // localStorage can be unavailable in restricted contexts.
  }
  if (gamingSelectedGamePathEl) {
    gamingSelectedGamePathEl.textContent = gamingSelectedGamePath || "未选择游戏 exe";
  }
  await refreshGamingOptimizerStatus();
}

async function browseGamingGameExe() {
  if (!hasTauri) {
    await setGamingSelectedGamePath(gamingPreviewExePath);
    return;
  }

  const dialogApi = window.__TAURI__?.dialog;
  if (!dialogApi?.open) {
    if (gamingActionMessageEl) {
      gamingActionMessageEl.textContent = "文件选择插件不可用，请重启应用后再试。";
    }
    return;
  }

  try {
    const selected = await dialogApi.open({
      multiple: false,
      directory: false,
      title: "选择游戏 exe",
      filters: [{ name: "Game executable", extensions: ["exe"] }]
    });
    const path = Array.isArray(selected) ? selected[0] : selected;
    if (typeof path === "string" && path.trim()) {
      await setGamingSelectedGamePath(path);
    }
  } catch (error) {
    if (gamingActionMessageEl) {
      gamingActionMessageEl.textContent = `选择游戏 exe 失败：${error}`;
    }
  }
}

async function runGamingAction(action) {
  const needsGamePath = action.includes("fullscreen_optimization");
  if (needsGamePath && !gamingSelectedGamePath) {
    if (gamingActionMessageEl) {
      gamingActionMessageEl.textContent = "请先选择游戏 exe。";
    }
    return;
  }

  const confirmed = await confirmDialog({
    variant: action === "disable_game_mode" ? "warning" : "info",
    eyebrow: "竞技模式",
    title: "确认应用竞技优化？",
    body: "程序会记录旧值，用于后续一键还原。",
    riskItems: action.includes("hags")
      ? ["HAGS 写入 HKLM，需要管理员权限，并且重启 Windows 后完全生效。"]
      : ["建议先关闭正在运行的游戏，再修改游戏相关兼容性设置。"],
    requireAcknowledge: action.includes("hags") || action.includes("fullscreen"),
    details: needsGamePath ? gamingSelectedGamePath : "",
    confirmText: "确认应用"
  });
  if (!confirmed) {
    return;
  }

  setBusy(true);
  if (gamingActionMessageEl) {
    gamingActionMessageEl.textContent = "正在应用竞技优化...";
  }
  try {
    await waitForNextPaint();
    const result = await invoke()("gaming_apply_optimizer_action", {
      request: {
        action,
        game_path: gamingSelectedGamePath || null
      }
    });
    await refreshGamingOptimizerStatus();
    if (gamingActionMessageEl) {
      const restart = result.requires_restart ? " 需要重启后完全生效。" : "";
      gamingActionMessageEl.textContent = `${result.message}${restart}`;
    }
  } catch (error) {
    if (gamingActionMessageEl) {
      gamingActionMessageEl.textContent = `应用竞技优化失败：${error}`;
    }
  } finally {
    setBusy(false);
  }
}

async function applyGamingCompetitivePreset() {
  const includeFullscreen = Boolean(gamingPresetFullscreen?.checked);
  if (includeFullscreen && !gamingSelectedGamePath) {
    if (gamingActionMessageEl) {
      gamingActionMessageEl.textContent = "要禁用全屏优化，请先选择游戏 exe；也可以取消该勾选后再应用预设。";
    }
    return;
  }

  const confirmed = await confirmDialog({
    variant: "warning",
    eyebrow: "竞技模式",
    title: "应用安全竞技 FPS 预设？",
    body: "预设只包含 HAGS、Game DVR/Game Bar 捕获、所选游戏全屏优化；不包含电源计划和 Game Mode。",
    riskItems: [
      "HAGS 需要管理员权限，并且重启 Windows 后完全生效。",
      "全屏优化只针对你选择的游戏 exe，程序会保留原本已有的兼容性 flags。"
    ],
    requireAcknowledge: true,
    details: gamingSelectedGamePath || "未选择游戏 exe",
    confirmText: "应用预设"
  });
  if (!confirmed) {
    return;
  }

  setBusy(true);
  if (gamingActionMessageEl) {
    gamingActionMessageEl.textContent = "正在应用安全竞技 FPS 预设...";
  }
  try {
    await waitForNextPaint();
    const result = await invoke()("gaming_apply_competitive_preset", {
      request: {
        game_path: gamingSelectedGamePath || null,
        include_hags: Boolean(gamingPresetHags?.checked),
        include_game_capture: Boolean(gamingPresetCapture?.checked),
        include_fullscreen_optimization: includeFullscreen
      }
    });
    await refreshGamingOptimizerStatus();
    if (gamingActionMessageEl) {
      const restart = result.requires_restart ? " 需要重启后完全生效。" : "";
      gamingActionMessageEl.textContent = `${result.message}${restart}`;
    }
  } catch (error) {
    if (gamingActionMessageEl) {
      gamingActionMessageEl.textContent = `应用安全预设失败：${error}`;
    }
  } finally {
    setBusy(false);
  }
}

async function restoreGamingOptimizerChanges() {
  const confirmed = await confirmDialog({
    variant: "danger",
    eyebrow: "竞技模式",
    title: "还原竞技模式记录的所有改动？",
    body: "会按照本程序记录的旧值反向写回注册表。",
    riskItems: ["只还原本程序记录过的改动，不会猜测或覆盖其它工具的未知更改。"],
    requireAcknowledge: true,
    confirmText: "还原改动"
  });
  if (!confirmed) {
    return;
  }

  setBusy(true);
  if (gamingActionMessageEl) {
    gamingActionMessageEl.textContent = "正在还原竞技模式改动...";
  }
  try {
    await waitForNextPaint();
    const result = await invoke()("gaming_restore_optimizer_changes");
    await refreshGamingOptimizerStatus();
    if (gamingActionMessageEl) {
      const restart = result.requires_restart ? " 需要重启后完全生效。" : "";
      gamingActionMessageEl.textContent = `${result.message}${restart}`;
    }
  } catch (error) {
    if (gamingActionMessageEl) {
      gamingActionMessageEl.textContent = `还原竞技模式改动失败：${error}`;
    }
  } finally {
    setBusy(false);
  }
}

async function initGamingOptimizerPanel() {
  if (gamingOptimizerStatus) {
    return;
  }
  if (gamingOptimizerInitPromise) {
    await gamingOptimizerInitPromise;
    return;
  }

  loadStoredGamingGamePath();
  gamingOptimizerInitPromise = refreshGamingOptimizerStatus().finally(() => {
    gamingOptimizerInitPromise = null;
  });
  await gamingOptimizerInitPromise;
}

function monitorIdentityLabel(monitor) {
  const identity = monitor?.current || monitor?.original || {};
  const name = identity.monitor_name || monitor?.hardware_id || "显示器";
  const hardware = identity.windows_hardware_id || monitor?.hardware_id || "-";
  return `${name} (${hardware})${monitor?.override_present ? "（已覆盖）" : ""}`;
}

function selectedMonitorIdentity() {
  const selectedId = monitorIdentitySelect?.value || "";
  return (monitorIdentityStatus?.monitors || []).find(
    (monitor) => monitor.device_instance_id === selectedId
  );
}

function fillMonitorIdentityInputs(monitor = selectedMonitorIdentity()) {
  const identity = monitor?.current || monitor?.original;
  if (!identity) {
    return;
  }
  if (monitorManufacturerInput) {
    monitorManufacturerInput.value = identity.manufacturer_id || "";
  }
  if (monitorProductInput) {
    monitorProductInput.value = identity.product_code_hex || "";
  }
  if (monitorNumericSerialInput) {
    monitorNumericSerialInput.value =
      identity.numeric_serial === null || identity.numeric_serial === undefined
        ? ""
        : String(identity.numeric_serial);
  }
  if (monitorSerialInput) {
    monitorSerialInput.value = identity.serial_number || "";
  }
  if (monitorNameInput) {
    monitorNameInput.value = identity.monitor_name || "";
  }
}

function appendMonitorIdentityCell(parent, label, value) {
  const cell = document.createElement("div");
  const labelEl = document.createElement("span");
  labelEl.textContent = label;
  const valueEl = document.createElement("strong");
  valueEl.textContent = value || "-";
  cell.append(labelEl, valueEl);
  parent.append(cell);
}

function renderMonitorIdentityCurrent(monitor = selectedMonitorIdentity()) {
  if (!monitorIdentityCurrentEl) {
    return;
  }
  monitorIdentityCurrentEl.innerHTML = "";
  if (!monitor) {
    appendMonitorIdentityCell(monitorIdentityCurrentEl, "状态", "未选择显示器");
    return;
  }
  const identity = monitor.current || monitor.original || {};
  const reported = monitor.windows_reported || {};
  appendMonitorIdentityCell(monitorIdentityCurrentEl, "覆盖身份", identity.windows_hardware_id || monitor.hardware_id);
  appendMonitorIdentityCell(monitorIdentityCurrentEl, "Windows 报告", reported.hardware_id || "-");
  appendMonitorIdentityCell(monitorIdentityCurrentEl, "显示器名称", identity.monitor_name);
  appendMonitorIdentityCell(monitorIdentityCurrentEl, "序列号", identity.serial_number || reported.serial_number);
  appendMonitorIdentityCell(monitorIdentityCurrentEl, "覆盖状态", monitor.override_present ? "已启用" : "未覆盖");
}

function renderMonitorIdentityLog(changes = monitorIdentityStatus?.changes || []) {
  if (!monitorIdentityLogEl) {
    return;
  }
  monitorIdentityLogEl.innerHTML = "";
  if (!Array.isArray(changes) || changes.length === 0) {
    const empty = document.createElement("article");
    const title = document.createElement("strong");
    title.textContent = "没有覆盖记录";
    const body = document.createElement("p");
    body.textContent = "应用修改后，这里会显示可还原记录。";
    empty.append(title, body);
    monitorIdentityLogEl.append(empty);
    return;
  }
  for (const change of changes.slice(0, 5)) {
    const item = document.createElement("article");
    const title = document.createElement("strong");
    const mode =
      change.apply_mode === "inf_driver"
        ? "INF"
        : change.apply_mode === "reenumerate_device"
          ? "重枚举"
          : "注册表";
    title.textContent = `${change.status || "unknown"} · ${mode} · ${change.target_hardware_id || "-"}`;
    const body = document.createElement("p");
    body.textContent = `${change.monitor_device_instance_id || "-"} · ${change.applied_at || "-"}`;
    const path = document.createElement("p");
    path.textContent = change.published_driver_inf || change.generated_inf_path || change.output || "";
    item.append(title, body, path);
    monitorIdentityLogEl.append(item);
  }
}

function startMonitorIdentityCountdown(pending) {
  if (monitorIdentityCountdownTimer) {
    window.clearInterval(monitorIdentityCountdownTimer);
    monitorIdentityCountdownTimer = null;
  }
  const update = () => {
    const expiresAt = Date.parse(pending?.expires_at || "");
    const remaining = Number.isFinite(expiresAt)
      ? Math.max(0, Math.ceil((expiresAt - Date.now()) / 1000))
      : Number(pending?.seconds_remaining || 0);
    if (monitorIdentityPendingStateEl) {
      monitorIdentityPendingStateEl.textContent = pending
        ? `待确认：${remaining}s`
        : "待确认：-";
    }
    if (monitorIdentityConfirmButton) {
      monitorIdentityConfirmButton.disabled = !pending || remaining <= 0;
    }
    if (remaining <= 0 && monitorIdentityCountdownTimer) {
      window.clearInterval(monitorIdentityCountdownTimer);
      monitorIdentityCountdownTimer = null;
      window.setTimeout(() => {
        void refreshMonitorIdentityStatus();
      }, 1200);
    }
  };
  update();
  if (pending) {
    monitorIdentityCountdownTimer = window.setInterval(update, 1000);
  }
}

function renderMonitorIdentityStatus(status = monitorIdentityStatus) {
  monitorIdentityStatus = status;
  if (!status) {
    return;
  }
  if (monitorIdentityAdminStateEl) {
    monitorIdentityAdminStateEl.textContent = `管理员：${status.is_administrator ? "是" : "否"}`;
  }
  if (monitorIdentityActiveStateEl) {
    monitorIdentityActiveStateEl.textContent = `活动显示器：${status.active_monitor_count || 0}`;
  }

  const monitors = Array.isArray(status.monitors) ? status.monitors : [];
  const previousSelection = monitorIdentitySelect?.value || "";
  if (monitorIdentitySelect) {
    monitorIdentitySelect.innerHTML = "";
    for (const monitor of monitors) {
      const option = document.createElement("option");
      option.value = monitor.device_instance_id;
      option.textContent = monitorIdentityLabel(monitor);
      monitorIdentitySelect.append(option);
    }
    const nextSelection =
      monitors.find((monitor) => monitor.device_instance_id === previousSelection)
        ?.device_instance_id ||
      monitors.find((monitor) => monitor.active)?.device_instance_id ||
      monitors[0]?.device_instance_id ||
      "";
    monitorIdentitySelect.value = nextSelection;
  }

  const selected = selectedMonitorIdentity();
  renderMonitorIdentityCurrent(selected);
  fillMonitorIdentityInputs(selected);
  renderMonitorIdentityLog(status.changes);
  startMonitorIdentityCountdown(status.pending_confirmation);
}

async function refreshMonitorIdentityStatus() {
  try {
    const status = await invoke()("monitor_identity_get_status");
    renderMonitorIdentityStatus(status);
    if (monitorIdentityMessageEl) {
      monitorIdentityMessageEl.textContent = "显示器身份状态已刷新。";
    }
  } catch (error) {
    if (monitorIdentityMessageEl) {
      monitorIdentityMessageEl.textContent = `读取显示器身份失败：${error}`;
    }
  }
}

async function initMonitorIdentityPanel() {
  if (monitorIdentityStatus) {
    return;
  }
  if (monitorIdentityInitPromise) {
    await monitorIdentityInitPromise;
    return;
  }
  monitorIdentityInitPromise = refreshMonitorIdentityStatus().finally(() => {
    monitorIdentityInitPromise = null;
  });
  await monitorIdentityInitPromise;
}

function randomUint32() {
  const cryptoApi = window.crypto || window.msCrypto;
  if (cryptoApi?.getRandomValues) {
    const value = new Uint32Array(1);
    cryptoApi.getRandomValues(value);
    return value[0] >>> 0;
  }
  return Math.floor(Math.random() * 0x100000000) >>> 0;
}

function randomInt(min, max) {
  return min + (randomUint32() % (max - min + 1));
}

function randomLetters(length) {
  return Array.from({ length }, () => String.fromCharCode(65 + randomInt(0, 25))).join("");
}

function randomHex(length) {
  let value = "";
  while (value.length < length) {
    value += randomUint32().toString(16).toUpperCase().padStart(8, "0");
  }
  return value.slice(0, length);
}

function randomProductCode() {
  return randomInt(1, 0xffff).toString(16).toUpperCase().padStart(4, "0");
}

function randomSerialText() {
  return `SN${randomHex(10)}`.slice(0, 13);
}

function randomMonitorName() {
  return `DSP-${randomHex(8)}`.slice(0, 13);
}

function randomizeMonitorIdentityFields() {
  if (monitorManufacturerInput) {
    monitorManufacturerInput.value = randomLetters(3);
  }
  if (monitorProductInput) {
    monitorProductInput.value = randomProductCode();
  }
  if (monitorNumericSerialInput) {
    monitorNumericSerialInput.value = String(randomInt(1, 0xffffffff));
  }
  if (monitorSerialInput) {
    monitorSerialInput.value = randomSerialText();
  }
  if (monitorNameInput) {
    monitorNameInput.value = randomMonitorName();
  }
  if (monitorIdentityMessageEl) {
    monitorIdentityMessageEl.textContent = "已随机生成显示器身份字段；尚未写入系统。";
  }
}

function readMonitorIdentityRequest() {
  const monitor = selectedMonitorIdentity();
  if (!monitor) {
    throw new Error("请选择一个显示器。");
  }
  const manufacturerId = String(monitorManufacturerInput?.value || "").trim().toUpperCase();
  const productCode = String(monitorProductInput?.value || "").trim().toUpperCase();
  const numericText = String(monitorNumericSerialInput?.value || "").trim();
  const numericSerial = numericText === "" ? null : Number.parseInt(numericText, 10);
  if (!/^[A-Z]{3}$/.test(manufacturerId)) {
    throw new Error("Manufacturer ID 必须是 3 个英文字母。");
  }
  if (!/^[0-9A-F]{1,4}$/.test(productCode)) {
    throw new Error("Product Code 必须是 1-4 位十六进制。");
  }
  if (numericSerial !== null && (!Number.isFinite(numericSerial) || numericSerial < 0 || numericSerial > 4294967295)) {
    throw new Error("Numeric serial 必须在 0 到 4294967295 之间。");
  }
  return {
    monitor_device_instance_id: monitor.device_instance_id,
    manufacturer_id: manufacturerId,
    product_code_hex: productCode.padStart(4, "0"),
    numeric_serial: numericSerial,
    serial_number: String(monitorSerialInput?.value || "").trim() || null,
    monitor_name: String(monitorNameInput?.value || "").trim() || null,
    rollback_timeout_secs: 30
  };
}

async function applyMonitorIdentityOverride(mode = "registry") {
  let request;
  try {
    request = readMonitorIdentityRequest();
  } catch (error) {
    if (monitorIdentityMessageEl) {
      monitorIdentityMessageEl.textContent = `输入无效：${error.message || error}`;
    }
    return;
  }

  const isInfInstall = mode === "inf";
  const confirmed = await confirmDialog({
    variant: "danger",
    eyebrow: isInfInstall ? "显示器身份" : "高级工具",
    title: isInfInstall ? "确认应用显示器身份修改？" : "确认仅写入注册表覆盖？",
    body: isInfInstall
      ? "程序会使用推荐流程应用新身份：生成签名 INF、写入 Windows 的 EDID 覆盖并重扫设备；不写入显示器 EEPROM。应用后必须在 30 秒内点击保留更改，否则会自动恢复。"
      : "程序只写入 Windows 的 EDID_OVERRIDE，不安装显示器 INF；不写入显示器 EEPROM。应用后必须在 30 秒内点击保留更改，否则会自动恢复。",
    riskItems: isInfInstall
      ? [
          "会调用 pnputil /add-driver /install 安装本次生成的 monitor INF，并记录发布出来的 oem*.inf 以便回滚。",
          "显示器可能短暂闪屏或黑屏；如果无法确认，30 秒后会恢复上一个 EDID_OVERRIDE 并卸载本次发布的驱动包。",
          "Windows 11 驱动签名策略可能拒绝未签名 INF；这种情况下程序会恢复已写入的覆盖并报告 pnputil 输出。",
          "直接绕过 Windows 读取显示器 EEPROM 的工具仍可能看到原始身份。"
        ]
      : [
          "厂商 ID（Manufacturer）和产品码（Product Code）会改变 Windows 读取到的显示器身份。",
          "如果显示异常且无法确认，30 秒后会自动恢复上一个 EDID_OVERRIDE 状态。",
          "直接绕过 Windows 读取显示器 EEPROM 的工具仍可能看到原始身份。"
        ],
    requireAcknowledge: true,
    details: request.monitor_device_instance_id,
    confirmText: isInfInstall ? "应用并启动保护" : "仅写注册表"
  });
  if (!confirmed) {
    return;
  }

  setBusy(true);
  if (monitorIdentityMessageEl) {
    monitorIdentityMessageEl.textContent = isInfInstall
      ? "正在应用显示器身份修改..."
      : "正在写入注册表覆盖...";
  }
  try {
    await waitForNextPaint();
    const command = isInfInstall ? "monitor_identity_install_inf_override" : "monitor_identity_apply_override";
    const result = await invoke()(command, { request });
    await refreshMonitorIdentityStatus();
    if (monitorIdentityMessageEl) {
      monitorIdentityMessageEl.textContent = result.message || "已应用，等待 30 秒确认。";
    }
  } catch (error) {
    if (monitorIdentityMessageEl) {
      monitorIdentityMessageEl.textContent = `应用显示器身份失败：${error}`;
    }
  } finally {
    setBusy(false);
  }
}

function installMonitorIdentityInfOverride() {
  void applyMonitorIdentityOverride("inf");
}

async function reenumerateMonitorIdentityDevice() {
  const monitor = selectedMonitorIdentity();
  if (!monitor) {
    if (monitorIdentityMessageEl) {
      monitorIdentityMessageEl.textContent = "请选择一个显示器。";
    }
    return;
  }

  const confirmed = await confirmDialog({
    variant: "danger",
    eyebrow: "高级工具",
    title: "确认强制重枚举显示器？",
    body: "程序会执行 pnputil /remove-device 和 /scan-devices，让 Windows 重新枚举当前显示器。操作后必须在 30 秒内点击保留更改，否则后台保护进程会移除本程序覆盖并回到物理 EDID。",
    riskItems: [
      "显示器可能短暂闪屏、黑屏或重新排列。",
      "不会手动删除整棵 Enum\\DISPLAY 注册表项，只使用 pnputil 官方设备移除和扫描命令。",
      "如果 30 秒内无法确认，会删除本程序 EDID_OVERRIDE，并卸载本程序记录的 oem*.inf 驱动包。"
    ],
    requireAcknowledge: true,
    details: monitor.device_instance_id,
    confirmText: "重枚举并启动回滚计时"
  });
  if (!confirmed) {
    return;
  }

  setBusy(true);
  if (monitorIdentityMessageEl) {
    monitorIdentityMessageEl.textContent = "正在强制重枚举显示器...";
  }
  try {
    await waitForNextPaint();
    const result = await invoke()("monitor_identity_reenumerate_device", {
      request: {
        monitor_device_instance_id: monitor.device_instance_id,
        rollback_timeout_secs: 30
      }
    });
    await refreshMonitorIdentityStatus();
    if (monitorIdentityMessageEl) {
      monitorIdentityMessageEl.textContent = result.message || "已强制重枚举，等待 30 秒确认。";
    }
  } catch (error) {
    if (monitorIdentityMessageEl) {
      monitorIdentityMessageEl.textContent = `强制重枚举失败：${error}`;
    }
  } finally {
    setBusy(false);
  }
}

async function confirmMonitorIdentityOverride() {
  const pending = monitorIdentityStatus?.pending_confirmation;
  if (!pending) {
    return;
  }
  setBusy(true);
  try {
    await waitForNextPaint();
    const result = await invoke()("monitor_identity_confirm_override", { token: pending.token });
    await refreshMonitorIdentityStatus();
    if (monitorIdentityMessageEl) {
      monitorIdentityMessageEl.textContent = result.message || "已保留显示器身份覆盖。";
    }
  } catch (error) {
    if (monitorIdentityMessageEl) {
      monitorIdentityMessageEl.textContent = `确认失败：${error}`;
    }
  } finally {
    setBusy(false);
  }
}

async function restoreMonitorIdentityOverride() {
  const pending = monitorIdentityStatus?.pending_confirmation;
  const confirmed = await confirmDialog({
    variant: "danger",
    eyebrow: "显示器身份",
    title: "还原显示器身份修改？",
    body: "程序会按变更记录恢复上一次状态，并重扫显示设备。",
    riskItems: ["只还原本程序记录的覆盖；其它工具未记录的修改不会被猜测覆盖。"],
    requireAcknowledge: true,
    confirmText: "还原"
  });
  if (!confirmed) {
    return;
  }

  setBusy(true);
  if (monitorIdentityMessageEl) {
    monitorIdentityMessageEl.textContent = "正在还原显示器身份修改...";
  }
  try {
    await waitForNextPaint();
    const result = await invoke()("monitor_identity_restore_change", {
      changeId: pending?.change_id || null
    });
    await refreshMonitorIdentityStatus();
    if (monitorIdentityMessageEl) {
      monitorIdentityMessageEl.textContent = result.message || "已还原显示器身份覆盖。";
    }
  } catch (error) {
    if (monitorIdentityMessageEl) {
      monitorIdentityMessageEl.textContent = `还原失败：${error}`;
    }
  } finally {
    setBusy(false);
  }
}

function applyTheme(theme, persist = false) {
  const normalized = theme === "light" ? "light" : "dark";
  document.documentElement.dataset.theme = normalized;
  if (themeToggleButton) {
    const label = normalized === "light" ? "浅色" : "深色";
    if (themeToggleLabel) {
      themeToggleLabel.textContent = label;
    } else {
      themeToggleButton.textContent = label;
    }
    setIconHref(themeToggleIconUse, normalized === "light" ? "ph-sun" : "ph-moon");
    themeToggleButton.setAttribute("aria-pressed", String(normalized === "dark"));
    themeToggleButton.title = normalized === "light" ? "切换到深色主题" : "切换到浅色主题";
  }
  if (persist) {
    try {
      window.localStorage?.setItem(THEME_STORAGE_KEY, normalized);
    } catch {
      // localStorage can be unavailable in restricted WebView contexts.
    }
  }
}

function toggleTheme() {
  const next = document.documentElement.dataset.theme === "light" ? "dark" : "light";
  applyTheme(next, true);
}

function closeDialog(value) {
  if (!dialogBackdrop || !dialogResolver) {
    return;
  }
  dialogBackdrop.classList.remove("open", "warning", "danger", "info");
  dialogBackdrop.setAttribute("aria-hidden", "true");
  if (dialogConfirm) {
    dialogConfirm.disabled = false;
  }
  const resolve = dialogResolver;
  dialogResolver = null;
  resolve(value);
}

function showDialog(options = {}) {
  if (!dialogBackdrop) {
    return Promise.resolve(true);
  }

  const {
    variant = "info",
    eyebrow = "确认操作",
    title = "确认操作",
    body = "",
    details = "",
    riskItems = [],
    requireAcknowledge = false,
    confirmText = "确认",
    cancelText = "取消",
    iconSymbol = "",
    linkAction = null,
    showCancel = true
  } = options;

  dialogBackdrop.className = `dialog-backdrop open ${variant}`;
  dialogBackdrop.setAttribute("aria-hidden", "false");
  if (dialogAccent) {
    dialogAccent.className = "dialog-accent";
  }
  if (dialogEyebrow) {
    dialogEyebrow.textContent = eyebrow;
  }
  if (dialogTitle) {
    dialogTitle.textContent = title;
  }
  if (dialogBody) {
    dialogBody.textContent = body;
  }
  if (dialogDetails) {
    dialogDetails.textContent = details;
  }
  if (dialogRiskList) {
    dialogRiskList.innerHTML = "";
    for (const item of riskItems) {
      const li = document.createElement("li");
      li.textContent = item;
      dialogRiskList.append(li);
    }
  }
  if (dialogIcon) {
    dialogIcon.dataset.variant = variant;
    if (dialogIconUse) {
      setIconHref(dialogIconUse, iconSymbol || (variant === "info" ? "ph-shield-check" : "ph-warning-circle"));
    } else {
      dialogIcon.textContent = variant === "danger" ? "!" : variant === "warning" ? "!" : "i";
    }
  }
  if (dialogCancel) {
    dialogCancel.textContent = cancelText;
    dialogCancel.hidden = !showCancel;
  }
  if (dialogConfirm) {
    dialogConfirm.textContent = confirmText;
    dialogConfirm.disabled = Boolean(requireAcknowledge);
  }
  if (dialogAcknowledgeWrap && dialogAcknowledge) {
    dialogAcknowledge.checked = false;
    dialogAcknowledgeWrap.hidden = !requireAcknowledge;
  }
  if (dialogLinkAction) {
    dialogLinkAction.hidden = !linkAction;
    dialogLinkAction.dataset.action = linkAction?.action || "";
    if (dialogLinkActionLabel) {
      dialogLinkActionLabel.textContent = linkAction?.label || "GitHub";
    }
    if (dialogLinkActionIconUse) {
      setIconHref(dialogLinkActionIconUse, linkAction?.iconSymbol || "ph-github-logo");
    }
  }

  return new Promise((resolve) => {
    dialogResolver = resolve;
    window.setTimeout(() => dialogConfirm?.focus(), 0);
  });
}

function normalizeConfirmText(text) {
  return String(text || "")
    .replace(/继续？$/, "")
    .replace(/确认继续？$/, "")
    .trim();
}

function confirmDialog(options) {
  return showDialog({
    variant: "warning",
    confirmText: "确认执行",
    ...options,
    showCancel: true
  });
}

function notifyDialog(options) {
  return showDialog({
    variant: "info",
    confirmText: "知道了",
    ...options,
    showCancel: false
  });
}

function showAboutDialog() {
  return notifyDialog({
    eyebrow: "关于",
    title: "知机",
    body: `作者：LingCore\n开源协议：MIT\n${PROJECT_REPOSITORY_URL}\n用于查看和调整 Windows 关键配置、虚拟内存、蓝屏收集和硬件信息。`,
    confirmText: "知道了",
    iconSymbol: "ph-info",
    linkAction: {
      action: "repository",
      label: "GitHub",
      iconSymbol: "ph-github-logo"
    }
  });
}

async function openProjectRepository() {
  try {
    await invoke()("open_project_repository");
  } catch (error) {
    try {
      window.open(PROJECT_REPOSITORY_URL, "_blank", "noopener,noreferrer");
    } catch {
      messageEl.textContent = `打开 GitHub 失败：${error}`;
    }
  }
}

const ACTION_RISK_ITEMS = {
  enable_hyper_v: [
    "会修改 Windows 可选功能，通常需要管理员权限和重启。",
    "启用后部分模拟器、虚拟机或安全功能的行为可能发生变化。"
  ],
  disable_hyper_v: [
    "依赖 Hyper-V（微软虚拟机监控程序）的虚拟机、WSL2、容器或模拟器可能无法启动。",
    "修改后通常需要重启，当前会话中的虚拟化工作请先保存。"
  ],
  enable_virtual_machine_platform: [
    "会修改 Windows 虚拟化平台组件，通常需要重启。",
    "WSL2、容器和部分虚拟化功能可能在重启后切换运行路径。"
  ],
  disable_virtual_machine_platform: [
    "WSL2、容器和部分虚拟化组件可能无法正常运行。",
    "修改后通常需要重启，建议先关闭相关虚拟机或容器。"
  ],
  enable_windows_hypervisor_platform: [
    "会修改第三方虚拟机和模拟器可能依赖的 Windows 虚拟机监控平台组件。",
    "修改后通常需要重启，正在运行的虚拟化程序请先退出。"
  ],
  disable_windows_hypervisor_platform: [
    "第三方虚拟机、模拟器或调试工具可能受到影响。",
    "修改后通常需要重启，当前虚拟化任务请先保存。"
  ],
  set_hypervisor_auto: [
    "会修改启动配置数据（BCD）里的虚拟机监控程序启动项（hypervisorlaunchtype）。",
    "重启后 Hyper-V 相关服务会随系统启动。"
  ],
  set_hypervisor_off: [
    "会修改启动配置数据（BCD）里的虚拟机监控程序启动项（hypervisorlaunchtype）。",
    "重启后 Hyper-V、WSL2 或依赖虚拟机监控程序（hypervisor）的功能可能无法运行。"
  ],
  restart_windows: [
    "电脑会立即重启，未保存的文件和任务会丢失。",
    "请先关闭正在运行的安装、虚拟机、游戏或磁盘任务。"
  ]
};

function riskItemsForAction(action) {
  return ACTION_RISK_ITEMS[action] || [];
}

function statusClass(status) {
  return String(status || "unknown").toLowerCase();
}

function normalizeSwitchState(value, options = {}) {
  const raw = String(value || "").trim();
  const lower = raw.toLowerCase();

  if (!raw || lower.includes("unknown") || lower.includes("could not")) {
    return {
      key: "unknown",
      label: "未知",
      detail: raw || "未读取到状态"
    };
  }

  if (options.auto && lower === "auto") {
    return {
      key: "auto",
      label: "自动（Auto）",
      detail: raw
    };
  }

  if (lower.includes("enabled") || lower === "on" || lower === "true") {
    return {
      key: "on",
      label: "开启",
      detail: raw
    };
  }

  if (lower.includes("disabled") || lower === "off" || lower === "false") {
    return {
      key: "off",
      label: "关闭",
      detail: raw
    };
  }

  if (lower === "system managed" || lower === "系统托管") {
    return {
      key: "managed",
      label: "系统托管",
      detail: raw
    };
  }

  if (lower.includes("未配置") || lower.includes("no pagefile")) {
    return {
      key: "off",
      label: "未配置",
      detail: raw
    };
  }

  if (lower === "custom" || lower === "自定义") {
    return {
      key: "custom",
      label: "自定义",
      detail: raw
    };
  }

  return {
    key: "unknown",
    label: raw,
    detail: raw
  };
}

function stateFromCheckItem(item) {
  const detected = String(item?.detected || "");
  const details = String(item?.details || "");
  const text = `${detected} ${details}`.toLowerCase();
  const name = String(item?.name || "").toLowerCase();

  if (name.includes("cpu virtualization")) {
    if (text.includes("virtualizationfirmwareenabled=true") || text.includes("hypervisorpresent=true")) {
      return { key: "on", label: "开启", detail: detected };
    }
    if (text.includes("virtualizationfirmwareenabled=false")) {
      return { key: "off", label: "关闭", detail: detected };
    }
  }

  if (name.includes("vt-d") || name.includes("iommu")) {
    if (text.includes("dmaprotectionavailable=true") || text.includes("availablesecurityproperties=1, 3")) {
      return { key: "on", label: "开启", detail: detected };
    }
    if (text.includes("dmaprotectionavailable=false")) {
      return { key: "off", label: "关闭", detail: detected };
    }
  }

  if (name.includes("csm") || name.includes("legacy boot")) {
    if (text.includes("bootmode=uefi")) {
      return { key: "off", label: "UEFI 启动", detail: detected };
    }
    if (text.includes("bootmode=legacy") || text.includes("bootmode=bios")) {
      return { key: "on", label: "兼容启动（CSM/Legacy）", detail: detected };
    }
  }

  if (name.includes("secure boot")) {
    if (text.includes("secureboot=enabled")) {
      return { key: "on", label: "开启", detail: detected };
    }
    if (text.includes("secureboot=disabled")) {
      return { key: "off", label: "关闭", detail: detected };
    }
    if (text.includes("secureboot=notsupported")) {
      return { key: "off", label: "不支持", detail: detected };
    }
  }

  if (name === "tpm") {
    if (text.includes("tpmpresent=false") || text.includes("isenabled_initialvalue=false")) {
      return { key: "off", label: "关闭", detail: detected };
    }
    if (
      text.includes("tpmpresent=true") ||
      text.includes("isenabled_initialvalue=true") ||
      text.includes("win32_tpm present")
    ) {
      return { key: "on", label: "开启", detail: detected };
    }
  }

  if (name.includes("hypervisor launch")) {
    if (text.includes("hypervisorlaunchtype=auto")) {
      return { key: "auto", label: "自动（Auto）", detail: detected };
    }
    if (text.includes("hypervisorlaunchtype=off")) {
      return { key: "off", label: "关闭（Off）", detail: detected };
    }
  }

  return {
    key: "unknown",
    label: "未知",
    detail: [detected, details].filter(Boolean).join(" | ") || item?.required || "-"
  };
}

function buildStateItems(report) {
  const results = Array.isArray(report?.results) ? report.results : [];
  const findResult = (name) => results.find((item) => item.name === name);
  const states = report?.feature_states || {};
  const virtualMemory = report?.virtual_memory || {};
  const vmMode =
    virtualMemory.automatic_managed_pagefile === true
      ? "系统托管"
      : virtualMemory.automatic_managed_pagefile === false
        ? "自定义"
        : "未知";

  return [
    {
      id: "cpu_virtualization",
      name: "CPU 虚拟化（VT-x / SVM）",
      source: "固件（BIOS/UEFI）",
      state: stateFromCheckItem(findResult("CPU virtualization"))
    },
    {
      id: "vtd_iommu",
      name: "设备直通（VT-d / IOMMU）",
      source: "固件（BIOS/UEFI）",
      state: stateFromCheckItem(findResult("VT-d / IOMMU"))
    },
    {
      id: "csm_legacy_boot",
      name: "兼容启动（CSM / Legacy Boot）",
      source: "固件（BIOS/UEFI）",
      state: stateFromCheckItem(findResult("CSM / Legacy Boot"))
    },
    {
      id: "hyper_v",
      name: "Hyper-V（微软虚拟机监控程序）",
      source: "Windows 功能",
      state: normalizeSwitchState(states.hyper_v)
    },
    {
      id: "virtual_machine_platform",
      name: "虚拟机平台（Virtual Machine Platform）",
      source: "Windows 功能",
      state: normalizeSwitchState(states.virtual_machine_platform)
    },
    {
      id: "windows_hypervisor_platform",
      name: "Windows 虚拟机监控平台（Windows Hypervisor Platform）",
      source: "Windows 功能",
      state: normalizeSwitchState(states.windows_hypervisor_platform)
    },
    {
      id: "secure_boot",
      name: "安全启动（Secure Boot）",
      source: "固件（BIOS/UEFI）",
      state: stateFromCheckItem(findResult("Secure Boot"))
    },
    {
      id: "tpm",
      name: "可信平台模块（TPM / Intel PTT / AMD fTPM）",
      source: "固件（BIOS/UEFI）",
      state: stateFromCheckItem(findResult("TPM"))
    },
    {
      id: "hypervisor_launch",
      name: "虚拟机监控程序启动项（hypervisorlaunchtype）",
      source: "启动配置（BCD）",
      state: normalizeSwitchState(states.hypervisor_launch, { auto: true })
    },
    {
      id: "fast_startup",
      name: "快速启动（Windows Fast Startup）",
      source: "快速启动注册表（Hiberboot）",
      state: normalizeSwitchState(states.fast_startup)
    },
    {
      id: "memory_compression",
      name: "内存压缩（Memory Compression）",
      source: "内存管理（Memory Management）",
      state: normalizeSwitchState(states.memory_compression)
    },
    {
      id: "virtual_memory",
      name: "虚拟内存",
      source: virtualMemory.system_drive ? `${virtualMemory.system_drive} 页面文件（pagefile.sys）` : "页面文件（pagefile.sys）",
      state: normalizeSwitchState(vmMode)
    }
  ];
}

function countDisplayStates(items) {
  return items.reduce(
    (counts, item) => {
      if (item.state.key === "on" || item.state.key === "auto" || item.state.key === "managed") {
        counts.on += 1;
      } else if (item.state.key === "off") {
        counts.off += 1;
      } else if (item.state.key === "custom") {
        counts.custom += 1;
      } else {
        counts.unknown += 1;
      }
      return counts;
    },
    { on: 0, off: 0, custom: 0, unknown: 0 }
  );
}

function formatMb(value) {
  const numeric = Number(value);
  if (!Number.isFinite(numeric) || numeric <= 0) {
    return "-";
  }
  if (numeric >= 1024) {
    const gb = numeric / 1024;
    return `${gb >= 10 ? gb.toFixed(0) : gb.toFixed(1)} GB`;
  }
  return `${Math.round(numeric)} MB`;
}

function formatMbWithZero(value) {
  const numeric = Number(value);
  if (!Number.isFinite(numeric)) {
    return "-";
  }
  if (numeric <= 0) {
    return "0 MB";
  }
  return formatMb(numeric);
}

function formatMbRaw(value) {
  const numeric = Number(value);
  return Number.isFinite(numeric) && numeric > 0 ? String(Math.round(numeric)) : "";
}

function hasPositiveMb(value) {
  const numeric = Number(value);
  return Number.isFinite(numeric) && numeric > 0;
}

function formatMbExact(value) {
  const numeric = Number(value);
  if (!Number.isFinite(numeric) || numeric <= 0) {
    return "-";
  }
  return `${Math.round(numeric)} MB (${formatMb(numeric)})`;
}

function formatDateTime(value) {
  if (!value) {
    return "-";
  }
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? String(value) : date.toLocaleString();
}

function roundMb(value) {
  const numeric = Number(value);
  return Number.isFinite(numeric) && numeric > 0 ? Math.round(numeric) : null;
}

function formulaValueText(formula, compact = false) {
  if (!formula) {
    return "-";
  }
  if (formula.mode === "managed") {
    return compact ? "自动" : "Windows 系统托管";
  }
  if (!formula.initial || !formula.maximum) {
    return "-";
  }
  if (compact) {
    return `${formatMb(formula.initial)} / ${formatMb(formula.maximum)}`;
  }
  return `初始 ${formatMbExact(formula.initial)}，最大 ${formatMbExact(formula.maximum)}`;
}

function firstConfiguredPagefile(virtualMemory = {}) {
  const configuredPagefiles = Array.isArray(virtualMemory.configured_pagefiles)
    ? virtualMemory.configured_pagefiles
    : [];
  return (
    configuredPagefiles.find((pagefile) => hasPositiveMb(pagefile.initial_size_mb) || hasPositiveMb(pagefile.maximum_size_mb)) ||
    configuredPagefiles.find((pagefile) => pagefile?.name) ||
    null
  );
}

function firstRuntimePagefile(virtualMemory = {}) {
  const pagefiles = Array.isArray(virtualMemory.pagefiles) ? virtualMemory.pagefiles : [];
  return (
    pagefiles.find((pagefile) => hasPositiveMb(pagefile.allocated_base_size_mb)) ||
    pagefiles.find((pagefile) => pagefile?.name) ||
    null
  );
}

function pagefilePairText(initial, maximum) {
  if (!hasPositiveMb(initial) && !hasPositiveMb(maximum)) {
    return "-";
  }
  return `初始 ${formatMb(initial)} / 最大 ${formatMb(maximum)}`;
}

function currentPagefileSummary(virtualMemory = {}) {
  const configured = firstConfiguredPagefile(virtualMemory);
  const runtime = firstRuntimePagefile(virtualMemory);
  const autoManaged = virtualMemory.automatic_managed_pagefile === true || isSystemManagedPagefile(configured);
  const mode =
    virtualMemory.configured_state ||
    (autoManaged ? "系统托管" : configured ? "自定义" : "未配置");
  const configuredText = autoManaged
    ? "Windows 自动管理"
    : configured
      ? pagefilePairText(configured.initial_size_mb, configured.maximum_size_mb)
      : "未配置页面文件";
  const runtimeText = runtime
    ? `已分配 ${formatMb(runtime.allocated_base_size_mb)} / 使用 ${formatMbWithZero(runtime.current_usage_mb)}`
    : "未检测到运行值";
  const noteParts = [];
  if (configured?.source) {
    noteParts.push(`来源：${configured.source}`);
  }
  if (runtime?.peak_usage_mb !== undefined && runtime?.peak_usage_mb !== null) {
    noteParts.push(`峰值：${formatMbWithZero(runtime.peak_usage_mb)}`);
  }
  if (!configured && virtualMemory.details) {
    noteParts.push(virtualMemory.details);
  }

  return {
    autoManaged,
    configured,
    runtime,
    mode,
    configuredText,
    runtimeText,
    note: noteParts.join("；") || "当前值来自 Windows 配置和运行状态，修改后通常需要重启才会完全生效。"
  };
}

function formulaMatchesCurrent(formula, summary) {
  if (!formula || !summary) {
    return false;
  }
  if (formula.mode === "managed") {
    return summary.autoManaged;
  }
  const configured = summary.configured;
  if (!configured || !hasPositiveMb(configured.initial_size_mb) || !hasPositiveMb(configured.maximum_size_mb)) {
    return false;
  }
  return (
    Math.round(Number(configured.initial_size_mb)) === Math.round(Number(formula.initial)) &&
    Math.round(Number(configured.maximum_size_mb)) === Math.round(Number(formula.maximum))
  );
}

function renderCurrentPagefileSummary(summary) {
  setText("#vmCurrentMode", summary?.mode || "-");
  setText("#vmCurrentConfigured", summary?.configuredText || "-");
  setText("#vmCurrentRuntime", summary?.runtimeText || "-");
  setText("#vmCurrentNote", summary?.note || "-");
}

function buildPagefileFormulaOptions(virtualMemory = {}) {
  const recommendation = virtualMemory.recommendation || {};
  const ram = roundMb(virtualMemory.total_physical_memory_mb);
  const systemMin = roundMb(recommendation.system_managed_min_estimate_mb);
  const formulas = [
    {
      id: "system",
      label: "系统托管",
      mode: "managed",
      initial: null,
      maximum: null,
      description: "推荐 Windows 自动按需调整页面文件，不需要手动填写几十 GB 的固定数值。",
      formula:
        systemMin
          ? `参考下限约 ${formatMbExact(systemMin)}；实际占用由 Windows 按需调整。`
          : "Windows 会按需调整页面文件大小。"
    }
  ];

  if (ram) {
    formulas.push({
      id: "compact",
      label: "省空间固定",
      mode: "custom",
      initial: 4096,
      maximum: 8192,
      description: "网友常见的省空间方案，适合高内存且不跑大模型/重负载；遇到内存错误应改回系统托管。",
      formula: "初始 = 4096 MB；最大 = 8192 MB。"
    });

    const dumpSize = ram + 300;
    formulas.push({
      id: "dump",
      label: "完整转储",
      mode: "custom",
      initial: dumpSize,
      maximum: dumpSize,
      description: "需要完整内存转储或排查蓝屏时使用，占用磁盘更明确。",
      formula: "初始 = 最大 = 物理内存 + 300 MB。"
    });
  }

  return formulas;
}

function defaultFormulaId(virtualMemory = {}) {
  const recommendation = virtualMemory.recommendation || {};
  return hasPositiveMb(recommendation.recommended_initial_mb) &&
    hasPositiveMb(recommendation.recommended_maximum_mb)
    ? "dump"
    : "system";
}

function selectedFormulaFor(virtualMemory = {}) {
  const formulas = buildPagefileFormulaOptions(virtualMemory);
  if (!selectedFormulaId || !formulas.some((formula) => formula.id === selectedFormulaId)) {
    selectedFormulaId = defaultFormulaId(virtualMemory);
  }
  return formulas.find((formula) => formula.id === selectedFormulaId) || formulas[0];
}

function renderFormulaPanel(
  virtualMemory = {},
  formulas = buildPagefileFormulaOptions(virtualMemory),
  currentSummary = currentPagefileSummary(virtualMemory)
) {
  if (!formulaOptionsEl) {
    return;
  }

  const selected = selectedFormulaFor(virtualMemory);
  formulaOptionsEl.innerHTML = "";
  formulaOptionButtons = [];
  setText("#vmFormulaRam", formatMb(virtualMemory.total_physical_memory_mb));

  for (const formula of formulas) {
    const isCurrent = formulaMatchesCurrent(formula, currentSummary);
    const button = document.createElement("button");
    button.className = [
      "formula-option",
      formula.id === selected.id ? "active" : "",
      isCurrent ? "current" : ""
    ]
      .filter(Boolean)
      .join(" ");
    button.type = "button";
    button.dataset.formula = formula.id;

    const title = document.createElement("span");
    title.className = "formula-option-title";
    title.textContent = formula.label;

    const value = document.createElement("span");
    value.className = "formula-option-value";
    value.textContent = formulaValueText(formula, true);

    button.append(title, value);
    if (isCurrent) {
      const badge = document.createElement("span");
      badge.className = "formula-option-badge";
      badge.textContent = "当前";
      button.append(badge);
    }
    button.addEventListener("click", () => {
      selectedFormulaId = formula.id;
      renderVirtualMemory(latestReport?.virtual_memory || {});
      switchView("memory");
    });
    formulaOptionsEl.append(button);
    formulaOptionButtons.push(button);
  }
}

function switchView(view) {
  for (const item of navItems) {
    item.classList.toggle("active", item.dataset.view === view);
  }
  for (const pane of panes) {
    pane.classList.toggle("active", pane.dataset.pane === view);
  }
  const [eyebrow, title] = viewTitles[view] || viewTitles.overview;
  setText("#viewEyebrow", eyebrow);
  setText("#viewTitle", title);
  if (view === "gaming" && !gamingOptimizerStatus) {
    void initGamingOptimizerPanel();
  }
  if (view === "monitorIdentity" && !monitorIdentityStatus) {
    void initMonitorIdentityPanel();
  }
}

function renderStatus(items) {
  const counts = countDisplayStates(items);
  setText("#onCount", String(counts.on));
  setText("#offCount", String(counts.off));
  setText("#customCount", String(counts.custom));
  setText("#unknownCount", String(counts.unknown));
  setText("#healthScore", `${counts.on}/${items.length || 0}`);
}

const CONTROL_ACTIVE_ACTIONS = {
  hyper_v: { on: "enable_hyper_v", off: "disable_hyper_v" },
  virtual_machine_platform: {
    on: "enable_virtual_machine_platform",
    off: "disable_virtual_machine_platform"
  },
  windows_hypervisor_platform: {
    on: "enable_windows_hypervisor_platform",
    off: "disable_windows_hypervisor_platform"
  },
  hypervisor_launch: { auto: "set_hypervisor_auto", off: "set_hypervisor_off" },
  fast_startup: { on: "enable_fast_startup", off: "disable_fast_startup" },
  memory_compression: { on: "enable_memory_compression", off: "disable_memory_compression" }
};

function renderControlStateNotes(items) {
  for (const item of items) {
    const node = document.querySelector(`[data-state-for="${item.id}"]`);
    if (node) {
      node.textContent = `当前：${item.state.label}`;
      node.className = `control-note state-inline state-${item.state.key}`;
    }

    const actions = CONTROL_ACTIVE_ACTIONS[item.id];
    if (!actions) {
      continue;
    }
    for (const [stateKey, action] of Object.entries(actions)) {
      const button = document.querySelector(`[data-action="${action}"]`);
      if (!button) {
        continue;
      }
      const active = item.state.key === stateKey;
      button.className = active ? `action-button is-active state-${stateKey}` : "action-button";
      button.title = active ? "当前状态" : "";
    }
  }
}

function createPagefileValue(value, label) {
  const cell = document.createElement("p");
  cell.className = "pagefile-value";
  const strong = document.createElement("strong");
  strong.textContent = value || "-";
  cell.append(strong, document.createTextNode(label));
  return cell;
}

function appendPagefileSection(card, labelText, cells) {
  const section = document.createElement("div");
  section.className = "pagefile-section";
  const label = document.createElement("p");
  label.className = "pagefile-section-label";
  label.textContent = labelText;
  const values = document.createElement("div");
  values.className = "pagefile-values";
  values.append(...cells);
  section.append(label, values);
  card.append(section);
}

function buildPagefileCard({ name, config, usage }) {
  const card = document.createElement("article");
  card.className = "pagefile-card";

  const head = document.createElement("div");
  head.className = "pagefile-card-head";
  const title = document.createElement("p");
  title.className = "pagefile-name";
  title.textContent = name;
  head.append(title);

  let flagText = "";
  if (config && !usage) {
    flagText = "新设置重启后生效";
  } else if (!config && usage) {
    flagText = "设置已删除，重启后停用";
  } else if (usage?.temp_page_file) {
    flagText = "临时页面文件";
  }
  if (flagText) {
    const flag = document.createElement("span");
    flag.className = "pagefile-flag";
    flag.textContent = flagText;
    head.append(flag);
  }
  card.append(head);

  if (!config) {
    appendPagefileSection(card, "设定值", [createPagefileValue("未配置", "注册表无此项")]);
  } else if (isSystemManagedPagefile(config)) {
    appendPagefileSection(card, "设定值", [createPagefileValue("系统托管", "大小由 Windows 决定")]);
  } else {
    appendPagefileSection(card, "设定值", [
      createPagefileValue(formatMb(config.initial_size_mb), "初始"),
      createPagefileValue(formatMb(config.maximum_size_mb), "最大")
    ]);
  }

  if (!usage) {
    appendPagefileSection(card, "运行中", [createPagefileValue("未运行", "重启后按设定值生效")]);
  } else {
    appendPagefileSection(card, "运行中", [
      createPagefileValue(formatMb(usage.allocated_base_size_mb), "已分配"),
      createPagefileValue(formatMbWithZero(usage.current_usage_mb), "使用中"),
      createPagefileValue(formatMbWithZero(usage.peak_usage_mb), "峰值")
    ]);
  }

  return card;
}

function isSystemManagedPagefile(pagefile) {
  const initial = Number(pagefile?.initial_size_mb);
  const maximum = Number(pagefile?.maximum_size_mb);
  return Number.isFinite(initial) && Number.isFinite(maximum) && initial === 0 && maximum === 0;
}

function renderVirtualMemory(virtualMemory = {}) {
  const pagefiles = Array.isArray(virtualMemory.pagefiles) ? virtualMemory.pagefiles : [];
  const configuredPagefiles = Array.isArray(virtualMemory.configured_pagefiles)
    ? virtualMemory.configured_pagefiles
    : [];
  const mode =
    virtualMemory.configured_state ||
    (virtualMemory.automatic_managed_pagefile === true
      ? "系统托管"
      : virtualMemory.automatic_managed_pagefile === false
        ? "未配置页面文件"
        : "未知");
  const modeState = normalizeSwitchState(mode);
  const currentSummary = currentPagefileSummary(virtualMemory);

  setText("#vmRam", formatMb(virtualMemory.total_physical_memory_mb));
  setStateText("#vmMode", modeState);
  setText(
    "#vmFree",
    `${virtualMemory.system_drive || "C:"} ${formatMb(virtualMemory.system_drive_free_mb)} 可用`
  );
  renderCurrentPagefileSummary(currentSummary);

  const formulas = buildPagefileFormulaOptions(virtualMemory);
  const selectedFormula = selectedFormulaFor(virtualMemory);
  const hasCustomRecommendation = selectedFormula.mode === "custom";

  if (pagefileInitialInput && pagefileMaximumInput) {
    if (hasCustomRecommendation) {
      pagefileInitialInput.value = formatMbRaw(selectedFormula.initial);
      pagefileMaximumInput.value = formatMbRaw(selectedFormula.maximum);
      pagefileInitialInput.placeholder = "公式初始大小";
      pagefileMaximumInput.placeholder = "公式最大大小";
      pagefileInitialInput.disabled = false;
      pagefileMaximumInput.disabled = false;
    } else {
      pagefileInitialInput.value = "";
      pagefileMaximumInput.value = "";
      pagefileInitialInput.placeholder = "无需填写";
      pagefileMaximumInput.placeholder = "无需填写";
      pagefileInitialInput.disabled = true;
      pagefileMaximumInput.disabled = true;
    }
  }

  if (applyCustomPagefileButton) {
    applyCustomPagefileButton.disabled = false;
    applyCustomPagefileButton.textContent = hasCustomRecommendation ? "应用输入值" : "启用系统托管";
    applyCustomPagefileButton.title = hasCustomRecommendation
      ? "应用当前两个输入框里的数值"
      : "系统托管方案没有数值，点击后切换为 Windows 自动管理";
  }
  renderFormulaPanel(virtualMemory, formulas, currentSummary);

  const list = document.querySelector("#pagefileList");
  list.innerHTML = "";

  const keyOf = (value) => String(value || "pagefile.sys").trim().toLowerCase();
  const cards = new Map();
  for (const config of configuredPagefiles) {
    cards.set(keyOf(config.name), { name: config.name || "pagefile.sys", config, usage: null });
  }
  for (const usage of pagefiles) {
    const key = keyOf(usage.name);
    if (cards.has(key)) {
      cards.get(key).usage = usage;
    } else {
      cards.set(key, { name: usage.name || "pagefile.sys", config: null, usage });
    }
  }

  if (cards.size === 0) {
    const empty = document.createElement("article");
    empty.className = "pagefile-card";
    const title = document.createElement("p");
    title.className = "pagefile-name";
    title.textContent = "未配置页面文件";
    const note = document.createElement("p");
    note.className = "pagefile-subtext";
    note.textContent =
      virtualMemory.details || "注册表没有有效 PagingFiles 项；如果刚修改过设置，重启后再检查。";
    empty.append(title, note);
    list.append(empty);
    return;
  }

  for (const entry of [...cards.values()].slice(0, 3)) {
    list.append(buildPagefileCard(entry));
  }
}

const STATE_GROUPS = [
  { title: "固件设置（BIOS / UEFI）", note: "需要进固件设置修改", match: (item) => item.source === "固件（BIOS/UEFI）" },
  { title: "Windows 功能", note: "可在单项控制里开关", match: (item) => item.source === "Windows 功能" },
  { title: "启动与内存", note: "启动配置（BCD）/ 快速启动（Hiberboot）/ 内存管理 / 页面文件", match: () => true }
];

function renderStateList(items) {
  const list = document.querySelector("#stateList");
  list.innerHTML = "";

  let remaining = [...items];
  for (const group of STATE_GROUPS) {
    const groupItems = remaining.filter((item) => group.match(item));
    remaining = remaining.filter((item) => !group.match(item));
    if (groupItems.length === 0) {
      continue;
    }

    const section = document.createElement("section");
    section.className = "state-group";

    const head = document.createElement("div");
    head.className = "state-group-head";
    const title = document.createElement("p");
    title.className = "state-group-title";
    title.textContent = group.title;
    const note = document.createElement("p");
    note.className = "state-group-note";
    note.textContent = group.note;
    head.append(title, note);
    section.append(head);

    for (const item of groupItems) {
      const row = document.createElement("article");
      row.className = `requirement-row state-card state-${item.state.key}`;

      const pill = document.createElement("span");
      pill.className = `status-pill state-${item.state.key}`;
      pill.textContent = item.state.label;

      const content = document.createElement("div");
      content.className = "requirement-content";
      const title = document.createElement("p");
      title.className = "requirement-title";
      title.textContent = item.name;
      const value = document.createElement("p");
      value.className = "requirement-value";
      value.textContent = item.source;
      content.append(title, value);

      row.append(pill, content);
      section.append(row);
    }

    list.append(section);
  }
}

function renderBlueScreen(info = {}) {
  const modeState = info.crash_dump_enabled === 3
    ? { key: "on", label: "小内存转储", detail: info.crash_dump_label }
    : info.crash_dump_enabled === 0
      ? { key: "off", label: "关闭", detail: info.crash_dump_label }
      : { key: "unknown", label: info.crash_dump_label || "未知", detail: info.details || "" };
  const pathState = info.minidump_dir_configured && info.minidump_dir_exists
    ? { key: "on", label: "已就绪" }
    : info.minidump_dir_configured
      ? { key: "unknown", label: "待创建" }
      : { key: "off", label: "需配置" };
  const toolState = info.tool_available
    ? { key: "on", label: "可用" }
    : { key: "off", label: "未找到" };

  setStateText("#dumpMode", modeState);
  setStateText("#dumpPathState", pathState);
  setStateText("#bsodToolState", toolState);
  setText("#dumpPath", info.minidump_dir || "-");
  setText("#dumpCount", `${info.dump_count || 0} 个文件`);
  setText("#bsodDetails", info.details || "开启后，下次蓝屏会生成转储文件（.dmp）。");

  const list = document.querySelector("#dumpList");
  if (!list) {
    return;
  }
  list.innerHTML = "";
  const dumps = Array.isArray(info.recent_dumps) ? info.recent_dumps : [];
  if (dumps.length === 0) {
    const empty = document.createElement("article");
    empty.className = "dump-row empty";
    const title = document.createElement("p");
    title.className = "dump-title";
    title.textContent = "还没有检测到小内存转储（Minidump）文件";
    const note = document.createElement("p");
    note.className = "dump-subtext";
    note.textContent = info.collection_ready
      ? "配置已就绪；下次蓝屏后这里会出现转储文件（.dmp）。"
      : "先点击“开启转储收集（DMP）”，确保下次蓝屏会留下分析文件。";
    empty.append(title, note);
    list.append(empty);
    return;
  }

  for (const dump of dumps) {
    const row = document.createElement("article");
    row.className = "dump-row";
    const main = document.createElement("div");
    const title = document.createElement("p");
    title.className = "dump-title";
    title.textContent = dump.name || "memory.dmp";
    const path = document.createElement("p");
    path.className = "dump-subtext";
    path.textContent = dump.path || "-";
    main.append(title, path);

    const meta = document.createElement("p");
    meta.className = "dump-meta";
    const size = document.createElement("strong");
    size.textContent = `${dump.size_kb || 0} KB`;
    meta.append(size, document.createTextNode(formatDateTime(dump.modified)));
    row.append(main, meta);
    list.append(row);
  }
}

function renderChecks(items) {
  checksListEl.innerHTML = "";

  for (const item of items) {
    const row = document.createElement("article");
    row.className = `check-row state-${item.state.key}`;

    const name = document.createElement("div");
    const title = document.createElement("p");
    title.className = "check-title";
    title.textContent = item.name;
    const source = document.createElement("p");
    source.className = "check-subtext";
    source.textContent = item.source;
    name.append(title, source);

    const status = document.createElement("span");
    status.className = `status-pill state-${item.state.key}`;
    status.textContent = item.state.label;

    const detected = document.createElement("p");
    detected.className = "check-detected";
    detected.textContent = item.state.detail || "-";

    row.append(name, status, detected);
    checksListEl.append(row);
  }
}

function renderReport(report) {
  latestReport = report;
  const stateItems = buildStateItems(report);
  renderStatus(stateItems);
  setText("#adminStatus", `管理员：${report.is_administrator ? "是（Yes）" : "否（No）"}`);
  setText("#boardInfo", report.hardware.board || "-");
  setText("#biosInfo", report.hardware.bios || "-");
  setText("#cpuInfo", report.hardware.cpu || "-");
  renderControlStateNotes(stateItems);
  renderVirtualMemory(report.virtual_memory);
  renderBlueScreen(report.blue_screen);
  renderStateList(stateItems);
  renderChecks(stateItems);
}

function setBusy(isBusy) {
  const selectedFormula = selectedFormulaFor(latestReport?.virtual_memory || {});
  const customFormulaSelected = selectedFormula?.mode === "custom";
  refreshButton.disabled = isBusy;
  if (restoreInitialButton) {
    restoreInitialButton.disabled = isBusy;
  }
  for (const button of actionButtons) {
    button.disabled = isBusy;
  }
  for (const button of firmwareButtons) {
    button.disabled = isBusy;
  }
  for (const button of pagefileActionButtons) {
    button.disabled = isBusy;
  }
  for (const button of bsodActionButtons) {
    button.disabled = isBusy;
  }
  for (const button of gamingPanelButtons) {
    button.disabled = isBusy;
  }
  for (const button of monitorIdentityPanelButtons) {
    button.disabled = isBusy;
  }
  if (monitorIdentityConfirmButton) {
    monitorIdentityConfirmButton.disabled = isBusy || !monitorIdentityStatus?.pending_confirmation;
  }
  if (monitorIdentitySelect) {
    monitorIdentitySelect.disabled = isBusy;
  }
  for (const input of [
    monitorManufacturerInput,
    monitorProductInput,
    monitorNumericSerialInput,
    monitorSerialInput,
    monitorNameInput
  ]) {
    if (input) {
      input.disabled = isBusy;
    }
  }
  for (const button of formulaOptionButtons) {
    button.disabled = isBusy;
  }
  if (pagefileInitialInput) {
    pagefileInitialInput.disabled = isBusy || !customFormulaSelected;
  }
  if (pagefileMaximumInput) {
    pagefileMaximumInput.disabled = isBusy || !customFormulaSelected;
  }
  for (const checkbox of [gamingPresetHags, gamingPresetCapture, gamingPresetFullscreen]) {
    if (checkbox) {
      checkbox.disabled = isBusy;
    }
  }
}

async function runChecks(mode = "fast") {
  const normalizedMode = mode === "full" ? "full" : "fast";
  if (runChecksPromise) {
    if (normalizedMode === "full" && runChecksPromiseMode !== "full") {
      runChecksPromiseMode = "full";
      runChecksPromise = runChecksPromise
        .finally(() => runChecksOnce("full"))
        .finally(() => {
          runChecksPromise = null;
          runChecksPromiseMode = null;
        });
    }
    return runChecksPromise;
  }

  runChecksPromiseMode = normalizedMode;
  runChecksPromise = runChecksOnce(normalizedMode).finally(() => {
    runChecksPromise = null;
    runChecksPromiseMode = null;
  });
  return runChecksPromise;
}

async function runChecksOnce(mode = "fast") {
  setBusy(true);
  messageEl.textContent = "正在读取系统状态...";
  try {
    await waitForNextPaint();
    const report = await invoke()("run_checks", { mode });
    renderReport(report);
    messageEl.textContent = hasTauri
      ? `最后刷新：${new Date(report.generated_at).toLocaleString()}`
      : "浏览器预览模式";
  } catch (error) {
    messageEl.textContent = `读取失败：${error}`;
  } finally {
    setBusy(false);
  }
}

async function runAction(action) {
  const meta = actionMeta[action];
  if (!meta) {
    return;
  }
  const riskItems = riskItemsForAction(action);

  const confirmed = await confirmDialog({
    variant: action === "restart_windows" ? "danger" : "warning",
    eyebrow: action === "restart_windows" ? "立即重启" : "系统设置",
    title: action === "restart_windows" ? "确认立即重启电脑？" : "确认修改 Windows 设置？",
    body: normalizeConfirmText(meta.confirm),
    riskItems,
    requireAcknowledge: riskItems.length > 0,
    details: meta.details || "这类操作通常需要管理员权限，并且需要重启后才会完全生效。",
    confirmText: action === "restart_windows" ? "立即重启" : "确认执行"
  });
  if (!confirmed) {
    return;
  }

  setBusy(true);
  messageEl.textContent = meta.busy;
  try {
    await waitForNextPaint();
    const result = await invoke()("apply_requirement_action", { action });
    const restartText = result.requires_restart ? " 需要重启后完全生效。" : "";
    const outputText = result.output ? ` 输出：${result.output}` : "";
    const resultMessage = `${result.message}${restartText}${outputText}`;
    messageEl.textContent = resultMessage;

    if (action !== "restart_windows") {
      await runChecks();
      messageEl.textContent = resultMessage;
    }
  } catch (error) {
    messageEl.textContent = `执行失败：${error}`;
  } finally {
    setBusy(false);
  }
}

function readPagefileInputs() {
  const initial = Number.parseInt(pagefileInitialInput?.value || "", 10);
  const maximum = Number.parseInt(pagefileMaximumInput?.value || "", 10);
  if (!Number.isFinite(initial) || !Number.isFinite(maximum)) {
    throw new Error("请填写初始大小和最大大小。");
  }
  if (initial < 256 || maximum < 256) {
    throw new Error("虚拟内存大小不能低于 256 MB。");
  }
  if (maximum < initial) {
    throw new Error("最大大小必须大于或等于初始大小。");
  }
  return { initial, maximum };
}

async function applyVirtualMemorySystemManaged(skipConfirm = false) {
  if (!skipConfirm) {
    const confirmed = await confirmDialog({
      variant: "warning",
      eyebrow: "虚拟内存",
      title: "改为 Windows 系统托管？",
      body: "Windows 会按需调整 pagefile.sys，大多数 32 GB 以上内存的电脑推荐使用这个模式。",
      riskItems: [
        "会修改系统盘页面文件配置，通常需要重启后完全生效。",
        "正在运行的大型应用或游戏不会立即使用新的页面文件策略。"
      ],
      requireAcknowledge: true,
      details: "该设置通常需要重启后完全生效。",
      confirmText: "启用系统托管"
    });
    if (!confirmed) {
      return;
    }
  }

  setBusy(true);
  messageEl.textContent = "正在设置虚拟内存为系统托管...";
  try {
    await waitForNextPaint();
    const result = await invoke()("set_virtual_memory_system_managed");
    const resultMessage = `${result.message} 需要重启后完全生效。`;
    await runChecks();
    messageEl.textContent = resultMessage;
  } catch (error) {
    messageEl.textContent = `虚拟内存设置失败：${error}`;
  } finally {
    setBusy(false);
  }
}

async function applyVirtualMemoryCustom(initial, maximum, skipConfirm = false) {
  if (!skipConfirm) {
    const confirmed = await confirmDialog({
      variant: "warning",
      eyebrow: "虚拟内存",
      title: "应用自定义页面文件？",
      body: `将系统盘 pagefile.sys 设置为：初始 ${initial} MB，最大 ${maximum} MB。`,
      riskItems: [
        "设置过小可能导致应用报错、崩溃或无法生成完整转储。",
        "设置过大会长期占用系统盘空间，修改后通常需要重启。"
      ],
      requireAcknowledge: true,
      details: "设置过小可能导致应用报错或崩溃；设置过大会占用系统盘空间。该设置通常需要重启后完全生效。",
      confirmText: "应用自定义"
    });
    if (!confirmed) {
      return;
    }
  }

  setBusy(true);
  messageEl.textContent = "正在设置自定义虚拟内存...";
  try {
    await waitForNextPaint();
    const result = await invoke()("set_virtual_memory_custom", {
      request: {
        initial_size_mb: initial,
        maximum_size_mb: maximum
      }
    });
    const resultMessage = `${result.message} 需要重启后完全生效。`;
    await runChecks();
    messageEl.textContent = resultMessage;
  } catch (error) {
    messageEl.textContent = `虚拟内存设置失败：${error}`;
  } finally {
    setBusy(false);
  }
}

async function configureMinidumpCollection() {
  const confirmed = await confirmDialog({
    variant: "warning",
    eyebrow: "蓝屏分析",
    title: "开启小内存转储？",
    body: "将把 Windows 的“写入调试信息”设置为小内存转储（Minidump），并把路径设置为 %SystemRoot%\\Minidump。",
    details: "这不会立刻生成文件；只有下次蓝屏时才会生成转储文件（.dmp）。通常不需要重启，但之后蓝屏才会看到结果。",
    confirmText: "开启转储收集（DMP）"
  });
  if (!confirmed) {
    return;
  }

  setBusy(true);
  messageEl.textContent = "正在开启转储收集（DMP）...";
  try {
    await waitForNextPaint();
    const result = await invoke()("configure_minidump_collection");
    await runChecks();
    messageEl.textContent = result.message;
  } catch (error) {
    messageEl.textContent = `转储收集（DMP）配置失败：${error}`;
  } finally {
    setBusy(false);
  }
}

async function openBlueScreenView() {
  setBusy(true);
  messageEl.textContent = "正在打开蓝屏查看器（BlueScreenView）...";
  try {
    await waitForNextPaint();
    const result = await invoke()("open_bluescreenview");
    messageEl.textContent = result.message;
  } catch (error) {
    messageEl.textContent = `打开蓝屏查看器（BlueScreenView）失败：${error}`;
  } finally {
    setBusy(false);
  }
}

async function exportBlueScreenReport() {
  setBusy(true);
  messageEl.textContent = "正在导出蓝屏报告...";
  try {
    await waitForNextPaint();
    const result = await invoke()("export_bluescreen_report");
    messageEl.textContent = `${result.message} ${result.output || ""}`.trim();
  } catch (error) {
    messageEl.textContent = `导出蓝屏报告失败：${error}`;
  } finally {
    setBusy(false);
  }
}

async function restoreInitialConfig() {
  const confirmed = await confirmDialog({
    variant: "danger",
    eyebrow: "恢复配置",
    title: "恢复第一次保存的系统配置？",
    body: "将恢复本软件第一次读取并保存的初始配置。",
    riskItems: [
      "会批量恢复多个 Windows 设置，执行后通常需要重启才会完全生效。",
      "BIOS/UEFI 项无法自动恢复，仍需要你进入固件界面手动确认。"
    ],
    requireAcknowledge: true,
    details:
      "会恢复：Hyper-V（微软虚拟机监控程序）、虚拟机平台（Virtual Machine Platform）、Windows 虚拟机监控平台（Windows Hypervisor Platform）、虚拟机监控程序启动项（hypervisorlaunchtype）、快速启动（Windows Fast Startup）、内存压缩（Memory Compression）、休眠可用性、虚拟内存。\n\n不会自动恢复固件项（BIOS/UEFI）：CPU 虚拟化（VT-x / SVM）、设备直通（VT-d / IOMMU）、安全启动（Secure Boot）、可信平台模块（TPM / PTT / fTPM）。执行后通常需要重启才会完全生效。",
    confirmText: "恢复初始配置"
  });
  if (!confirmed) {
    return;
  }

  setBusy(true);
  messageEl.textContent = "正在恢复初始配置...";
  try {
    await waitForNextPaint();
    const result = await invoke()("restore_initial_config");
    const resultMessage = `${result.message} 需要重启后完全生效。`;
    const firmwareText =
      "固件项（BIOS/UEFI）无法由 Windows 程序自动恢复：CPU 虚拟化（VT-x / SVM）、设备直通（VT-d / IOMMU）、安全启动（Secure Boot）、可信平台模块（TPM / PTT / fTPM）。请按主板 BIOS 页面手动确认。";
    await notifyDialog({
      variant: "warning",
      eyebrow: "恢复完成",
      title: "已恢复可由 Windows 修改的项目",
      body: resultMessage,
      details: firmwareText,
      confirmText: "知道了"
    });
    await runChecks();
    messageEl.textContent = resultMessage;
  } catch (error) {
    messageEl.textContent = `恢复初始配置失败：${error}`;
  } finally {
    setBusy(false);
  }
}

refreshButton.addEventListener("click", (event) => runChecks(event.shiftKey ? "full" : "fast"));
restoreInitialButton?.addEventListener("click", restoreInitialConfig);
themeToggleButton?.addEventListener("click", toggleTheme);
aboutButton?.addEventListener("click", showAboutDialog);

for (const item of navItems) {
  item.addEventListener("click", () => switchView(item.dataset.view));
}

for (const button of actionButtons) {
  button.addEventListener("click", () => runAction(button.dataset.action));
}

gamingApplyPresetButton?.addEventListener("click", applyGamingCompetitivePreset);
gamingRestoreChangesButton?.addEventListener("click", restoreGamingOptimizerChanges);
gamingRefreshButton?.addEventListener("click", refreshGamingOptimizerStatus);
gamingBrowseGameButton?.addEventListener("click", browseGamingGameExe);
for (const button of gamingActionButtons) {
  button.addEventListener("click", () => runGamingAction(button.dataset.gamingAction));
}
monitorIdentitySelect?.addEventListener("change", () => {
  const selected = selectedMonitorIdentity();
  renderMonitorIdentityCurrent(selected);
  fillMonitorIdentityInputs(selected);
});
monitorIdentityRandomButton?.addEventListener("click", randomizeMonitorIdentityFields);
monitorIdentityRefreshButton?.addEventListener("click", refreshMonitorIdentityStatus);
monitorIdentityApplyButton?.addEventListener("click", () => applyMonitorIdentityOverride("registry"));
monitorIdentityInstallInfButton?.addEventListener("click", installMonitorIdentityInfOverride);
monitorIdentityReenumerateButton?.addEventListener("click", reenumerateMonitorIdentityDevice);
monitorIdentityConfirmButton?.addEventListener("click", confirmMonitorIdentityOverride);
monitorIdentityRollbackButton?.addEventListener("click", restoreMonitorIdentityOverride);

applyCustomPagefileButton?.addEventListener("click", async () => {
  const formula = selectedFormulaFor(latestReport?.virtual_memory || {});
  if (formula?.mode !== "custom") {
    await applyVirtualMemorySystemManaged();
    return;
  }
  try {
    const { initial, maximum } = readPagefileInputs();
    await applyVirtualMemoryCustom(initial, maximum);
  } catch (error) {
    messageEl.textContent = `虚拟内存设置失败：${error.message || error}`;
  }
});

configureDumpButton?.addEventListener("click", configureMinidumpCollection);
openBlueScreenViewButton?.addEventListener("click", openBlueScreenView);
exportBlueScreenReportButton?.addEventListener("click", exportBlueScreenReport);

for (const button of firmwareButtons) {
  button.addEventListener("click", async () => {
    const confirmed = await confirmDialog({
      variant: "danger",
      eyebrow: "固件设置（BIOS / UEFI）",
      title: "立即重启并进入固件设置（BIOS/UEFI）？",
      body: "电脑会立即重启，并尝试进入主板固件设置（BIOS/UEFI）界面。",
      riskItems: [
        "电脑会立即重启，未保存的文件和任务会丢失。",
        "进入固件设置（BIOS/UEFI）后需要手动修改参数，误改启动或安全项可能导致系统无法按预期启动。"
      ],
      requireAcknowledge: true,
      details: "请先保存其他程序中的工作。进入固件设置（BIOS/UEFI）后，CPU 虚拟化（VT-x / SVM）、设备直通（VT-d / IOMMU）、安全启动（Secure Boot）、可信平台模块（TPM / PTT / fTPM）需要手动修改。",
      confirmText: "重启进 BIOS"
    });
    if (!confirmed) {
      return;
    }

    setBusy(true);
    messageEl.textContent = "正在请求重启进入固件设置（BIOS/UEFI）...";
    try {
      await waitForNextPaint();
      await invoke()("restart_to_firmware");
    } catch (error) {
      messageEl.textContent = `无法重启进入固件设置（BIOS/UEFI）：${error}`;
      setBusy(false);
    }
  });
}

dialogConfirm?.addEventListener("click", () => closeDialog(true));
dialogCancel?.addEventListener("click", () => closeDialog(false));
dialogLinkAction?.addEventListener("click", () => {
  if (dialogLinkAction.dataset.action === "repository") {
    openProjectRepository();
  }
});
dialogAcknowledge?.addEventListener("change", () => {
  if (dialogConfirm && dialogAcknowledgeWrap && !dialogAcknowledgeWrap.hidden) {
    dialogConfirm.disabled = !dialogAcknowledge.checked;
  }
});
dialogBackdrop?.addEventListener("click", (event) => {
  if (event.target === dialogBackdrop && !dialogCancel?.hidden) {
    closeDialog(false);
  }
});
window.addEventListener("keydown", (event) => {
  if (!dialogBackdrop?.classList.contains("open")) {
    return;
  }
  if (event.key === "Escape" && !dialogCancel?.hidden) {
    closeDialog(false);
  }
});

installDesktopShellGuards();
applyTheme(document.documentElement.dataset.theme);
bindWindowControls();
loadStoredGamingGamePath();
switchView("overview");
setTimeout(runChecks, 80);
