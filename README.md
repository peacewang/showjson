# ShowJSON

[![CI](https://github.com/peacewang/showjson/actions/workflows/ci.yml/badge.svg)](https://github.com/peacewang/showjson/actions/workflows/ci.yml)
[![Release](https://github.com/peacewang/showjson/actions/workflows/release.yml/badge.svg)](https://github.com/peacewang/showjson/actions/workflows/release.yml)

ShowJSON 是一个轻量、本地优先的跨平台 JSON 快速查看器。

在浏览器、终端或任意应用中复制文本，按下 `Cmd/Ctrl + Shift + J`，即可立即查看结构化结果，不必再把内容粘贴到 IDE 或在线格式化网站。

[下载 ShowJSON](https://github.com/peacewang/showjson/releases)

## 主要功能

- 自动识别标准 JSON、转义 JSON、NDJSON/JSON Lines。
- 从服务端日志、告警消息等混合文本中提取 JSON。
- Tree、Pretty、Raw 三种查看模式。
- 搜索 Key/Value，展开或折叠节点。
- 复制节点值、JSONPath 或完整格式化结果。
- 保留超出 JavaScript 安全范围的大整数。
- 本地保存最近 50 条剪贴板历史，支持删除和清空。
- 解析失败时显示原文、错误位置和原因，由用户决定是否修复。
- 所有数据只在本机处理，不上传服务器，也不会持续监听剪贴板。

## 下载与安装

请前往 [Releases](https://github.com/peacewang/showjson/releases) 下载与你的系统对应的安装包：

| 系统 | 安装包 |
| --- | --- |
| macOS Intel | `darwin-x64.dmg` |
| macOS Apple Silicon | `darwin-aarch64.dmg` |
| Windows x64 | `windows-x64-setup.exe` |
| Linux x64 | `linux-amd64.AppImage` 或 `linux-amd64.deb` |

### macOS 首次打开

当前 macOS 安装包尚未使用 Apple Developer ID 签名和公证，因此第一次打开时，macOS 可能提示“无法验证开发者”或“Apple 无法检查其是否包含恶意软件”。

1. 打开 DMG，将 ShowJSON 拖入“应用程序”。
2. 尝试打开一次 ShowJSON。
3. 打开“系统设置/系统偏好设置” → “隐私与安全性/安全性与隐私”。
4. 在安全提示旁点击“仍要打开”，输入登录密码并确认。

确认一次后，后续可以像普通应用一样启动。请只从本仓库的 Releases 页面下载安装包。

### Linux AppImage

如果 AppImage 无法直接运行，先赋予执行权限：

```bash
chmod +x ShowJSON-*.AppImage
```

## 快速上手

1. 启动 ShowJSON，它会驻留在系统托盘。
2. 在任意应用中复制包含 JSON 的文本。
3. 按 `Cmd + Shift + J`（macOS）或 `Ctrl + Shift + J`（Windows/Linux）。
4. 在 ShowJSON 中查看、搜索或复制结果。
5. 按 `Esc`、关闭窗口或切换到其他应用即可隐藏窗口。

只有全局快捷键会读取并识别剪贴板。点击应用图标或托盘图标只会显示现有界面，不会读取新的剪贴板内容。

## 常见问题

### 为什么切换到其他应用后窗口会消失？

ShowJSON 被设计为快速查看窗口。完成查看后切换应用，窗口会自动隐藏，避免遮挡当前工作。

### JSON 无法解析时会自动修改原文吗？

不会。ShowJSON 会显示原文、错误位置和问题原因，只有点击“修复”后才会尝试修复并覆盖当前数据。

### 剪贴板历史保存在哪里？

历史记录保存在当前电脑的本地应用数据中，不会同步或上传。最多保存 50 条，总容量上限 25 MB；单条超过 5 MB 时不会写入历史。

### 如何彻底退出？

关闭窗口只会隐藏 ShowJSON。需要彻底退出时，请使用托盘菜单中的“退出”。

## 系统要求

- macOS 12 或更高版本。
- Windows 10/11 x64。
- 主流 x64 Linux 桌面发行版。

## License

[MIT](LICENSE)
