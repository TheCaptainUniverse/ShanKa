# Shanka 闪改

[English](./README.en.md)

闪改是一个基于 Tauri + Vue + Bun 的系统级 AI 文本改写桌面应用。
它默认本地优先：设置、人格、快捷键和可选改写历史保存在本机，API Key
保存在系统密钥链中，只有当你主动触发改写时，选中文本才会发送到你配置的
AI 服务。

闪改使用 MIT License 开源。你可以商用、修改、分发或私有使用，但需要保留
版权声明和许可证文本。

## 使用方式

闪改在后台运行，可以改写其他桌面应用里的选中文本。首次启动默认使用中文界面，
也可以在设置中切换语言。重复启动闪改会激活已有实例，而不会启动第二个后台进程。

默认快捷键：

- 安心模式：在鼠标上方生成可编辑的差异预览。你可以查看改动、切换到结果文本、
  复制、替换，或者换一种人格重新生成。
- 闪改模式：直接改写并替换当前选中文本，适合确定要快速处理的场景。

快捷键可以在设置中修改。录制快捷键时，闪改会暂停安心/闪改模式触发，避免录制
过程中误改文本。

## 首次设置

1. 从应用窗口或系统托盘打开闪改。
2. 选择 DeepSeek、OpenAI、OpenRouter 等服务商预设。
3. 填写 API Key、Base URL 和模型，然后运行连接测试。
4. 如果希望随系统启动，可以开启开机启动。
5. 保存设置后，在任意桌面应用中选中文本。
6. 建议先触发安心模式，确认预览流程正常后再使用闪改模式。

API Key 会尽量保存到系统密钥链。应用配置只保存密钥引用，不保存明文 Key。

## 构建发布包

推荐使用 GitHub Actions 生成跨平台发布包。推送版本 tag 后，
`.github/workflows/release.yml` 会在 Windows、Linux 和 macOS runner 上分别构建，
并把产物上传到 GitHub Draft Release：

```bash
git tag v0.1.0
git push origin v0.1.0
```

当前自动发布目标：

- Windows x64：MSI 和 NSIS 安装包。
- Linux x64：Tauri Linux bundle，具体格式由当前 Tauri bundler 输出决定。
- macOS Apple Silicon：`aarch64-apple-darwin` bundle。
- macOS Intel：`x86_64-apple-darwin` bundle。

发布前建议先在 Windows 本机运行完整预检：

```bash
bun run release:preflight
```

也可以只构建本机 Windows 包：

```bash
bun run tauri build
bun run release:smoke
bun run release:manual-text-smoke
bun run release:install-smoke
bun run release:msi-smoke
bun run release:manifest
```

当前 Windows 输出：

- `src-tauri/target/release/bundle/msi/Shanka_0.1.0_x64_en-US.msi`
- `src-tauri/target/release/bundle/nsis/Shanka_0.1.0_x64-setup.exe`

测试打包版本前，请关闭已有的 `shanka.exe` 进程。开发版或旧打包版可能已经占用了
全局快捷键。

如果要隔离手动 Windows 文本链路测试环境，可以用 `SHANKA_CONFIG_DIR` 指定临时
配置目录，避免污染日常配置：

```powershell
$env:SHANKA_CONFIG_DIR="$env:TEMP\ShankaManualConfig"
src-tauri\target\release\shanka.exe
```

测试结束后关闭闪改，再删除临时目录。

也可以启动带夹具的手动测试 profile：

```bash
bun run release:manual-text-test
```

它会使用 mock 改写设置启动闪改，打开 Notepad 和浏览器测试夹具，打印 Windows
文本链路检查需要使用的快捷键，并在 `docs/release/manual/` 下生成预填报告。

需要自动打开生成报告时：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\release-manual-text-test.ps1 -OpenReport
```

需要把闪改进程 stdout/stderr 保存到报告目录时：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\release-manual-text-test.ps1 -CaptureLog
```

## 开发

```bash
bun install
bun run dev
```

常用命令：

```bash
bun run dev:web
bun run dev:server
bun run dev:app
bun run check
bun run typecheck
bun run db:generate
bun run db:push
bun run tauri build
```

发布前建议运行：

```bash
bun run check
bun run release:preflight
```

跨平台发布使用版本 tag 触发 GitHub Actions：

```bash
git tag v0.1.0
git push origin v0.1.0
```

## 常见问题

- 未检测到选中文本：确认目标应用中存在有效文本选区，然后重新触发安心模式。
- 快捷键注册失败：可能被系统或其他应用占用。请在设置中录制其他组合键。
- 粘贴替换失败：如果 Windows 目标应用以管理员权限运行，闪改会把结果保存到剪贴板。
- macOS 输入不可用：在系统设置中为闪改开启辅助功能权限。
- Linux 行为取决于桌面会话。X11 是当前优先验证路径；Wayland 可能限制全局快捷键和模拟输入。

## 本地优先与隐私

闪改默认把配置、人格、快捷键和改写历史保存在本机。API Key 会尽量通过系统密钥链
保存，配置文件只保留密钥引用。

选中文本只会在你触发安心模式或闪改模式后发送，并且只发送到你配置的服务商端点。
改写历史可以在设置中关闭或清空。

默认日志会避免输出完整选中文本、服务商响应正文和完整 API Key。排查剪贴板或服务商
问题时，可以临时在设置中开启调试日志。

## 开源协议

闪改使用 MIT License。你可以使用、复制、修改、合并、发布、分发、再许可和销售
本软件的副本，但需要在所有副本或实质性部分中包含版权声明和许可证文本。

完整条款见 [`LICENSE`](./LICENSE)。

## 项目架构

- `src/`：Vue 3 设置面板和 HUD 前端。
- `src-tauri/`：Rust 系统宿主，负责全局快捷键、剪贴板捕获、输入模拟、托盘、窗口、服务商调用、密钥链、本地历史和改写流程。
- `server/`：Bun + Hono 本地服务总线和 Drizzle SQLite schema，保留作开发实验；生产改写流程当前在 Rust 中运行，无需 sidecar。
- `shared/`：共享契约、事件、模式、HUD 状态和错误码。
