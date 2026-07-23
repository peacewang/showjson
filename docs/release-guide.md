# ShowJSON GitHub Release 操作指南

这份文档面向第一次发布桌面 App 的维护者。目标是让 GitHub 在云端分别编译 macOS、Windows 和 Linux 安装包，本机不需要准备三个操作系统。

## 1. 是否应该单独建立仓库

ShowJSON 应使用独立的 `showjson` 仓库；当前远程仓库为 `https://github.com/peacewang/showjson.git`。

旧开发目录曾位于父级 `peacetool` Git 仓库中。GitHub 只识别仓库根目录下的 `.github/workflows/*.yml`，所以迁移到独立仓库后，CI 和 Release workflow 才会按预期运行。

单独仓库更适合 App 产品：

- `v0.1.0` 等 Tag 只表示 ShowJSON 的版本。
- GitHub Release、Issue、安装包和更新日志不会与其他工具混在一起。
- Release workflow 可以直接位于正确的仓库根目录。
- 后续接入 macOS/Windows 签名、自动更新更清晰。

如果坚持放在 monorepo，也可以把 Workflow 移到父仓库的 `.github/workflows/`，配置 `projectPath: showjson` 和路径触发条件；但第一次做 App 不推荐增加这层复杂度。

## 2. 首次初始化独立仓库

当前维护目录：

```bash
cd /Users/peacewang/code/peace/github/showjson
```

仓库已经创建时，检查远程即可：

```bash
gh auth status
git remote -v
```

其他机器不需要重新执行 `git init`，直接运行 `git clone https://github.com/peacewang/showjson.git`。

## 3. Workflow 会生成什么

推送符合 `v*` 的 Tag 后，[release.yml](../.github/workflows/release.yml) 会启动四个独立任务：

| 运行环境 | 产物 | 用户安装方式 |
|---|---|---|
| macOS Intel | x64 DMG | 打开 DMG，将 App 拖入 Applications |
| macOS Apple Silicon | arm64 DMG | 打开 DMG，将 App 拖入 Applications |
| Windows x64 | NSIS `setup.exe` | 双击安装 |
| Ubuntu x64 | AppImage、Deb | 直接运行 AppImage 或安装 Deb |

所有任务成功后，GitHub 会创建一个 Draft Release。Draft 不会公开，适合先下载安装验证。

## 4. 发布一个新版本

以下示例发布 `0.2.0`。

### 4.1 同步版本号

```bash
npm run version:set -- 0.2.0
```

该命令会同时更新：

- `package.json`
- `package-lock.json`
- `src-tauri/tauri.conf.json`
- `src-tauri/Cargo.toml`

随后刷新并提交 Cargo lockfile：

```bash
cargo check --manifest-path src-tauri/Cargo.toml
```

### 4.2 本地检查

```bash
npm ci
npm run check
npm test
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

至少在当前 Mac 上构建一次：

```bash
npm run tauri build -- --bundles dmg
```

### 4.3 提交并推送 Tag

```bash
git add .
git commit -m "release: v0.2.0"
git tag -a v0.2.0 -m "ShowJSON v0.2.0"
git push origin main
git push origin v0.2.0
```

不要在代码版本仍为 `0.1.0` 时推送 `v0.2.0`，否则 Tag、安装包和应用内版本会不一致。

## 5. 在 GitHub 查看编译进度

1. 打开仓库网页。
2. 进入 `Actions`。
3. 打开 `Release desktop apps`。
4. 应看到 macOS Intel、macOS ARM、Windows、Linux 四个 Job。
5. 某个 Job 失败时，展开第一个红色步骤查看错误，不要只看最后的汇总错误。
6. 全部成功后进入仓库右侧 `Releases`，打开 Draft Release。

下载每个平台的安装包，在真实系统上完成以下冒烟测试：

- 可以安装和启动。
- 托盘图标存在。
- `Cmd/Ctrl + Shift + J` 能读取剪贴板。
- 点击托盘图标只显示窗口，不读取剪贴板。
- 切换到其他应用后窗口自动隐藏。
- 剪贴板历史在重启后仍存在。
- 无效 JSON 可以定位并在用户点击后修复。

验证完成后，在 Draft Release 中补充更新说明并点击 `Publish release`。

## 6. 签名应分两个阶段处理

### 第一阶段：内部测试

先使用未签名的 Draft Release 验证三平台功能。

- macOS 可能提示无法验证开发者。
- Windows 可能出现 SmartScreen 提示。
- Linux 通常可以直接运行。

这不影响功能验证，但不适合正式公开分发。

### 第二阶段：正式公开发布

macOS：

- 需要 Apple Developer Program 账号。
- 配置 Developer ID Application 证书。
- 在 GitHub Secrets 中保存证书和 notarization 凭据。
- 开启 code signing 与 notarization。

Windows：

- 推荐购买代码签名证书，或使用 Azure Trusted Signing。
- 对 NSIS 安装程序和主程序签名，降低 SmartScreen 警告概率。

签名凭据只能放在 GitHub Actions Secrets 中，禁止提交到仓库。

## 7. 发布失败时的安全处理

- Tag 已推送但构建失败：修复代码后删除远端 Tag，再重新创建同版本 Tag；或者直接递增 Patch 版本。
- Draft Release 可以安全删除，不影响代码。
- 不要公开包含失败或未验证安装包的 Release。
- 不要在本地把证书、密码、Token 写入 `.env` 后提交。
