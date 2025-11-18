# Dioxus Inspector / 디옥서스 인스펙터

> Click a rendered element (Cmd/Ctrl + Shift + Click) and jump straight to the original Rust source line in your IDE.  
> 렌더된 요소를 **Cmd/Ctrl + Shift + Click** 하면 IDE에서 원본 소스 라인으로 바로 이동합니다.

## 🚀 빠른 시작

### 1. 의존성 추가 (이미 완료됨)

```toml
# Cargo.toml
[dependencies]
dioxus-inspector = { path = "../../crates/dioxus/packages/inspector", features = ["client"] }

[features]
inspector = ["dioxus-inspector"]
```

### 2. 컴포넌트 설정

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

### 3. Server 실행

```bash
# Terminal 1: Inspector Server
npm run dev:inspector

# Terminal 2: Dioxus App (Web)
cd apps/metacity-server
dx serve --features inspector

# (선택) Desktop/Tauri
cargo tauri dev --features inspector
```

### 4. 사용하기

1. 브라우저에서 앱 열기
2. **Cmd+Shift+Click** (또는 Ctrl+Shift+Click)
3. 컴포넌트 클릭
4. IDE가 자동으로 열림! (VSCode, Cursor, Windsurf, JetBrains 등 대부분 CLI 지원 IDE)

## 📝 상세 사용법

### DOM 메타데이터는 자동 삽입

`dx serve --features inspector` 처럼 **Debug 빌드**를 실행하면 Dioxus 매크로가 모든 DOM 요소에 `data-inspector` 속성을 자동으로 추가합니다. (파일 경로, 줄, 열 정보 포함)  
따라서 더 이상 `data_inspector` 속성을 직접 작성할 필요가 없습니다.

### 조건부 컴파일

- **Debug 빌드**: Inspector 활성화
- **Release 빌드**: 자동으로 제거 (성능 영향 0)

```bash
# Debug (inspector 포함)
dx serve --features inspector

# Release (inspector 제외)
dx build --release
```

## 🎯 IDE 지원 / Supported IDEs

- VSCode / Code Insiders
- Cursor
- Windsurf
- WebStorm / IntelliJ / Fleet (JetBrains)
- 기타 `--goto file:line[:column]` 형태의 CLI를 제공하는 IDE (커스텀 명령 추가 가능)

Inspector server는 환경변수(`EDITOR`, `TERM_PROGRAM`), 실행 중인 프로세스, CLI 존재 여부(`which`, `where`)를 활용해 IDE를 감지합니다. 필요하면 `scripts/inspector-server.js`에서 감지 순서를 커스터마이징하세요.

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

## 🐛 문제 해결

### 클릭해도 반응 없음
```bash
# 1. Inspector server 실행 중인지 확인
npm run dev:inspector

# 2. 브라우저 콘솔 확인
# "Inspector client installed" 메시지 있어야 함
```

### IDE가 안 열림
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

## 📚 아키텍처 / Architecture

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

## 🎨 예제 / Example

`apps/metacity-server/src/components/app.rs`에서 실전 예시를 볼 수 있습니다. 아래와 같이 InspectorClient만 초기화하면 DOM 노드에는 자동으로 메타데이터가 주입됩니다.

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

Debug 모드에서 RSX 매크로가 모든 노드에 `data-inspector` 속성을 자동으로 부여합니다.

## ✅ CI / 검증 방법

Inspector가 Dioxus fork 내부에 포함되어 있으므로, CI에서 다음 명령들을 통해 회귀를 막을 수 있습니다.

1. **FMT & Clippy**
   ```bash
   cargo fmt --workspace -- packages/rsx/src/element.rs packages/inspector packages/inspector-macros
   cargo clippy -p dioxus-inspector -p dioxus-inspector-macros --all-features -- -D warnings
   ```

2. **WASM 빌드 검사** (브라우저 클라이언트 확인)
   ```bash
   cargo check -p dioxus-inspector --features client --target wasm32-unknown-unknown
   ```

3. **Downstream Smoke Test** (예: POS-agent)
   ```bash
   cd apps/metacity-server
   cargo check --features inspector
   # or run dx serve in CI with xvfb if 통합 테스트가 필요
   ```

4. **Inspector Server Lint (선택)**
   ```bash
   npm run lint -- scripts/inspector-server.js
   ```

이 검증 절차를 CI 파이프라인에 넣으면 Inspector 관련 변경이 들어와도 안정적으로 동작하는지 빠르게 확인할 수 있습니다.

## 📄 라이선스

MIT OR Apache-2.0
