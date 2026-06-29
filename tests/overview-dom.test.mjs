import { readFileSync } from "node:fs";
import assert from "node:assert/strict";
import { JSDOM } from "jsdom";

const html = readFileSync(new URL("../frontend/index.html", import.meta.url), "utf8");
const js = readFileSync(new URL("../frontend/main.js", import.meta.url), "utf8");
const css = readFileSync(new URL("../frontend/styles.css", import.meta.url), "utf8");
const iconPng = readFileSync(new URL("../src-tauri/icons/icon.png", import.meta.url));
const iconIco = readFileSync(new URL("../src-tauri/icons/icon.ico", import.meta.url));
const packageJson = JSON.parse(readFileSync(new URL("../package.json", import.meta.url), "utf8"));
const packageLock = JSON.parse(readFileSync(new URL("../package-lock.json", import.meta.url), "utf8"));
const licenseText = readFileSync(new URL("../LICENSE", import.meta.url), "utf8");
const thirdPartyNotices = readFileSync(new URL("../THIRD_PARTY_NOTICES.md", import.meta.url), "utf8");
const readme = readFileSync(new URL("../README.md", import.meta.url), "utf8");
const readmeEn = readFileSync(new URL("../README.en.md", import.meta.url), "utf8");
const cargoToml = readFileSync(new URL("../src-tauri/Cargo.toml", import.meta.url), "utf8");
const tauriConfig = JSON.parse(readFileSync(new URL("../src-tauri/tauri.conf.json", import.meta.url), "utf8"));
const tauriCapability = JSON.parse(readFileSync(new URL("../src-tauri/capabilities/default.json", import.meta.url), "utf8"));

const dom = new JSDOM(html, {
  url: "http://localhost/",
  runScripts: "outside-only",
  pretendToBeVisual: true
});

const windowApiCalls = [];
let fakeWindowMaximized = false;
let fakeResizeHandler = null;
const fakeTauriWindow = {
  minimize() {
    windowApiCalls.push("minimize");
    return Promise.resolve();
  },
  toggleMaximize() {
    windowApiCalls.push("toggleMaximize");
    fakeWindowMaximized = !fakeWindowMaximized;
    return Promise.resolve();
  },
  close() {
    windowApiCalls.push("close");
    return Promise.resolve();
  },
  isMaximized() {
    windowApiCalls.push("isMaximized");
    return Promise.resolve(fakeWindowMaximized);
  },
  onResized(callback) {
    windowApiCalls.push("onResized");
    fakeResizeHandler = callback;
    return Promise.resolve(() => {});
  }
};
dom.window.__TAURI__ = {
  window: {
    getCurrentWindow() {
      return fakeTauriWindow;
    }
  }
};
dom.window.eval(js);

// main.js 在 80ms 后自动 runChecks（无 Tauri 时渲染 previewReport）
await new Promise((resolve) => setTimeout(resolve, 300));

const { document } = dom.window;
const failures = [];

function check(name, fn) {
  try {
    fn();
    console.log(`  ok: ${name}`);
  } catch (error) {
    failures.push(name);
    console.error(`  FAIL: ${name}\n    ${error.message}`);
  }
}

async function checkAsync(name, fn) {
  try {
    await fn();
    console.log(`  ok: ${name}`);
  } catch (error) {
    failures.push(name);
    console.error(`  FAIL: ${name}\n    ${error.message}`);
  }
}

function hexToRgb(hex) {
  const normalized = hex.replace("#", "");
  return {
    r: Number.parseInt(normalized.slice(0, 2), 16) / 255,
    g: Number.parseInt(normalized.slice(2, 4), 16) / 255,
    b: Number.parseInt(normalized.slice(4, 6), 16) / 255
  };
}

function channelToLinear(value) {
  return value <= 0.03928 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
}

function luminance(hex) {
  const { r, g, b } = hexToRgb(hex);
  return 0.2126 * channelToLinear(r) + 0.7152 * channelToLinear(g) + 0.0722 * channelToLinear(b);
}

function contrastRatio(foreground, background) {
  const lighter = Math.max(luminance(foreground), luminance(background));
  const darker = Math.min(luminance(foreground), luminance(background));
  return (lighter + 0.05) / (darker + 0.05);
}

function pngInfo(buffer) {
  assert.deepEqual([...buffer.subarray(0, 8)], [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);
  return {
    width: buffer.readUInt32BE(16),
    height: buffer.readUInt32BE(20),
    colorType: buffer[25]
  };
}

function icoSizes(buffer) {
  assert.equal(buffer.readUInt16LE(0), 0, "ICO reserved field should be 0");
  assert.equal(buffer.readUInt16LE(2), 1, "ICO type should be icon");
  const count = buffer.readUInt16LE(4);
  return Array.from({ length: count }, (_, index) => {
    const offset = 6 + index * 16;
    return [buffer[offset] || 256, buffer[offset + 1] || 256];
  });
}

console.log("overview-dom.test.mjs");

check("旧的 Windows 功能面板已删除", () => {
  for (const id of ["hyperVState", "vmpState", "whpState", "hypervisorLaunchState", "fastStartupState"]) {
    assert.equal(document.getElementById(id), null, `#${id} 不应存在`);
  }
  assert.equal(document.querySelector(".state-grid"), null, ".state-grid 不应存在");
});

check("概览页改为健康面板 + 状态矩阵仪表盘", () => {
  assert.ok(document.querySelector("#overviewPane .health-panel"), "应存在系统体检健康面板");
  assert.ok(document.querySelector("#overviewPane .state-matrix-panel"), "应存在状态矩阵面板");
  assert.equal(document.querySelectorAll("#overviewPane .state-matrix-panel").length, 1);
  assert.equal(document.querySelector("#healthScore").textContent, "7/12");
});

check("新信息架构包含分组导航和顶部全局操作", () => {
  const navLabels = [...document.querySelectorAll(".nav-section-label")].map((node) => node.textContent);
  assert.deepEqual(navLabels, ["总览", "控制", "诊断"]);
  assert.equal(document.querySelector('[data-view="controls"]').textContent.includes("快速控制"), true);
  assert.equal(document.querySelector('[data-view="gaming"]').textContent.includes("竞技模式"), true);
  assert.ok(document.querySelector(".search-box input"), "顶部应有搜索输入框");
  assert.ok(document.getElementById("themeToggleButton"), "顶部应有亮暗主题切换按钮");
  assert.equal(document.querySelector(".header-actions #aboutButton"), null, "顶部操作区不应再放关于按钮");
  assert.ok(document.querySelector(".sidebar-actions #aboutButton"), "侧栏底部应有关于按钮");
  assert.ok(document.getElementById("refreshButton"), "顶部应有刷新全部按钮");
  assert.ok(document.getElementById("restoreInitialButton"), "顶部应有恢复初始按钮");
});

check("Phosphor 图标以本地 SVG sprite 接入关键导航和操作区", () => {
  for (const id of [
    "ph-gauge",
    "ph-sliders-horizontal",
    "ph-memory",
    "ph-warning-octagon",
    "ph-list-checks",
    "ph-cpu",
    "ph-magnifying-glass",
    "ph-moon",
    "ph-sun",
    "ph-arrows-clockwise",
    "ph-arrow-counter-clockwise",
    "ph-shield-check",
    "ph-hard-drives",
    "ph-info"
  ]) {
    assert.ok(document.getElementById(id), `缺少 Phosphor symbol: ${id}`);
  }

  assert.equal(document.querySelector(".icon-sprite").getAttribute("aria-hidden"), "true");
  assert.deepEqual(
    [...document.querySelectorAll(".nav-code use")].map((node) => node.getAttribute("href")),
    [
      "#ph-gauge",
      "#ph-sliders-horizontal",
      "#ph-memory",
      "#ph-gear-six",
      "#ph-hard-drives",
      "#ph-warning-octagon",
      "#ph-list-checks",
      "#ph-cpu"
    ]
  );
  assert.equal(document.querySelector(".search-box .search-icon use").getAttribute("href"), "#ph-magnifying-glass");
  assert.equal(document.querySelector("#aboutButton use").getAttribute("href"), "#ph-info");
  assert.equal(document.querySelector("#refreshButton use").getAttribute("href"), "#ph-arrows-clockwise");
  assert.equal(document.querySelector("#restoreInitialButton use").getAttribute("href"), "#ph-arrow-counter-clockwise");
  assert.equal(document.querySelector("#themeToggleIcon use").getAttribute("href"), "#ph-moon");
  assert.equal(document.querySelector(".health-panel .section-icon use").getAttribute("href"), "#ph-shield-check");
  assert.equal(document.querySelector("#appDialog .dialog-icon use").getAttribute("href"), "#ph-warning-circle");
  assert.match(css, /\.ph-icon\s*\{[\s\S]*fill: currentColor;[\s\S]*height: 16px;/, "图标应继承当前文字颜色");
  assert.match(
    css,
    /\.search-box input\s*\{[\s\S]*font-size: 12px;[\s\S]*padding: 0 var\(--space-5\) 0 calc\(var\(--space-5\) \+ 23px\);/,
    "搜索框应使用更小字号并为左侧图标预留输入内边距"
  );
  assert.match(
    css,
    /\.search-box input::placeholder\s*\{[\s\S]*font-size: 12px;[\s\S]*font-weight: 600;/,
    "搜索框占位文字应保持紧凑"
  );
});

check("竞技模式页面包含安全预设、实验项和可还原操作", () => {
  const pane = document.getElementById("gamingPane");
  assert.ok(pane, "应存在竞技模式面板");
  assert.ok(document.getElementById("gamingApplyPresetButton"), "应有安全预设按钮");
  assert.ok(document.getElementById("gamingRestoreChangesButton"), "应有还原按钮");
  assert.equal(document.getElementById("gamingPresetHags").checked, true);
  assert.equal(document.getElementById("gamingPresetCapture").checked, true);
  assert.equal(document.getElementById("gamingPresetFullscreen").checked, true);
  assert.equal(pane.textContent.includes("不包含电源计划"), true, "安全预设不应包含电源计划");
  assert.ok(document.querySelector('[data-gaming-card="game-mode"].experimental'), "Game Mode 应标为实验项");
  assert.equal(document.querySelectorAll(".gaming-action-button").length, 8);
});

check("竞技模式状态改为按需加载，避免启动时额外后端查询", () => {
  assert.ok(
    js.includes("loadStoredGamingGamePath();\nswitchView(\"overview\");"),
    "启动时应只恢复已选游戏路径，然后进入总览"
  );
  assert.ok(!js.includes("void initGamingOptimizerPanel();\nswitchView(\"overview\");"), "启动时不应立即刷新竞技模式状态");
  assert.match(
    js,
    /if \(view === "gaming" && !gamingOptimizerStatus\) \{\s*void initGamingOptimizerPanel\(\);/s,
    "进入竞技模式页时应按需初始化竞技模式状态"
  );
});

check("长耗时操作先让 UI 完成一帧绘制并避免重复刷新", () => {
  assert.match(js, /function waitForNextPaint\(\) \{[\s\S]*requestAnimationFrame[\s\S]*setTimeout\(resolve, 0\);/, "应提供下一帧让路工具");
  assert.ok(js.includes("let runChecksPromise = null;"), "整机刷新应有 in-flight 去重状态");
  assert.ok(js.includes("let runChecksPromiseMode = null;"), "整机刷新应记录 in-flight 模式");
  assert.match(
    js,
    /if \(runChecksPromise\) \{[\s\S]*return runChecksPromise;\s*\}/,
    "重复触发整机刷新时应复用正在运行的 Promise"
  );
  assert.ok(
    js.includes("if (normalizedMode === \"full\" && runChecksPromiseMode !== \"full\")"),
    "full 深度刷新不应被正在运行的 fast 刷新吞掉"
  );
  assert.ok(js.includes("const report = await invoke()(\"run_checks\", { mode });"), "run_checks 应显式传入刷新模式");
  assert.ok(js.includes("refreshButton.addEventListener(\"click\", (event) => runChecks(event.shiftKey ? \"full\" : \"fast\"));"), "普通刷新应走 fast，Shift 刷新保留 full");
  assert.ok(js.includes("await waitForNextPaint();\n    const report = await invoke()(\"run_checks\", { mode });"), "run_checks 前应先让 UI 绘制 busy 状态");
  assert.ok(js.includes("let formulaOptionButtons = [];"), "公式按钮应缓存，避免 setBusy 时重复查询 DOM");
  assert.ok(!js.includes("document.querySelectorAll(\".formula-option\")"), "setBusy 不应每次重新 querySelectorAll 公式按钮");
});

check("交互动效保持轻量，避免模糊和大面积阴影过渡", () => {
  const buttonBlock = css.slice(css.indexOf("button {\n"), css.indexOf("button:disabled"));
  const requirementRowBlock = css.slice(css.indexOf(".requirement-row {\n"), css.indexOf(".requirement-row:hover"));
  assert.ok(!css.includes("backdrop-filter"), "弹窗遮罩不应使用 backdrop-filter 模糊");
  assert.ok(!css.includes("box-shadow 120ms ease"), "常用 hover 过渡不应动画化 box-shadow");
  assert.ok(!buttonBlock.includes("transform"), "全局按钮过渡不应动画化 transform");
  assert.ok(!requirementRowBlock.includes("transform"), "状态列表行过渡不应动画化 transform");
  assert.ok(css.includes("transition: border-color 100ms ease;"), "面板/列表 hover 应保留轻量边框反馈");
});

check("桌面外壳屏蔽浏览器式交互痕迹", () => {
  for (const permission of ["core:webview:deny-internal-toggle-devtools"]) {
    assert.ok(tauriCapability.permissions.includes(permission), `缺少 WebView 限制权限：${permission}`);
  }

  assert.match(
    css,
    /html,\s*[\r\n]+body\s*\{[\s\S]*user-select: none;[\s\S]*-webkit-user-select: none;/,
    "页面默认不应像网页一样可随手选中文字"
  );
  assert.match(
    css,
    /input,\s*[\s\S]*textarea,\s*[\s\S]*select,\s*[\s\S]*\[contenteditable=""\],\s*[\s\S]*\[contenteditable="true"\]\s*\{[\s\S]*user-select: text;/,
    "输入类控件仍应允许选择文字"
  );
  assert.match(css, /img,\s*[\r\n]+svg\s*\{[\s\S]*-webkit-user-drag: none;/, "图片和 SVG 不应触发网页拖拽");

  const contextEvent = new dom.window.MouseEvent("contextmenu", { bubbles: true, cancelable: true });
  document.body.dispatchEvent(contextEvent);
  assert.equal(contextEvent.defaultPrevented, true, "右键网页菜单应被拦截");

  const dragEvent = new dom.window.Event("dragstart", { bubbles: true, cancelable: true });
  document.querySelector(".titlebar-mark-icon").dispatchEvent(dragEvent);
  assert.equal(dragEvent.defaultPrevented, true, "拖拽网页元素应被拦截");

  const titleSelectEvent = new dom.window.Event("selectstart", { bubbles: true, cancelable: true });
  document.querySelector(".titlebar-title").dispatchEvent(titleSelectEvent);
  assert.equal(titleSelectEvent.defaultPrevented, true, "普通界面文字不应触发网页选区");

  const inputSelectEvent = new dom.window.Event("selectstart", { bubbles: true, cancelable: true });
  document.querySelector(".search-box input").dispatchEvent(inputSelectEvent);
  assert.equal(inputSelectEvent.defaultPrevented, false, "搜索输入框仍应允许选择文字");

  for (const event of [
    new dom.window.KeyboardEvent("keydown", { key: "F5", bubbles: true, cancelable: true }),
    new dom.window.KeyboardEvent("keydown", { key: "r", ctrlKey: true, bubbles: true, cancelable: true }),
    new dom.window.KeyboardEvent("keydown", { key: "ArrowLeft", altKey: true, bubbles: true, cancelable: true })
  ]) {
    dom.window.dispatchEvent(event);
    assert.equal(event.defaultPrevented, true, `${event.key} 网页快捷键应被拦截`);
  }
});

check("发布构建启用体积优化 profile", () => {
  assert.ok(cargoToml.includes("[profile.release]"), "Cargo.toml 应配置 release profile");
  assert.ok(cargoToml.includes("lto = true"), "release 应开启 LTO");
  assert.ok(cargoToml.includes("codegen-units = 1"), "release 应减少 codegen units 以优化体积");
  assert.ok(cargoToml.includes("panic = \"abort\""), "release 应使用 abort panic strategy");
  assert.ok(cargoToml.includes("strip = \"symbols\""), "release 应剥离符号");
});

check("发布身份不再暴露旧工程名", () => {
  assert.equal(packageJson.name, "zhiji");
  assert.equal(packageLock.name, "zhiji");
  assert.equal(packageLock.packages[""].name, "zhiji");
  assert.match(cargoToml, /\[package\][\s\S]*name = "zhiji"/);
  assert.match(cargoToml, /\[lib\][\s\S]*name = "zhiji_lib"/);
});

check("开源协议和第三方声明已写入仓库", () => {
  assert.ok(licenseText.startsWith("MIT License"));
  assert.ok(licenseText.includes("Copyright (c) 2026 LingCore"));
  assert.ok(readme.includes("[English](README.en.md)"));
  assert.ok(readme.includes("GitHub Releases: https://github.com/LingCore/zhiji/releases"));
  assert.ok(readme.includes("项目源码使用 MIT License"));
  assert.ok(readmeEn.includes("[中文](README.md)"));
  assert.ok(readmeEn.includes("GitHub Releases: https://github.com/LingCore/zhiji/releases"));
  assert.ok(readmeEn.includes("The project source code is released under the MIT License."));
  assert.ok(thirdPartyNotices.includes("BlueScreenView"));
  assert.ok(thirdPartyNotices.includes("not covered by this project's"));
});

await checkAsync("竞技模式选择游戏 exe 后刷新全屏优化状态", async () => {
  document.getElementById("gamingBrowseGameButton").click();
  await new Promise((resolve) => setTimeout(resolve, 0));
  assert.equal(document.getElementById("gamingSelectedGamePath").textContent.includes("game.exe"), true);
  assert.notEqual(document.getElementById("gamingFullscreenBadge").textContent, "未选择游戏");
});

check("Tauri 主窗口按无边框方案配置", () => {
  const mainWindow = tauriConfig.app.windows.find((windowConfig) => windowConfig.label === "main");
  assert.ok(mainWindow, "必须存在 label=main 的窗口配置");
  assert.equal(tauriConfig.productName, "知机");
  assert.equal(tauriConfig.mainBinaryName, "Zhiji");
  assert.equal(tauriConfig.identifier, "com.lingcore.zhiji");
  assert.deepEqual(tauriConfig.bundle.targets, ["nsis"]);
  assert.equal(mainWindow.title, "知机");
  assert.equal(mainWindow.decorations, false, "无边框窗口必须关闭 decorations");
  assert.equal(mainWindow.shadow, true, "Windows 11 无边框窗口应保留 shadow");
  assert.equal(mainWindow.transparent, false, "不做 Mica/Acrylic 时 transparent 应保持 false");
  assert.equal(mainWindow.resizable, true, "无边框窗口仍应可调整大小");
  assert.ok(mainWindow.minWidth >= 900, "应保留最小宽度避免布局压坏");
  assert.ok(mainWindow.minHeight >= 600, "应保留最小高度避免布局压坏");

  for (const permission of [
    "core:window:allow-start-dragging",
    "core:window:allow-toggle-maximize",
    "core:window:allow-minimize",
    "core:window:allow-close",
    "core:window:allow-is-maximized"
  ]) {
    assert.ok(tauriCapability.permissions.includes(permission), `缺少无边框窗口权限：${permission}`);
  }
  assert.equal(tauriConfig.bundle.windows.nsis.installerIcon, "icons/icon.ico");
  assert.equal(tauriConfig.bundle.windows.nsis.uninstallerIcon, "icons/icon.ico");
});

check("应用图标复用标题栏 Logo 的浅色芯片风格", () => {
  const png = pngInfo(iconPng);
  assert.equal(png.width, 256);
  assert.equal(png.height, 256);
  assert.equal(png.colorType, 6, "PNG 图标应保留透明通道");
  assert.deepEqual(icoSizes(iconIco), [
    [16, 16],
    [32, 32],
    [48, 48],
    [64, 64],
    [128, 128],
    [256, 256]
  ]);
  assert.match(
    css,
    /\.titlebar-mark\s*\{[\s\S]*background: linear-gradient\(135deg, #f5f5f5, #bdbdbd\);[\s\S]*color: #050505;/,
    "标题栏 Logo 应继续使用浅灰渐变底和深色芯片图标"
  );
});

check("自定义标题栏具备拖拽区和窗口控制按钮", () => {
  const titlebar = document.getElementById("appTitlebar");
  assert.ok(titlebar, "应存在自定义标题栏");
  assert.equal(titlebar.getAttribute("data-tauri-drag-region"), "", "标题栏应声明 Tauri 拖拽区域");
  assert.equal(document.querySelector(".titlebar-left").getAttribute("data-tauri-drag-region"), "", "标题栏左侧也应可拖拽");
  assert.equal(document.querySelector(".titlebar-title").textContent, "知机");
  assert.equal(document.querySelector(".titlebar-subtitle"), null, "标题栏左侧不应再显示第二行副标题");
  assert.equal(document.querySelector(".titlebar-pill"), null, "标题栏不应显示 Tauri / Rust 技术标签");
  assert.ok(document.querySelector(".titlebar-status #message"), "运行状态应移动到标题栏");
  assert.equal(document.querySelector(".sidebar-footer"), null, "侧栏底部运行状态卡片应移除");
  assert.equal(document.querySelector(".sidebar .brand"), null, "侧栏不应再保留重复的品牌 logo 区域");
  assert.equal(document.querySelectorAll(".sidebar-system .meta-chip").length, 1, "侧栏状态区只保留管理员状态");
  assert.equal(document.querySelectorAll(".sidebar-actions #aboutButton").length, 1, "侧栏操作区应只放一个关于入口");
  assert.equal(document.getElementById("windowMinimizeButton").getAttribute("aria-label"), "最小化窗口");
  assert.equal(document.getElementById("windowMaximizeButton").getAttribute("aria-label"), "最大化窗口");
  assert.equal(document.getElementById("windowCloseButton").getAttribute("aria-label"), "关闭窗口");
  assert.ok(css.includes(".titlebar {\n  align-items: center;"), "应存在 titlebar 样式块");
  assert.match(css, /\.titlebar\s*\{[\s\S]*flex: 0 0 42px;[\s\S]*height: 42px;/, "标题栏高度应更接近桌面应用标题栏");
  assert.match(css, /\.titlebar-copy\s*\{[\s\S]*display: grid;/, "标题栏应用名应保持单独成组排版");
  assert.ok(!css.includes(".titlebar-subtitle"), "标题栏副标题样式应移除");
  assert.ok(!css.includes(".titlebar-pill"), "标题栏技术标签样式应移除");
  assert.match(css, /\.window-icon-minimize::before\s*\{[\s\S]*background: currentColor;/, "最小化图标应由 CSS 绘制");
  assert.match(css, /\.window-icon-maximize\s*\{[\s\S]*border: 1\.5px solid currentColor;/, "最大化图标应由 CSS 绘制");
  assert.match(css, /\.window-icon-close::before,[\s\S]*\.window-icon-close::after\s*\{[\s\S]*background: currentColor;/, "关闭图标应由 CSS 绘制");
  assert.match(css, /\.titlebar\s*\{[\s\S]*-webkit-app-region: drag;/, "标题栏应设置 drag region");
  assert.match(css, /\.titlebar\s*\{[\s\S]*height: 42px;[\s\S]*overflow: hidden;/, "标题栏必须隐藏意外溢出的子元素");
  assert.match(css, /\.titlebar-status\s*\{[\s\S]*height: 26px;[\s\S]*max-height: 26px;[\s\S]*overflow: hidden;/, "标题栏运行状态 pill 高度必须被锁定");
  assert.match(css, /\.titlebar-status\s*\{[\s\S]*flex: 0 1 auto;[\s\S]*width: fit-content;/, "标题栏状态标签背景应按内容自适应宽度");
  assert.ok(!css.includes("flex: 0 1 min(34vw, 420px);"), "标题栏状态标签不能用固定比例 flex-basis 撑开背景");
  assert.match(css, /\.titlebar-status\s*\{[\s\S]*margin-left: auto;/, "标题栏运行状态应贴近窗口控制区");
  assert.match(css, /\.message\.titlebar-message\s*\{[\s\S]*min-height: 0;[\s\S]*text-overflow: ellipsis;[\s\S]*white-space: nowrap;/, "标题栏运行状态文本应覆盖通用 message 高度并支持省略");
  assert.ok(
    css.indexOf(".message.titlebar-message {") > css.indexOf(".message {\n"),
    "标题栏 message 覆盖规则必须位于通用 .message 之后，避免 min-height 回归"
  );
  assert.match(css, /\.titlebar-button\s*\{[\s\S]*-webkit-app-region: no-drag;/, "标题栏按钮必须排除拖拽区域");
  assert.match(css, /\.app-shell\s*\{[\s\S]*grid-template-columns: 188px minmax\(0, 1fr\);[\s\S]*min-height: 0;/, "侧栏宽度应收窄并让内容区占据剩余空间");
  assert.match(css, /\.sidebar\s*\{[\s\S]*grid-template-rows: auto minmax\(0, 1fr\) auto;/, "侧栏应保留管理员状态、导航和底部操作三行");
  assert.match(css, /\.sidebar-actions\s*\{[\s\S]*border-top: 1px solid var\(--line-soft\);/, "侧栏底部操作区应与导航分隔");
});

await checkAsync("标题栏按钮会调用 Tauri window API", async () => {
  windowApiCalls.length = 0;
  fakeWindowMaximized = false;

  document.getElementById("windowMinimizeButton").click();
  await new Promise((resolve) => setTimeout(resolve, 0));
  assert.ok(windowApiCalls.includes("minimize"), "点击最小化应调用 minimize");

  document.getElementById("windowMaximizeButton").click();
  await new Promise((resolve) => setTimeout(resolve, 0));
  assert.ok(windowApiCalls.includes("toggleMaximize"), "点击最大化应调用 toggleMaximize");
  assert.ok(windowApiCalls.includes("isMaximized"), "最大化后应查询 isMaximized 更新图标");
  assert.equal(document.getElementById("windowMaximizeButton").getAttribute("aria-label"), "还原窗口");
  assert.equal(document.getElementById("windowMaximizeIcon").classList.contains("is-restore"), true);

  document.getElementById("appTitlebar").dispatchEvent(new dom.window.MouseEvent("dblclick", { bubbles: true }));
  await new Promise((resolve) => setTimeout(resolve, 0));
  assert.equal(windowApiCalls.filter((call) => call === "toggleMaximize").length, 2, "双击标题栏应切换最大化/还原");
  assert.equal(document.getElementById("windowMaximizeIcon").classList.contains("is-restore"), false);

  document.getElementById("windowCloseButton").click();
  await new Promise((resolve) => setTimeout(resolve, 0));
  assert.ok(windowApiCalls.includes("close"), "点击关闭应调用 close");

  fakeWindowMaximized = true;
  await fakeResizeHandler?.();
  await new Promise((resolve) => setTimeout(resolve, 0));
  assert.equal(document.documentElement.dataset.windowMaximized, "true", "resize 后应刷新最大化状态");
});

check("亮暗主题切换会更新根属性、按钮状态和本地保存", () => {
  const button = document.getElementById("themeToggleButton");
  assert.equal(document.documentElement.dataset.theme, "dark");
  assert.equal(button.textContent.trim(), "深色");
  assert.equal(button.getAttribute("aria-pressed"), "true");
  assert.equal(document.querySelector("#themeToggleIcon use").getAttribute("href"), "#ph-moon");

  button.click();
  assert.equal(document.documentElement.dataset.theme, "light");
  assert.equal(button.textContent.trim(), "浅色");
  assert.equal(button.getAttribute("aria-pressed"), "false");
  assert.equal(document.querySelector("#themeToggleIcon use").getAttribute("href"), "#ph-sun");
  assert.equal(dom.window.localStorage.getItem("pc-requirements-theme"), "light");

  button.click();
  assert.equal(document.documentElement.dataset.theme, "dark");
  assert.equal(button.textContent.trim(), "深色");
  assert.equal(button.getAttribute("aria-pressed"), "true");
  assert.equal(document.querySelector("#themeToggleIcon use").getAttribute("href"), "#ph-moon");
  assert.equal(dom.window.localStorage.getItem("pc-requirements-theme"), "dark");
});

await checkAsync("关于按钮会弹出紧凑作者信息", async () => {
  document.getElementById("aboutButton").click();
  await new Promise((resolve) => setTimeout(resolve, 0));

  const dialog = document.getElementById("appDialog");
  assert.ok(dialog.classList.contains("open"), "关于弹窗应打开");
  assert.equal(document.getElementById("dialogEyebrow").textContent, "关于");
  assert.equal(document.getElementById("dialogTitle").textContent, "知机");
  assert.ok(document.getElementById("dialogBody").textContent.includes("作者：LingCore"));
  assert.ok(document.getElementById("dialogBody").textContent.includes("开源协议：MIT"));
  assert.ok(document.getElementById("dialogBody").textContent.includes("https://github.com/LingCore/zhiji"));
  assert.equal(document.querySelector("#appDialog .dialog-icon use").getAttribute("href"), "#ph-info");
  assert.equal(document.getElementById("dialogLinkAction").hidden, false, "关于弹窗应显示 GitHub 按钮");
  assert.equal(document.getElementById("dialogLinkActionLabel").textContent, "GitHub");
  assert.equal(document.querySelector("#dialogLinkAction use").getAttribute("href"), "#ph-github-logo");
  assert.equal(document.getElementById("dialogCancel").hidden, true, "关于弹窗不需要取消按钮");

  document.getElementById("dialogLinkAction").click();
  await new Promise((resolve) => setTimeout(resolve, 0));
  assert.equal(dom.window.__lastOpenedProjectUrl, "https://github.com/LingCore/zhiji");

  document.getElementById("dialogConfirm").click();
  await new Promise((resolve) => setTimeout(resolve, 0));
  assert.equal(dialog.classList.contains("open"), false, "确认后关于弹窗应关闭");
});

check("亮色主题使用中性灰白界面且状态颜色可读", () => {
  const lightCss = css.slice(css.indexOf(':root[data-theme="light"]'));
  assert.ok(!css.includes("#fff8ed"), "亮色主题不应再使用偏暖米色背景");
  assert.ok(css.includes("--bg: #f4f4f5;"), "亮色主题背景 token 应使用中性灰");
  assert.ok(css.includes("--custom: #3f3f46;"), "亮色自定义状态应使用中性灰");
  assert.ok(
    css.includes("linear-gradient(135deg, #f3f4f6 0%, #ffffff 52%, #f4f4f5 100%)"),
    "亮色背景应使用中性灰白渐变"
  );
  assert.ok(css.includes(":root[data-theme=\"light\"] .restore-button {\n  background: #f4f4f5;"), "恢复按钮应使用中性灰");
  assert.ok(css.includes(":root[data-theme=\"light\"] .nav-item.active {\n  background: #e4e4e7;"), "亮色导航选中态应使用中性灰");
  assert.ok(css.includes(":root[data-theme=\"light\"] .state-group-title,\n:root[data-theme=\"light\"] .health-score span,\n:root[data-theme=\"light\"] .feature-state {\n  color: #27272a;"), "亮色标题和关键数值不应到处使用蓝色");
  assert.ok(css.includes(":root[data-theme=\"light\"] .state-group {\n  border-color: #e4e4e7;"), "亮色分组容器描边应更轻");
  assert.ok(css.includes(":root[data-theme=\"light\"] .requirement-row {\n  border-color: #e4e4e7;"), "亮色功能条目不能继承暗色深描边");
  assert.ok(css.includes(":root[data-theme=\"light\"] .requirement-row:hover {\n  border-color: #d4d4d8;"), "亮色功能条目 hover 才轻微加深");
  assert.ok(css.includes(":root[data-theme=\"light\"] .health-panel:hover,\n:root[data-theme=\"light\"] .metric-card:hover,\n:root[data-theme=\"light\"] .panel:hover {\n  border-color: #d4d4d8;"), "亮色主卡片 hover 描边不能继承暗色深灰");
  assert.ok(css.includes(":root[data-theme=\"light\"] .formula-option:hover,\n:root[data-theme=\"light\"] .dump-row:hover {\n  border-color: #d4d4d8;"), "亮色列表卡片 hover 描边应保持浅灰");
  assert.ok(!css.includes(":root[data-theme=\"light\"] .panel:hover {\n  border-color: #4c4c4c;"), "亮色 panel hover 不应使用暗色深描边");
  assert.ok(css.includes(":root[data-theme=\"light\"] .status-pill.state-off {\n  background: #fef2f2;\n  border-color: #fee2e2;"), "关闭状态色块边线应柔和");
  assert.ok(css.includes(":root[data-theme=\"light\"] .status-pill.state-on,\n:root[data-theme=\"light\"] .status-pill.state-managed {\n  background: #ecfdf5;\n  border-color: #bbf7d0;"), "开启状态色块边线应柔和");

  for (const forbidden of [
    "#eaf2fb",
    "#e8f0ff",
    "#dbe7ff",
    "#edf4fb",
    "#f3f8fc",
    "#f5f3ff",
    "#6d28d9",
    "rgba(37, 99, 235",
    "rgba(14, 165, 233"
  ]) {
    assert.ok(!lightCss.includes(forbidden), `亮色主题不应残留过多蓝/紫底色: ${forbidden}`);
  }

  const pairs = [
    ["#047857", "#ecfdf5", "开启状态"],
    ["#b91c1c", "#fef2f2", "关闭状态"],
    ["#1d4ed8", "#eff6ff", "Auto 状态"],
    ["#3f3f46", "#f4f4f5", "自定义状态"],
    ["#92400e", "#fffbeb", "未知状态"]
  ];
  for (const [foreground, background, label] of pairs) {
    assert.ok(
      contrastRatio(foreground, background) >= 4.5,
      `${label} 文本和背景对比度应满足 WCAG AA`
    );
  }
});

check("暗色主题主色调为中性黑白灰且不含蓝色强调", () => {
  const lightOverridesStart = css.indexOf(':root[data-theme="light"] body');
  const baseThemeCss = css.slice(0, lightOverridesStart);
  const darkCss = baseThemeCss.replace(/:root\[data-theme="light"\]\s*\{[\s\S]*?\n\}/, "");

  for (const token of [
    "--bg: #080808;",
    "--panel: #181818;",
    "--line: #333333;",
    "--text: #f2f2f2;",
    "--accent: #f5f5f5;",
    "--accent-strong: #d4d4d4;",
    "--custom: #bdbdbd;"
  ]) {
    assert.ok(darkCss.includes(token), `暗色主题缺少灰阶 token: ${token}`);
  }

  for (const forbidden of [
    "#38bdf8",
    "#0ea5e9",
    "#7dd3fc",
    "#e0f2fe",
    "#14b8a6",
    "rgba(56, 189, 248",
    "rgba(20, 184, 166",
    "rgba(12, 17, 23",
    "rgba(18, 25, 33",
    "rgba(21, 28, 36",
    "#101820",
    "#0e151c",
    "#151c24",
    "#18212b",
    "#202b37",
    "#1d2833"
  ]) {
    assert.ok(!darkCss.includes(forbidden), `暗色主题不应残留蓝色/蓝黑主色: ${forbidden}`);
  }

  assert.ok(
    darkCss.includes("linear-gradient(135deg, #050505 0%, #101010 52%, #171717 100%)"),
    "暗色背景应是纯灰阶渐变"
  );
  assert.ok(darkCss.includes(".primary-button {\n  background: #f5f5f5;"), "暗色主按钮应使用白色主色");
  assert.ok(
    darkCss.includes(".status-pill.state-auto {\n  background: var(--accent-soft);"),
    "Auto 状态应使用灰阶强调"
  );
  assert.ok(darkCss.includes(".sidebar {\n  background: #0b0b0b;"), "侧栏背景应是灰阶黑色");
  assert.ok(darkCss.includes(".workspace-header {\n  align-items: center;\n  background: #111111;"), "顶部栏背景应是灰阶黑色");
  assert.match(
    darkCss,
    /\.health-panel,\s*\.metric-card,\s*\.panel\s*\{[\s\S]*background: #181818;/,
    "暗色主卡片背景应是中性灰，不应是蓝黑"
  );
  assert.ok(darkCss.includes(".control-table {\n  background: #181818;"), "控制表背景应是中性灰");
  assert.ok(darkCss.includes(".checks-table {\n  background: #181818;"), "检查表背景应是中性灰");
  assert.equal(document.querySelector("#overviewPane .state-matrix-panel .subtle").textContent, "绿色/灰色代表可用或托管，红色代表关闭，琥珀代表未知。");

  const allowedSemanticHex = new Set([
    "#10b981",
    "#f59e0b",
    "#ef4444",
    "#6ee7b7",
    "#fca5a5",
    "#fbbf24",
    "#34d399",
    "#fb7185",
    "#fde68a",
    "#f8cfcf"
  ]);
  const isGrayHex = (hex) => {
    const normalized = hex.toLowerCase();
    const red = normalized.slice(1, 3);
    const green = normalized.slice(3, 5);
    const blue = normalized.slice(5, 7);
    return red === green && green === blue;
  };
  const nonGrayHex = [...new Set([...darkCss.matchAll(/#[0-9a-fA-F]{6}/g)].map((match) => match[0].toLowerCase()))]
    .filter((hex) => !isGrayHex(hex) && !allowedSemanticHex.has(hex));
  assert.deepEqual(nonGrayHex, [], `暗色主题除状态色外不应使用非灰阶 hex: ${nonGrayHex.join(", ")}`);

  const allowedSemanticRgb = new Set(["16,185,129", "245,158,11", "239,68,68", "0,0,0", "255,255,255"]);
  const nonGrayRgb = [...new Set([...darkCss.matchAll(/rgba?\(([^)]+)\)/g)].map((match) => match[1]))]
    .map((value) => value.split(",").slice(0, 3).map((part) => Number(part.trim())))
    .filter(([red, green, blue]) => !(red === green && green === blue))
    .map((channels) => channels.join(","))
    .filter((channels) => !allowedSemanticRgb.has(channels));
  assert.deepEqual(nonGrayRgb, [], `暗色主题除状态色外不应使用非灰阶 rgb: ${nonGrayRgb.join(", ")}`);
});

check("模块间距由全局 spacing token 管理", () => {
  for (const token of [
    "--space-0: 0;",
    "--space-1: 2px;",
    "--space-2: 4px;",
    "--space-3: 6px;",
    "--space-4: 8px;",
    "--space-5: 10px;",
    "--space-6: 12px;",
    "--space-7: 14px;",
    "--space-8: 16px;",
    "--space-9: 18px;",
    "--space-10: 20px;",
    "--space-12: 24px;",
    "--space-16: 32px;",
    "--module-padding: var(--space-7);",
    "--module-gap: var(--space-6);"
  ]) {
    assert.ok(css.includes(token), `缺少 spacing token: ${token}`);
  }

  assert.match(
    css,
    /\.health-panel,\s*\.metric-card,\s*\.panel\s*\{[\s\S]*padding: var\(--module-padding\);/,
    "普通 .panel 必须有默认模块内边距，不能再出现截图里的贴边卡片"
  );
  assert.ok(css.includes(".state-matrix-panel {\n  padding: var(--module-padding);"), "状态矩阵应复用模块内边距");
  assert.ok(css.includes(".vm-status-panel,\n.vm-recommend-panel,\n.bsod-action-panel {\n  min-height: 0;\n  overflow: hidden;\n  padding: var(--module-padding);"), "主要功能面板应复用模块内边距");
});

check("系统开关列表按来源分为 3 组（多列容器）", () => {
  const sections = [...document.querySelectorAll("#stateList > .state-group")];
  assert.equal(sections.length, 3, "#stateList 下应是 3 个 .state-group 容器");
  const heads = sections.map((s) => s.querySelector(".state-group-title").textContent);
  assert.deepEqual(heads, ["固件设置（BIOS / UEFI）", "Windows 功能", "启动与内存"]);
});

check("每组条目数量与来源一致", () => {
  const groups = {};
  for (const section of document.querySelectorAll("#stateList > .state-group")) {
    const name = section.querySelector(".state-group-title").textContent;
    groups[name] = [...section.querySelectorAll(".requirement-title")].map((n) => n.textContent);
  }
  assert.deepEqual(groups["固件设置（BIOS / UEFI）"], [
    "CPU 虚拟化（VT-x / SVM）",
    "设备直通（VT-d / IOMMU）",
    "兼容启动（CSM / Legacy Boot）",
    "安全启动（Secure Boot）",
    "可信平台模块（TPM / Intel PTT / AMD fTPM）"
  ]);
  assert.deepEqual(groups["Windows 功能"], [
    "Hyper-V（微软虚拟机监控程序）",
    "虚拟机平台（Virtual Machine Platform）",
    "Windows 虚拟机监控平台（Windows Hypervisor Platform）"
  ]);
  assert.deepEqual(groups["启动与内存"], [
    "虚拟机监控程序启动项（hypervisorlaunchtype）",
    "快速启动（Windows Fast Startup）",
    "内存压缩（Memory Compression）",
    "虚拟内存"
  ]);
});

check("条目总数包含新增内存压缩项（12 项，无丢失）", () => {
  assert.equal(document.querySelectorAll("#stateList .requirement-row").length, 12);
});

check("每行带状态标签且渲染了 preview 数据", () => {
  const rows = [...document.querySelectorAll("#stateList .requirement-row")];
  for (const row of rows) {
    const pill = row.querySelector(".status-pill");
    assert.ok(pill && pill.textContent.trim().length > 0, "状态 pill 不应为空");
  }
  const hyperVRow = rows.find((r) => r.querySelector(".requirement-title").textContent === "Hyper-V（微软虚拟机监控程序）");
  assert.equal(hyperVRow.querySelector(".status-pill").textContent, "开启");
});

check("单项控制的当前状态注释仍正常渲染", () => {
  const note = document.querySelector('[data-state-for="hyper_v"]');
  assert.ok(note.textContent.includes("开启"), `期望包含“开启”，实际：${note.textContent}`);
  const memoryCompressionNote = document.querySelector('[data-state-for="memory_compression"]');
  assert.ok(
    memoryCompressionNote.textContent.includes("开启"),
    `期望包含“开启”，实际：${memoryCompressionNote.textContent}`
  );
});

check("关键英文系统术语都有中文解释", () => {
  const controlTitles = [...document.querySelectorAll(".control-title")].map((node) => node.textContent);
  for (const label of [
    "CPU 虚拟化（VT-x / SVM）",
    "设备直通（VT-d / IOMMU）",
    "兼容启动（CSM / Legacy Boot）",
    "安全启动（Secure Boot）",
    "可信平台模块（TPM / Intel PTT / AMD fTPM）",
    "Hyper-V（微软虚拟机监控程序）",
    "虚拟机平台（Virtual Machine Platform）",
    "Windows 虚拟机监控平台（Windows Hypervisor Platform）",
    "虚拟机监控程序启动项（hypervisorlaunchtype）",
    "内存压缩（Memory Compression）"
  ]) {
    assert.ok(controlTitles.includes(label), `缺少双语控制标题：${label}`);
  }
  assert.equal(document.querySelector("#memoryPane h2").textContent, "页面文件（pagefile.sys）当前配置");
  assert.equal(document.querySelector("#bsodPane h2").textContent, "转储文件（DMP）收集状态");
  assert.equal(document.querySelector("#openBlueScreenViewButton").textContent, "打开蓝屏查看器（BlueScreenView）");
  assert.equal(document.querySelector('[data-action="set_hypervisor_auto"]').textContent, "自动（Auto）");
  assert.equal(document.querySelector('[data-action="set_hypervisor_off"]').textContent, "关闭（Off）");
  assert.equal(document.querySelector("#hardwarePane .label").textContent, "主板");
  assert.equal(document.querySelectorAll("#hardwarePane .label")[1].textContent, "固件版本（BIOS）");
  assert.match(
    css,
    /\.control-title\s*\{[\s\S]*line-height: 1\.3;[\s\S]*overflow-wrap: anywhere;/,
    "双语标题应允许换行，避免显示不全"
  );
  assert.match(
    css,
    /\.requirement-title\s*\{[\s\S]*line-height: 1\.3;[\s\S]*overflow-wrap: anywhere;/,
    "状态矩阵双语标题应允许换行"
  );
});

check("分段按钮能容纳双语短标签", () => {
  const hypervisorMode = document.querySelector('[data-action="set_hypervisor_auto"]').closest(".segmented");
  assert.equal(hypervisorMode.children.length, 2);
  assert.match(
    css,
    /\.segmented\s*\{[\s\S]*grid-template-columns: repeat\(2, minmax\(96px, 1fr\)\);[\s\S]*min-width: 212px;/,
    "分段按钮默认列宽应能容纳“自动（Auto）/关闭（Off）”"
  );
  assert.match(
    css,
    /\.action-button\s*\{[\s\S]*min-width: 0;[\s\S]*overflow-wrap: anywhere;[\s\S]*white-space: normal;/,
    "按钮文字应允许在极窄空间换行，不能溢出按钮"
  );
  assert.match(
    css,
    /\.segmented\s*\{[\s\S]*justify-self: stretch;[\s\S]*min-width: 0;[\s\S]*grid-template-columns: repeat\(2, minmax\(0, 1fr\)\);/,
    "窄屏下分段按钮应跟随容器宽度收缩"
  );
});

check("状态矩阵的来源和值不再使用纯英文系统参数", () => {
  const rows = [...document.querySelectorAll("#stateList .requirement-row")];
  const sourceByName = Object.fromEntries(
    rows.map((row) => [
      row.querySelector(".requirement-title").textContent,
      row.querySelector(".requirement-value").textContent
    ])
  );
  assert.equal(sourceByName["CPU 虚拟化（VT-x / SVM）"], "固件（BIOS/UEFI）");
  assert.equal(sourceByName["虚拟机监控程序启动项（hypervisorlaunchtype）"], "启动配置（BCD）");
  assert.equal(sourceByName["快速启动（Windows Fast Startup）"], "快速启动注册表（Hiberboot）");
  assert.equal(sourceByName["内存压缩（Memory Compression）"], "内存管理（Memory Management）");
  assert.equal(sourceByName["虚拟内存"], "C: 页面文件（pagefile.sys）");
  assert.ok(
    rows.some((row) => row.querySelector(".status-pill").textContent === "自动（Auto）"),
    "Auto 状态应显示为中文+英文"
  );
  assert.equal(document.querySelector("#adminStatus").textContent, "管理员：是（Yes）");
});

check("分段开关高亮当前激活的一侧", () => {
  // preview 数据：hyper_v=Enabled, vmp=Disabled, whp=Disabled, hypervisor_launch=Auto, fast_startup=Enabled, memory_compression=Enabled
  const expectations = [
    ["enable_hyper_v", "state-on", true],
    ["disable_hyper_v", null, false],
    ["disable_virtual_machine_platform", "state-off", true],
    ["enable_virtual_machine_platform", null, false],
    ["disable_windows_hypervisor_platform", "state-off", true],
    ["set_hypervisor_auto", "state-auto", true],
    ["set_hypervisor_off", null, false],
    ["enable_fast_startup", "state-on", true],
    ["disable_fast_startup", null, false],
    ["enable_memory_compression", "state-on", true],
    ["disable_memory_compression", null, false]
  ];
  for (const [action, stateClass, shouldBeActive] of expectations) {
    const button = document.querySelector(`[data-action="${action}"]`);
    assert.ok(button, `找不到按钮 ${action}`);
    assert.equal(
      button.classList.contains("is-active"),
      shouldBeActive,
      `${action} 的 is-active 应为 ${shouldBeActive}`
    );
    if (stateClass) {
      assert.ok(button.classList.contains(stateClass), `${action} 应有 ${stateClass}`);
    }
  }
});

check("预设达标判定已删除（无 overallStatus 徽章）", () => {
  assert.equal(document.getElementById("overallStatus"), null, "#overallStatus 不应存在");
});

check("状态计数卡片仍正常（中性统计）", () => {
  // preview 数据：on=5(CPU虚拟化/VT-d/Hyper-V/快速启动/内存压缩) + auto(Hypervisor) + managed(虚拟内存) 计入开启=7，
  // off=4(CSM→UEFI 显示为 off 样式之外，VMP/WHP/SecureBoot/TPM)，具体值以渲染结果为准，只验证总和=12
  const on = Number(document.getElementById("onCount").textContent);
  const off = Number(document.getElementById("offCount").textContent);
  const custom = Number(document.getElementById("customCount").textContent);
  const unknown = Number(document.getElementById("unknownCount").textContent);
  for (const value of [on, off, custom, unknown]) {
    assert.ok(Number.isFinite(value), "计数应为数字");
  }
  assert.equal(on + off + custom + unknown, 12, "四类计数总和应为 12 项");
});

check("虚拟内存：设定值与运行值合并为一张卡片", () => {
  const cards = document.querySelectorAll("#pagefileList .pagefile-card");
  assert.equal(cards.length, 1, "preview 只有 C:\\pagefile.sys，应只有一张卡片");
  const card = cards[0];
  assert.equal(card.querySelectorAll(".pagefile-name").length, 1, "文件名只出现一次");
  const labels = [...card.querySelectorAll(".pagefile-section-label")].map((n) => n.textContent);
  assert.deepEqual(labels, ["设定值", "运行中"]);
  assert.ok(card.textContent.includes("系统托管"), "preview 配置为系统托管");
  assert.ok(card.textContent.includes("312 MB"), "使用中应显示 312 MB");
  assert.equal(card.querySelector(".pagefile-flag"), null, "设定与运行都正常时不应有提示标签");
});

check("运行值为 0 显示 0 MB 而不是 -", () => {
  const cardHtml = dom.window.eval(`buildPagefileCard({
    name: "C:\\\\pagefile.sys",
    config: { initial_size_mb: 1024, maximum_size_mb: 2048 },
    usage: { allocated_base_size_mb: 1024, current_usage_mb: 0, peak_usage_mb: 0, temp_page_file: false }
  }).outerHTML`);
  assert.ok(cardHtml.includes("0 MB"), "0 应显示为 0 MB");
  assert.ok(!cardHtml.includes("<strong>-</strong>"), "不应出现 - 占位");
});

check("仅有设定值时显示“重启后生效”标签", () => {
  const cardHtml = dom.window.eval(`buildPagefileCard({
    name: "D:\\\\pagefile.sys",
    config: { initial_size_mb: 4096, maximum_size_mb: 8192 },
    usage: null
  }).outerHTML`);
  assert.ok(cardHtml.includes("新设置重启后生效"));
  assert.ok(cardHtml.includes("未运行"));
});

check("仅有运行值时显示“重启后停用”标签", () => {
  const cardHtml = dom.window.eval(`buildPagefileCard({
    name: "E:\\\\pagefile.sys",
    config: null,
    usage: { allocated_base_size_mb: 2048, current_usage_mb: 100, peak_usage_mb: 300, temp_page_file: false }
  }).outerHTML`);
  assert.ok(cardHtml.includes("设置已删除，重启后停用"));
  assert.ok(cardHtml.includes("未配置"));
});

check("“执行所选方案”按钮已删除，只剩一个应用按钮", () => {
  assert.equal(document.getElementById("applyRecommendedPagefile"), null, "#applyRecommendedPagefile 不应存在");
  const buttons = document.querySelectorAll(".vm-actions button");
  assert.equal(buttons.length, 1, "vm-actions 应只有一个按钮");
});

check("系统托管方案下按钮可用且文案为“启用系统托管”", () => {
  // preview 推荐方案是系统托管，且默认选中
  const button = document.getElementById("applyCustomPagefile");
  assert.equal(button.textContent, "启用系统托管");
  assert.equal(button.disabled, false, "按钮不应被禁用");
  assert.ok(button.title.includes("系统托管"), "title 应解释行为");
});

check("虚拟内存输入框占位文案紧凑且字号更小", () => {
  assert.equal(document.getElementById("pagefileInitialInput").placeholder, "无需填写");
  assert.equal(document.getElementById("pagefileMaximumInput").placeholder, "无需填写");
  assert.ok(css.includes(".vm-input-grid input {\n  background: #101010;"), "应存在虚拟内存输入框样式块");
  assert.match(
    css,
    /\.vm-input-grid input\s*\{[\s\S]*font-size: 12px;[\s\S]*font-weight: 700;[\s\S]*text-overflow: ellipsis;/,
    "输入框文字应更小且允许省略"
  );
  assert.match(
    css,
    /\.vm-input-grid input::placeholder\s*\{[\s\S]*font-size: 11px;[\s\S]*font-weight: 650;/,
    "placeholder 应比输入值更小"
  );
});

await checkAsync("高风险操作弹窗必须展示后果清单并勾选确认", async () => {
  const restart = document.querySelector('[data-action="restart_windows"]');
  restart.click();
  await new Promise((resolve) => setTimeout(resolve, 0));

  const dialog = document.getElementById("appDialog");
  const confirm = document.getElementById("dialogConfirm");
  const acknowledge = document.getElementById("dialogAcknowledge");

  assert.ok(dialog.classList.contains("open"), "弹窗应打开");
  assert.ok(dialog.classList.contains("danger"), "立即重启应使用 danger 变体");
  assert.equal(document.getElementById("dialogTitle").textContent, "确认立即重启电脑？");
  assert.ok(document.querySelectorAll("#dialogRiskList li").length >= 2, "应列出至少两条后果");
  assert.equal(document.getElementById("dialogAcknowledgeWrap").hidden, false, "应显示风险确认勾选");
  assert.equal(confirm.disabled, true, "未勾选前确认按钮应禁用");

  acknowledge.checked = true;
  acknowledge.dispatchEvent(new dom.window.Event("change", { bubbles: true }));
  assert.equal(confirm.disabled, false, "勾选后确认按钮应可用");

  document.getElementById("dialogCancel").click();
  await new Promise((resolve) => setTimeout(resolve, 0));
  assert.equal(dialog.classList.contains("open"), false, "取消后弹窗应关闭");
});

check("显示器身份面板默认呈现推荐流程并折叠高级工具", () => {
  const pane = document.getElementById("monitorIdentityPane");
  assert.ok(pane, "应存在显示器身份面板");
  assert.ok(document.querySelector('[data-view="monitorIdentity"]'), "应有显示器身份导航入口");
  assert.ok(document.getElementById("monitorIdentitySelect"), "应有显示器选择器");
  assert.ok(document.getElementById("monitorManufacturerInput"), "应有 Manufacturer ID 输入");
  assert.ok(document.getElementById("monitorProductInput"), "应有 Product Code 输入");
  assert.equal(document.getElementById("monitorIdentityInstallInfButton").textContent, "应用修改");
  assert.ok(document.querySelector(".monitor-identity-advanced"), "应把高风险工具折叠进高级工具");
  assert.ok(document.getElementById("monitorIdentityApplyButton"), "高级工具应有仅写注册表按钮");
  assert.ok(document.getElementById("monitorIdentityReenumerateButton"), "高级工具应有强制重枚举按钮");
  assert.ok(document.getElementById("monitorIdentityConfirmButton"), "应有保留更改按钮");
  assert.ok(document.getElementById("monitorIdentityRollbackButton"), "应有回滚按钮");
  assert.equal(pane.textContent.includes("自动保护"), true);
  assert.ok(js.includes("monitor_identity_apply_override"), "前端应调用显示器身份应用命令");
  assert.ok(js.includes("monitor_identity_install_inf_override"), "前端应调用显示器身份 INF 安装命令");
  assert.ok(js.includes("monitor_identity_reenumerate_device"), "前端应调用显示器强制重枚举命令");
  assert.ok(js.includes("monitor_identity_confirm_override"), "前端应调用显示器身份确认命令");
  assert.ok(js.includes("monitor_identity_restore_change"), "前端应调用显示器身份还原命令");
});

await checkAsync("显示器身份预览应用后进入待确认并可保留", async () => {
  document.querySelector('[data-view="monitorIdentity"]').click();
  await new Promise((resolve) => setTimeout(resolve, 30));

  assert.equal(document.getElementById("monitorIdentitySelect").options.length, 1);
  assert.equal(document.getElementById("monitorManufacturerInput").value, "LHC");
  assert.equal(document.getElementById("monitorProductInput").value, "906A");

  document.getElementById("monitorManufacturerInput").value = "DEL";
  document.getElementById("monitorProductInput").value = "A123";
  document.getElementById("monitorSerialInput").value = "SN123456";
  document.getElementById("monitorNameInput").value = "FAKE PANEL";
  document.getElementById("monitorIdentityInstallInfButton").click();
  await new Promise((resolve) => setTimeout(resolve, 0));

  const acknowledge = document.getElementById("dialogAcknowledge");
  assert.equal(document.getElementById("dialogTitle").textContent, "确认应用显示器身份修改？");
  acknowledge.checked = true;
  acknowledge.dispatchEvent(new dom.window.Event("change", { bubbles: true }));
  document.getElementById("dialogConfirm").click();
  await new Promise((resolve) => setTimeout(resolve, 50));

  assert.match(document.getElementById("monitorIdentityPendingState").textContent, /待确认：\d+s/);
  assert.equal(document.getElementById("monitorIdentityConfirmButton").disabled, false);

  document.getElementById("monitorIdentityConfirmButton").click();
  await new Promise((resolve) => setTimeout(resolve, 50));
  assert.equal(document.getElementById("monitorIdentityConfirmButton").disabled, true);
});

await checkAsync("显示器身份推荐流程预览后进入待确认并记录驱动包", async () => {
  document.querySelector('[data-view="monitorIdentity"]').click();
  await new Promise((resolve) => setTimeout(resolve, 30));

  document.getElementById("monitorManufacturerInput").value = "BUX";
  document.getElementById("monitorProductInput").value = "0F04";
  document.getElementById("monitorSerialInput").value = "SN22A7F2E5D4";
  document.getElementById("monitorNameInput").value = "DSP-E70E077D";
  document.getElementById("monitorIdentityInstallInfButton").click();
  await new Promise((resolve) => setTimeout(resolve, 0));

  assert.equal(document.getElementById("dialogTitle").textContent, "确认应用显示器身份修改？");
  const acknowledge = document.getElementById("dialogAcknowledge");
  acknowledge.checked = true;
  acknowledge.dispatchEvent(new dom.window.Event("change", { bubbles: true }));
  document.getElementById("dialogConfirm").click();
  await new Promise((resolve) => setTimeout(resolve, 50));

  assert.match(document.getElementById("monitorIdentityPendingState").textContent, /待确认：\d+s/);
  assert.equal(document.getElementById("monitorIdentityLog").textContent.includes("INF"), true);
  assert.equal(document.getElementById("monitorIdentityLog").textContent.includes("oem42.inf"), true);

  document.getElementById("monitorIdentityConfirmButton").click();
  await new Promise((resolve) => setTimeout(resolve, 50));
});

await checkAsync("显示器身份强制重枚举预览进入待确认并说明回滚", async () => {
  document.querySelector('[data-view="monitorIdentity"]').click();
  await new Promise((resolve) => setTimeout(resolve, 30));

  document.getElementById("monitorIdentityReenumerateButton").click();
  await new Promise((resolve) => setTimeout(resolve, 0));

  assert.equal(document.getElementById("dialogTitle").textContent, "确认强制重枚举显示器？");
  assert.equal(document.getElementById("dialogBody").textContent.includes("pnputil /remove-device"), true);
  const acknowledge = document.getElementById("dialogAcknowledge");
  acknowledge.checked = true;
  acknowledge.dispatchEvent(new dom.window.Event("change", { bubbles: true }));
  document.getElementById("dialogConfirm").click();
  await new Promise((resolve) => setTimeout(resolve, 50));

  assert.match(document.getElementById("monitorIdentityPendingState").textContent, /待确认：\d+s/);
  assert.equal(document.getElementById("monitorIdentityLog").textContent.includes("重枚举"), true);

  document.getElementById("monitorIdentityConfirmButton").click();
  await new Promise((resolve) => setTimeout(resolve, 50));
});

await checkAsync("显示器身份一键随机只填字段且不立即写入系统", async () => {
  document.querySelector('[data-view="monitorIdentity"]').click();
  await new Promise((resolve) => setTimeout(resolve, 30));

  assert.ok(document.getElementById("monitorIdentityRandomButton"), "应有一键随机按钮");
  document.getElementById("monitorIdentityRandomButton").click();

  assert.match(document.getElementById("monitorManufacturerInput").value, /^[A-Z]{3}$/);
  assert.match(document.getElementById("monitorProductInput").value, /^[0-9A-F]{4}$/);
  assert.ok(Number(document.getElementById("monitorNumericSerialInput").value) >= 1);
  assert.match(document.getElementById("monitorSerialInput").value, /^SN[0-9A-F]{1,11}$/);
  assert.match(document.getElementById("monitorNameInput").value, /^DSP-[0-9A-F]{1,8}$/);
  assert.equal(document.getElementById("monitorIdentityMessage").textContent.includes("尚未写入系统"), true);
});

if (failures.length > 0) {
  console.error(`\n${failures.length} 项失败`);
  process.exit(1);
}
console.log("\n全部通过");
