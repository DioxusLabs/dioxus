# Dioxus Inspector

English • [한국어](./README.ko.md) • [简体中文](./README.zh.md)

Click any rendered element (Cmd/Ctrl + Shift + Click) and jump straight to the original Rust source line in your IDE.

## 🚀 빠른 시작

### 1. Add the dependency

```toml
# Cargo.toml
[dependencies]
dioxus-inspector = { path = "../../crates/dioxus/packages/inspector", features = ["client"] }

[features]
inspector = ["dioxus-inspector"]
```

### 2. Initialize the client in your component

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

### 3. Run the inspector server + your app

```bash
# Terminal 1: Inspector Server
npm run dev:inspector

# Terminal 2: Dioxus App (Web)
cd apps/metacity-server
dx serve --features inspector

# (선택) Desktop/Tauri
cargo tauri dev --features inspector
```

### 4. Use it

1. Open your app in a browser (web or desktop)
2. Hold **Cmd/Ctrl + Shift** and click the element you want
3. The Inspector server spawns your IDE (`code`, `cursor`, `windsurf`, JetBrains, …)

## 📝 상세 사용법

### DOM 메타데이터는 자동 삽입

When you run a **debug build** with the `inspector` feature, the patched `rsx!` macro automatically injects a `data-inspector` attribute into every DOM node (file, line, column, tag). You no longer need to annotate elements manually.

### 조건부 컴파일

- **Debug 빌드**: Inspector 활성화
- **Release 빌드**: 자동으로 제거 (성능 영향 0)

```bash
# Debug (inspector 포함)
dx serve --features inspector

# Release (inspector 제외)
dx build --release
```

## 🎯 Supported IDEs

- VSCode / Code Insiders
- Cursor
- Windsurf
- WebStorm / IntelliJ / Fleet (JetBrains family)
- Any IDE that exposes a `--goto file:line[:column]` CLI (you can customize the command)

The Node inspector server auto-detects IDEs using `EDITOR`, `TERM_PROGRAM`, running processes, or CLI availability (`which`/`where`). Adjust `scripts/inspector-server.js` if you need a custom detection order.

## 🔧 설정 / 커스터마이징

### 다른 포트 사용

```rust
const INSPECTOR_ENDPOINT: &str = "http://127.0.0.1:8888";

InspectorClient::new(INSPECTOR_ENDPOINT).install()
```

Server도 동일한 포트로:
```javascript
// scripts/inspector-server.js
const PORT = 8888;
```

### 커스텀 단축키

```rust
use dioxus_inspector::client::ClickModifier;

let client = InspectorClient::new(endpoint)
    .with_modifier(ClickModifier {
        meta: false,   // Cmd/Ctrl 불필요
        shift: true,   // Shift만 필요
    });
```

## 🐛 Troubleshooting

### 클릭해도 반응 없음
```bash
# 1. Inspector server 실행 중인지 확인
npm run dev:inspector

# 2. 브라우저 콘솔 확인
# "Inspector client installed" 메시지 있어야 함
```

### IDE doesn't open
```bash
# 1. Server 로그 확인
[Inspector] Opening: code --goto /path/to/file.rs:42:1

# 2. IDE CLI 설치 확인
which windsurf  # 또는 code, cursor

# 3. 수동으로 테스트
windsurf --goto /path/to/file.rs:42:1
```

### CORS 에러
→ `inspector-server.js`에 이미 CORS 설정됨. 포트 확인.

## 📚 Architecture

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

## 🎨 Example

See `apps/metacity-server/src/components/app.rs` for a full integration. A minimal snippet looks like:

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

In debug builds the patched `rsx!` macro injects `data-inspector` automatically.

## ✅ CI recommendations

1. **FMT & Clippy**
   ```bash
   cargo fmt --workspace -- packages/rsx/src/element.rs packages/inspector packages/inspector-macros
   cargo clippy -p dioxus-inspector -p dioxus-inspector-macros --all-features -- -D warnings
   ```

2. **WASM 빌드 검사** (브라우저 클라이언트 확인)
   ```bash
   cargo check -p dioxus-inspector --features client --target wasm32-unknown-unknown
   ```

3. **Downstream smoke test** (e.g., POS-agent)
   ```bash
   cd apps/metacity-server
   cargo check --features inspector
   # or run dx serve in CI with xvfb if 통합 테스트가 필요
   ```

4. **Inspector server lint (optional)**
   ```bash
   npm run lint -- scripts/inspector-server.js
   ```

Add these steps to your CI pipeline to catch regressions in both the core RSX patch and the inspector runtime.

## 📄 라이선스

MIT OR Apache-2.0
