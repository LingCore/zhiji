# 知机

A small Tauri + Rust desktop app for checking the Windows requirements used by this PC setup:

- CPU virtualization enabled in firmware (Intel VT-x / AMD SVM)
- Microsoft Hyper-V enabled
- Secure Boot disabled
- TPM disabled or not visible to Windows
- BCD `hypervisorlaunchtype` set to `Auto`

The app checks settings and exposes separate controls for each setting:

- Enable or disable Hyper-V
- Enable or disable Virtual Machine Platform
- Enable or disable Windows Hypervisor Platform
- Set `hypervisorlaunchtype` to `auto` or `off`
- Restart Windows
- Restart into BIOS/UEFI

Firmware-only settings such as VT-x, Secure Boot, and TPM usually cannot be changed directly by a Windows desktop app. The app provides a BIOS restart button and guidance for those.

## Run

Install prerequisites first:

- Rust stable toolchain
- Node.js/npm, if you want to use the npm Tauri CLI workflow
- Microsoft Edge WebView2 Runtime, usually already included on Windows 10/11

Then run:

```powershell
cd "C:\Users\bob\Documents\Codex\2026-05-26\ensure-your-pc-meets-the-requirements\pc-requirements-tauri"
npm install
npm run tauri dev
```

If you prefer Cargo:

```powershell
cargo install tauri-cli --version "^2"
cargo tauri dev
```

## Build

```powershell
npm run tauri build
```

The generated installer/bundle will be under `src-tauri\target\release\bundle`.

## Repository

GitHub: https://github.com/LingCore/zhiji

## Gaming Optimizer

Open `竞技模式` to apply low-risk competitive FPS optimizations that are separate from power plans:

- Enable HAGS (`HwSchMode=2`, administrator + reboot required).
- Disable Xbox Game Bar / Game DVR capture for the current user.
- Disable fullscreen optimizations for a selected game executable while preserving existing compatibility flags.
- Game Mode is kept as an experimental per-machine toggle and is not part of the default safe preset.
- Every registry write is recorded so the app can restore its own changes later.

Current local environment installed on this PC:

- Node.js/npm: `C:\Tools\nodejs`
- Rust/Cargo: `C:\Users\bob\.cargo`, `C:\Users\bob\.rustup`
- Built executable: `src-tauri\target\release\Zhiji.exe`
- NSIS installer: `src-tauri\target\release\bundle\nsis\知机_0.1.0_x64-setup.exe`

## Notes

- Run the app as Administrator for the most reliable Hyper-V, Secure Boot, TPM, and BCD results.
- Once Hyper-V is running, some Windows CPU virtualization fields may report `False`; the app treats an already-running hypervisor as proof that hardware virtualization is available and in use.
- To enter BIOS on the current GIGABYTE board, restart and press `Delete`; use `F2` to switch to Advanced Mode if needed.

## License

The project source code is released under the MIT License. See `LICENSE`.

Bundled third-party tools are listed in `THIRD_PARTY_NOTICES.md`.
