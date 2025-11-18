# Dioxus Inspector / 迪氧索斯代码探查器

[English](./README.md) • [한국어](./README.ko.md) • 简体中文

按住 **Cmd/Ctrl + Shift** 点击任意渲染的元素，即可在 IDE 中直接跳转到对应的 Rust 源码行。

## 🚀 快速开始

### 1. 添加依赖

```toml
# Cargo.toml
[dependencies]
dioxus-inspector = { path = "../../crates/dioxus/packages/inspector", features = ["client"] }

[features]
inspector = ["dioxus-inspector"]
```

### 2. 在组件中初始化客户端

```rust
use dioxus::prelude::*;
use dioxus_inspector::InspectorClient;

#[component]
pub fn App() -> Element {
    #[cfg(feature = "inspector")]
    {
        use_effect(|| {
            if let Err(err) = InspectorClient::new("http://127.0.0.1:41235").install() {
                tracing::warn!(?err, "Inspector client failed to initialize");
            }
            || {}
        });
    }

    rsx! {
        div {
            class: "app",
            "Hello World"
        }
    }
}
```

### 3. 启动 Inspector Server 与应用

```bash
# Terminal 1: Inspector Server
npm run dev:inspector

# Terminal 2: Dioxus App (Web)
cd apps/metacity-server
dx serve --features inspector

# （可选）Desktop/Tauri
cargo tauri dev --features inspector
```

### 4. 使用

1. 在浏览器（Web 或 Desktop WebView）中打开应用
2. 按住 **Cmd/Ctrl + Shift** 点击想要查看的元素
3. Inspector server 会自动调用 IDE CLI（`code`/`cursor`/`windsurf`/JetBrains 等）

## 📝 额外说明

### DOM 元数据自动注入

只要以 `--features inspector` 运行 **Debug 构建**，经过补丁的 `rsx!` 宏就会为每个 DOM 节点自动添加 `data-inspector`（包含文件、行、列、标签信息）。无需手动编写任何属性。

### 条件编译

- **Debug**：Inspector 启用
- **Release**：Inspector 自动移除，对性能无影响

```bash
# Debug（含 inspector）
dx serve --features inspector

# Release（不含 inspector）
dx build --release
```

## 🎯 支持的 IDE

- VSCode / Code Insiders
- Cursor
- Windsurf
- WebStorm / IntelliJ / Fleet（JetBrains 家族）
- 任何提供 `--goto file:line[:column]` CLI 的 IDE（可自定义命令）

Node 版 Inspector Server 会依据 `EDITOR`、`TERM_PROGRAM`、正在运行的进程或 CLI 是否存在（`which`/`where`）来自动识别 IDE。如需自定义顺序，可修改 `scripts/inspector-server.js`。

## 🔧 配置 / 自定义

### 修改端口

```rust
const INSPECTOR_ENDPOINT: &str = "http://127.0.0.1:8888";
InspectorClient::new(INSPECTOR_ENDPOINT).install()
```

对应地，Node 服务器中：
```javascript
// scripts/inspector-server.js
const PORT = 8888;
```

### 自定义快捷键

```rust
use dioxus_inspector::client::ClickModifier;

let client = InspectorClient::new(endpoint)
    .with_modifier(ClickModifier {
        meta: false,  // 不需要 Cmd/Ctrl
        shift: true,  // 仅 Shift
    });
```

## 🐛 常见问题

### 点击无响应
```bash
# 1. 检查 Inspector server 是否在运行
npm run dev:inspector

# 2. 打开浏览器控制台，确认看到 "Inspector client installed"
```

### IDE 没有打开
```bash
# 1. 查看 server 日志
[Inspector] Opening: code --goto /path/to/file.rs:42:1

# 2. 检查 IDE CLI 是否已安装
which code   # 或 cursor、windsurf

# 3. 手动执行一次
windsurf --goto /path/to/file.rs:42:1
```

### CORS 报错
➡ `scripts/inspector-server.js` 默认开启了 CORS，确认端口一致即可。

## 📚 架构

```
Browser (WASM)                Dev Server (Node.js)           IDE
    │                              │                          │
    │  Cmd/Ctrl+Shift+Click        │                          │
    │──────────────────────────────>│                          │
    │                              │                          │
    │  POST /api/inspector/open    │  spawn('code'/'cursor')  │
    │  { file, line, column }      │─────────────────────────>│
    │                              │                          │
    │  ← 200 OK                    │                          │
    │                              │                     File opens!
```

## 🎨 示例

参考 `apps/metacity-server/src/components/app.rs`。简化示例：

```rust
#[cfg(feature = "inspector")]
use dioxus_inspector::InspectorClient;

#[component]
pub fn App() -> Element {
    #[cfg(feature = "inspector")]
    use_effect(|| {
        InspectorClient::new("http://127.0.0.1:41235/api/inspector/open")
            .install()
            .ok();
        || {}
    });

    rsx! { div { class: "app", "Hello" } }
}
```

在 Debug 构建中，`rsx!` 会自动注入 `data-inspector`。

## ✅ CI 建议

1. **格式化 + Clippy**
   ```bash
   cargo fmt --workspace -- packages/rsx/src/element.rs packages/inspector packages/inspector-macros
   cargo clippy -p dioxus-inspector -p dioxus-inspector-macros --all-features -- -D warnings
   ```

2. **WASM 构建检查**（验证浏览器客户端）
   ```bash
   cargo check -p dioxus-inspector --features client --target wasm32-unknown-unknown
   ```

3. **下游项目冒烟测试**（如 POS-agent）
   ```bash
   cd apps/metacity-server
   cargo check --features inspector
   ```

4. **Inspector Server Lint（可选）**
   ```bash
   npm run lint -- scripts/inspector-server.js
   ```

在 CI 中加入这些命令即可防止 RSX 补丁或 Inspector runtime 回归。

## 📄 License

MIT OR Apache-2.0
