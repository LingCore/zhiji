# Zhiji

[中文](README.md)

Zhiji is a Tauri + Rust desktop utility for inspecting and adjusting key Windows settings, virtual memory, blue screen dump collection, competitive gaming optimizations, and hardware information.

![Zhiji overview](docs/images/overview.png)

## Features

- Inspect CPU virtualization, Hyper-V, Secure Boot, TPM, BCD, and related states.
- Enable or disable Hyper-V, Virtual Machine Platform, and Windows Hypervisor Platform independently.
- Adjust `hypervisorlaunchtype`, virtual memory, and small memory dump collection.
- Open or export BlueScreenView crash analysis results.
- Apply reversible, low-risk FPS-related optimizations in Competitive Mode.
- View hardware information and monitor identity details, with monitor identity testing tools.

Firmware settings such as VT-x / SVM, Secure Boot, and TPM usually cannot be changed directly by a Windows desktop app. Zhiji provides status guidance and an entry point to restart into BIOS/UEFI.

## Download

Most users should download the installer from GitHub Releases:

- GitHub Releases: https://github.com/LingCore/zhiji/releases

## Development

Install these prerequisites first:

- Rust stable toolchain
- Node.js/npm
- Microsoft Edge WebView2 Runtime, usually already included on Windows 10/11

```powershell
npm install
npm run tauri dev
```

## Build

```powershell
npm run tauri build
```

The generated installer is placed under:

```text
src-tauri\target\release\bundle\nsis\
```

## License

The project source code is released under the MIT License. See [LICENSE](LICENSE).

Bundled third-party tools are listed in [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
