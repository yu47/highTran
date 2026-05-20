# HighTran

基于 **飞书 Lark Drive API** 的安全端到端文件中继传输工具。

文件在上传前使用 AES-256-GCM 加密，中继服务器（飞书云盘）仅看到加密后的数据。加密密钥从用户线下交换的短提取码衍生而来。

## 功能

- **发送** — 选择文件，获取 6 位提取码
- **接收** — 输入提取码和保存目录，下载文件
- **测速** — 测量通过 Lark Drive 的上传/下载吞吐量（1/8/16 MB）
- **深色模式** — 明暗主题切换
- **国际化** — 中文 / English 界面
- **加密中继** — AES-256-GCM，密钥由提取码经 MD5 衍生

## 架构

```
发送方                      Lark Drive                     接收方
  │                          │                            │
  │──── meta.enc ──────────►│                            │
  │                          │◄──── start.enc ───────────│
  │──── chunk_N.enc ───────►│                            │
  │──── complete.enc ──────►│◄──── chunk_N.enc ─────────│
  │                          │                            │
```

- 所有文件在客户端加密后上传
- 提取码既是会话标识符，也是密钥种子
- 依赖飞书云盘的文件/文件夹 API 作为传输层

## 构建

### 前置条件

- [Rust](https://rustup.rs/)（Windows 上需 MSVC 工具链）
- [Node.js](https://nodejs.org/) 18+
- [Tauri v2 CLI](https://v2.tauri.app/start/prerequisites/)

### 命令

```bash
# 安装前端依赖
npm install

# 开发模式（热重载）
npm run tauri dev

# 生产构建（Windows 上生成 MSI + NSIS）
npm run tauri build
```

构建产物位于：

```
src-tauri/target/release/bundle/msi/          # Windows 安装包 (MSI)
src-tauri/target/release/bundle/nsis/         # Windows 安装包 (NSIS)
```

## 配置

在设置面板（齿轮图标）中配置凭据。开发环境内置的默认凭据：

- **App ID**：`cli_aa871fad79f8de15`
- **App Secret**：`FMVDzOY6TVErA94tzzuFHeDlnigRui72`

配置存储在浏览器 `localStorage` 中，键名为 `ft-lark-config`。

## 技术栈

| 层       | 技术                            |
|----------|--------------------------------|
| 界面     | Tauri v2 + Vue 3 + TypeScript  |
| 后端     | Rust (reqwest, tokio)          |
| 加密     | AES-256-GCM (aes-gcm crate)    |
| 密钥衍生 | MD5 (md-5 crate)               |
| 传输     | 飞书 Lark Drive REST API       |
| 打包     | MSI / NSIS                     |

## 许可证

MIT
