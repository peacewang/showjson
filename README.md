# ShowJSON

ShowJSON 是一个本地优先的跨平台 JSON 快速查看器。

在浏览器、终端或任意应用中复制文本，然后按 `Cmd/Ctrl + Shift + J`，即可在快速窗口中查看格式化后的 JSON。

读取系统剪贴板只有这一种触发方式。点击托盘图标或托盘菜单只显示当前窗口，不会读取或识别剪贴板。

## 当前能力

- 全局快捷键和系统托盘。
- 窗口切换到其他应用时自动隐藏，不会持续遮挡其他界面。
- 本地持久化剪贴板历史，支持去重、删除和清空。
- 标准 JSON、整体转义 JSON、NDJSON/JSON Lines。
- 从服务端日志等混合文本中提取一个或多个 JSON 片段。
- Tree、Pretty、Raw 三种查看模式。
- 搜索 Key/Value、分层展开和折叠。
- 复制节点值、JSONPath 或完整格式化 JSON。
- 使用 `lossless-json` 保留超出 JavaScript 安全范围的整数。
- 所有内容仅在本地处理，不会在后台持续监听剪贴板。
- 解析失败时直接显示原文和错误位置；只在用户点击后尝试修复。
- 单实例运行；关闭窗口时隐藏到托盘，托盘菜单可彻底退出。

## 使用

1. 启动 ShowJSON。
2. 在任意应用中复制包含 JSON 的文本。
3. 按 `Cmd/Ctrl + Shift + J` 读取并识别剪贴板。
4. 查看完成后按 `Esc` 隐藏窗口，或直接切换到其他应用。

点击托盘图标只会重新显示当前界面，不会触发新的识别。

也可以在空白页中选择“直接粘贴文本”进行手动解析。

## 本地开发

### 环境要求

- Node.js 20+
- Rust stable
- macOS：Xcode Command Line Tools
- Windows：Microsoft C++ Build Tools、WebView2
- Linux：Tauri 2 所需的 WebKitGTK 4.1 与 AppIndicator 开发包

安装依赖：

```bash
npm install
```

运行桌面开发版本：

```bash
npm run tauri dev
```

运行检查和测试：

```bash
npm run check
npm test
cargo check --manifest-path src-tauri/Cargo.toml
```

构建当前平台安装包：

```bash
npm run tauri build
```

## 发布产物

GitHub Actions 在推送 `v*` Tag 后为以下平台构建草稿 Release：

- macOS Intel：DMG
- macOS Apple Silicon：DMG
- Windows x64：NSIS `setup.exe`
- Linux x64：AppImage 和 Deb

macOS 安装包正式公开分发前需要配置签名和 notarization；Windows 正式公开分发时建议配置代码签名。

第一次发布请按照 [GitHub Release 操作指南](docs/release-guide.md) 完成独立建仓、版本同步、Tag、Actions 和 Draft Release 验证。

## 项目结构

```text
src/
  lib/components/TreeNode.svelte  JSON 树节点
  lib/json/parser.ts              JSON 识别和日志片段提取
  lib/json/format.ts              格式化、统计、搜索和 JSONPath
  routes/+page.svelte             主界面
src-tauri/
  src/lib.rs                      快捷键、剪贴板、托盘和窗口管理
  tauri.conf.json                 桌面打包配置
```

## MVP 边界

当前版本是只读查看器，不包含 JSON 编辑、云同步和后台自动监听剪贴板。剪贴板历史最多保存 50 条、总计 25 MB，单条超过 5 MB 不记录；单次文本输入上限为 50 MB。
