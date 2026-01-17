<p>
    <p align="center" >
      <img src="../../notes/header-light.svg#gh-light-mode-only" >
      <img src="../../notes/header-dark.svg#gh-dark-mode-only" >
      <a href="https://dioxuslabs.com">
          <img src="../../notes/dioxus_splash_8.avif">
      </a>
    </p>
</p>
<div align="center">
  <!-- Crates version -->
  <a href="https://crates.io/crates/dioxus">
    <img src="https://img.shields.io/crates/v/dioxus.svg?style=flat-square"
    alt="Crates.io version" />
  </a>
  <!-- Downloads -->
  <a href="https://crates.io/crates/dioxus">
    <img src="https://img.shields.io/crates/d/dioxus.svg?style=flat-square"
      alt="Download" />
  </a>
  <!-- docs -->
  <a href="https://docs.rs/dioxus">
    <img src="https://img.shields.io/badge/docs-latest-blue.svg?style=flat-square"
      alt="docs.rs docs" />
  </a>
  <!-- CI -->
  <a href="https://github.com/jkelleyrtp/dioxus/actions">
    <img src="https://github.com/dioxuslabs/dioxus/actions/workflows/main.yml/badge.svg"
      alt="CI status" />
  </a>

  <!--Awesome -->
  <a href="https://dioxuslabs.com/awesome">
    <img src="https://cdn.rawgit.com/sindresorhus/awesome/d7305f38d29fed78fa85652e3a63e154dd8e8829/media/badge.svg" alt="Awesome Page" />
  </a>
  <!-- Discord -->
  <a href="https://discord.gg/XgGxMSkvUM">
    <img src="https://img.shields.io/discord/899851952891002890.svg?logo=discord&style=flat-square" alt="Discord Link" />
  </a>
</div>

<div align="center">
  <h3>
    <a href="https://dioxuslabs.com"> Website </a>
    <span> | </span>
    <a href="https://github.com/DioxusLabs/dioxus/tree/main/examples"> Examples </a>
    <span> | </span>
    <a href="https://dioxuslabs.com/learn/0.7/tutorial"> Guide </a>
    <span> | </span>
    <a href="https://github.com/DioxusLabs/dioxus/blob/main/notes/translations/zh-cn/README.md"> 中文 </a>
    <span> | </span>
    <a href="https://github.com/DioxusLabs/dioxus/blob/main/notes/translations/pt-br/README.md"> PT-BR </a>
    <span> | </span>
    <a href="https://github.com/DioxusLabs/dioxus/blob/main/notes/translations/ja-jp/README.md"> 日本語 </a>
    <span> | </span>
    <a href="https://github.com/DioxusLabs/dioxus/blob/main/notes/translations/tr-tr"> Türkçe </a>
    <span> | </span>
    <a href="https://github.com/DioxusLabs/dioxus/blob/main/notes/translations/ko-kr"> 한국어 </a>
  </h3>
</div>
<br>
<br>

웹, 데스크톱, 모바일 등 다양한 플랫폼을 단일 코드베이스로 구축하세요. 제로 설정 환경, 통합 핫 리로딩, 시그널 기반 상태 관리. 서버 함수로 백엔드 기능을 추가하고 CLI를 사용하여 번들링하세요.

```rust
fn app() -> Element {
    let mut count = use_signal(|| 0);

    rsx! {
        h1 { "High-Five counter: {count}" }
        button { onclick: move |_| count += 1, "Up high!" }
        button { onclick: move |_| count -= 1, "Down low!" }
    }
}
```

## ⭐️ 고유 기능:

- 세 줄의 코드로 크로스 플랫폼 앱 개발 (웹, 데스크톱, 모바일, 서버 등)
- [인체공학적 상태 관리](https://dioxuslabs.com/blog/release-050)은 React, Solid, Svelte의 장점을 결합했습니다
- 극도로 높은 성능을 자랑하며 Rust의 가장 빠른 wasm 프레임워크 [sledgehammer](https://dioxuslabs.com/blog/templates-diffing)에 의해 구동됩니다
- 웹, macOS, Linux, Windows에 배포할 수 있는 통합 번들러
- 그 외에도 많은 기능! [Dioxus 투어를 살펴보세요](https://dioxuslabs.com/learn/0.7/).

## 핫 리로딩

한 개의 명령어, `dx serve`로 앱을 실행하세요. 마크업과 스타일을 수정하면 실시간으로 결과를 확인할 수 있습니다. Rust 코드 핫 리로딩은 아직 1차 지원은 아니지만 [hot-lib-reloader](https://docs.rs/hot-lib-reloader/latest/hot_lib_reloader/)를 통해 가능합니다.

<div align="center">
  <img src="../../notes/hotreload.gif">
</div>

## 웹 및 데스크톱 배포를 위한 번들러

단순히 `dx bundle`을 실행하면 앱이 빌드되고 최적화된 번들로 패키징됩니다. 웹에서는 [`.avif` 생성, `.wasm` 압축, 최소화](https://dioxuslabs.com/learn/0.7/tutorial/assets) 등을 활용할 수 있습니다. [50kb 미만](https://github.com/ealmloff/tiny-dioxus/)의 웹앱과 15mb 미만의 데스크톱/모바일 앱을 빌드하세요.

<div align="center">
  <img src="../../notes/bundle.gif">
</div>

## 환상적인 문서

깨끗하고 읽기 쉽고 포괄적인 문서를 구축하는 데 많은 노력을 기울였습니다. 모든 HTML 요소와 리스너는 MDN 문서로 문서화되었으며, 저희 문서 사이트는 Dioxus 자체와 지속적인 통합을 실행하여 문서가 항상 최신 상태로 유지되도록 합니다. 가이드, 참조, 레시피 등을 보려면 [Dioxus 웹사이트](https://dioxuslabs.com/learn/0.7/)를 확인하세요. 재미있는 사실: 저희는 Dioxus 웹사이트를 새로운 Dioxus 기능의 테스트베드로 사용합니다 - [확인해보세요!](https://github.com/dioxusLabs/docsite)

<div align="center">
  <img src="../../notes/docs.avif">
</div>

## 개발자 경험에 중점

Dioxus는 개발자 경험을 우선시하며, end-to-end 도구 개발에 많은 노력을 기울였습니다. RSX 코드를 자동으로 포맷하고 HTML을 RSX로 변환하는 등 다양한 기능을 제공하는 [VSCode 확장 프로그램](https://marketplace.visualstudio.com/items?itemName=DioxusLabs.dioxus)을 제작했습니다. 또한 새로운 앱 생성, 서비스 실행, 크로스 플랫폼 번들링을 지원하는 매우 강력한 CLI 를 개발했으며, 배포는 향후 로드맵에 포함되어 있습니다.

<div align="center">
  <img src="../../notes/autofmt.gif">
</div>

## 커뮤니티

Dioxus는 커뮤니티 중심의 프로젝트로, 매우 활발한 [Discord](https://discord.gg/XgGxMSkvUM) 및 [GitHub](https://github.com/DioxusLabs/dioxus/issues) 커뮤니티를 보유하고 있습니다. 우리는 항상 도움을 구하고 있으며, 질문에 답변하고 시작하는 데 도움을 드릴 준비가 되어 있습니다. [우리의 SDK](https://github.com/DioxusLabs/dioxus-std)는 커뮤니티에서 운영되며, 무료 업그레이드와 지원을 받는 최고의 Dioxus 크레이트를 위한 [GitHub 조직](https://github.com/dioxus-community/)도 운영하고 있습니다.

<div align="center">
  <img src="../../notes/dioxus-community.avif">
</div>

## 풀타임 코어 팀

Dioxus는 사이드 프로젝트에서 소수의 전임 엔지니어 팀으로 성장했습니다. FutureWei, Satellite.im, GitHub Accelerator 프로그램의 후원 덕분에 Dioxus를 풀타임으로 개발할 수 있게 되었습니다. 저희의 장기 목표는 유료 고품질 엔터프라이즈 도구를 제공하여 Dioxus가 자립할 수 있도록 하는 것입니다. 귀사가 Dioxus 도입에 관심이 있고 저희와 함께 일하고 싶다면, 연락해 주세요!

## 지원 플랫폼

<div align="center">
  <table style="width:100%">
    <tr>
      <td>
      <b>웹</b>
      <br />
      <em>Tier 1 지원</em>
      </td>
      <td>
        <ul>
          <li>WebAssembly을 사용하여 DOM에 직접 렌더링</li>
          <li>SSR로 사전 렌더링하고 클라이언트에서 리하이드레이션</li>
          <li>React와 유사한 약 50kb 크기의 간단한 "Hello World"</li>
          <li>내장된 개발 서버와 핫 리로딩으로 빠른 반복</li>
        </ul>
      </td>
    </tr>
    <tr>
      <td>
      <b>풀스택</b>
      <br />
      <em>Tier 1 지원</em>
      </td>
      <td>
        <ul>
          <li>서스펜스, 리하이드레이션, 서버 사이드 렌더링</li>
          <li>서버 함수로 백엔드 기능을 빠르게 추가</li>
          <li>추출기, 미들웨어, 라우팅 통합</li>
          <li>데스크톱 및 모바일과 호환 가능!</li>
        </ul>
      </td>
    </tr>
    <tr>
      <td>
      <b>데스크톱</b>
      <br />
      <em>Tier 1 지원</em>
      </td>
      <td>
        <ul>
          <li>Webview를 사용하여 렌더링하거나 실험적으로 WGPU 또는 <a href="https://freyaui.dev">Freya</a> (skia)와 함께 렌더링</li>
          <li>제로 설정. 간단히 `cargo run` 또는 `dx serve`로 앱을 빌드</li>
          <li>IPC 없이 네이티브 시스템 접근에 대한 완전한 지원</li>
          <li>macOS, Linux, Windows를 지원합니다. 3MB 미만의 포터블 바이너리</li>
        </ul>
      </td>
    </tr>
    <tr>
      <td>
      <b>Liveview</b>
      <br />
      <em>Tier 1 지원</em>
      </td>
      <td>
        <ul>
          <li>앱 전체 또는 단일 컴포넌트만을 서버에서 완전히 렌더링</li>
          <li>Axum, Warp와 같은 인기 있는 Rust 프레임워크와의 통합</li>
          <li>극도로 낮은 지연 시간과 10,000개 이상의 동시 앱 지원 가능</li>
        </ul>
      </td>
    </tr>
    <tr>
      <td>
      <b>모바일</b>
      <br />
      <em>Tier 2 지원</em>
      </td>
      <td>
        <ul>
          <li>Webview를 사용하여 렌더링하거나 실험적으로 WGPU 또는 Skia와 함께 렌더링</li>
          <li>iOS 및 Android 지원</li>
          <li>현재는 꽤 실험적이며, 2024년 내내 많은 개선 예정</li>
        </ul>
      </td>
    </tr>
    <tr>
      <td>
      <b>터미널</b>
      <br />
      <em>Tier 2 지원</em>
      </td>
      <td>
        <ul>
          <li><a href="https://github.com/vadimdemedes/ink">ink.js</a>와 유사하게 앱을 터미널에 직접 렌더링</li>
          <li>익숙한 브라우저의 flexbox 및 CSS 모델로 구동</li>
          <li>텍스트 입력, 버튼, 포커스 시스템과 같은 내장 위젯</li>
        </ul>
      </td>
    </tr>
  </table>
</div>

## 예제 실행하기

> 이 저장소의 메인 브랜치에 있는 예제들은 dioxus의 git 버전과 CLI를 대상으로 합니다. 최신 안정 버전의 dioxus와 호환되는 예제를 찾고 있다면 [0.5 브랜치](https://github.com/DioxusLabs/dioxus/tree/v0.6/examples)를 확인하세요.

이 저장소 최상위에 있는 예제들은 다음과 같이 실행할 수 있습니다:

```sh
cargo run --example <example>
```

그러나 dioxus-cli 다운로드를 권장합니다. git 버전의 dioxus를 사용 중인 경우, 다음 명령어로 CLI의 일치하는 버전을 설치할 수 있습니다:

```sh
cargo install --git https://github.com/DioxusLabs/dioxus dioxus-cli --locked
```

CLI를 사용하면 웹 플랫폼에서 예제를 실행할 수도 있습니다. 이 명령어로 기본 데스크톱 기능을 비활성화하고 웹 기능을 활성화하면 됩니다:

```sh
dx serve --example <example> --platform web -- --no-default-features
```

## Dioxus vs 다른 프레임워크

우리는 모든 프레임워크를 사랑하며 Rust 생태계의 혁신을 지켜보는 것을 즐깁니다. 실제로, 많은 프로젝트가 다른 프레임워크와 공유되고 있습니다. 예를 들어, 우리의 flex-box 라이브러리 [Taffy](https://github.com/DioxusLabs/taffy)는 [Bevy](https://bevyengine.org/), [Zed](https://zed.dev/), [Lapce](https://lapce.dev/), [Iced](https://github.com/iced-rs/iced) 등에서 사용되고 있습니다.

Dioxus는 다른 프레임워크와 차별화되는 몇 가지 핵심 사항에 중점을 둡니다:

- **React처럼**: 우리는 컴포넌트, props, 훅과 같은 개념을 사용하여 UI를 구축하며, 상태 관리는 SolidJS보다 Svelte에 더 가깝습니다.
- **HTML과 CSS**: 우리는 HTML과 CSS를 완전히 활용하며, 특이점도 모두 수용합니다.
- **독립적 렌더러**: [우리의 빠른 VirtualDOM](https://dioxuslabs.com/blog/templates-diffing) 덕분에 원하는 플랫폼의 렌더러로 교체할 수 있습니다.
- **협업적**: 가능한 한 [Taffy](https://github.com/DioxusLabs/taffy), [magnanis](https://github.com/DioxusLabs/manganis), [include_mdbook](https://github.com/DioxusLabs/include_mdbook), [blitz](http://github.com/dioxusLabs/blitz)와 같은 크레이트를 분리하여 생태계가 함께 성장할 수 있도록 합니다.

### Dioxus vs Tauri

Tauri는 프론트엔드가 React, Vue, Svelte 등의 웹 기반 프레임워크로 작성된 데스크톱(모바일 지원 예정) 앱을 구축하기 위한 프레임워크입니다. 네이티브 작업이 필요할 때마다 Rust 함수를 작성하여 프론트엔드에서 호출할 수 있습니다.

- **네이티브 Rust**: Tauri의 아키텍처는 UI를 JavaScript 또는 WebAssembly로 제한합니다. 반면 Dioxus에서는 Rust 코드가 사용자의 머신에서 네이티브로 실행되어 스레드 생성, 파일 시스템 접근 등의 작업을 IPC 브리지를 사용하지 않고 수행할 수 있습니다. 이는 앱의 아키텍처를 대폭 단순화하고 구축을 용이하게 합니다. 원하신다면 Dioxus-Web을 프론트엔드로 사용하여 Tauri 앱을 구축할 수도 있습니다.
- **범위 차이**: Tauri는 JavaScript와 복잡한 빌드 도구를 지원해야 하므로 할 수 있는 작업의 범위가 제한됩니다. Dioxus는 Rust에 전적으로 집중하고 있기 때문에 Server Functions, 고급 번들링, 네이티브 렌더러와 같은 추가 유틸리티를 제공할 수 있습니다.
- **공유된 DNA**: Tauri와 Dioxus는 별도의 프로젝트이지만, Tao와 Wry와 같은 라이브러리를 공유합니다. 이들은 Tauri 팀에서 유지 관리하는 윈도잉 및 웹뷰 라이브러리입니다.

### Dioxus vs Leptos

Leptos는 SolidJS와 SolidStart와 유사하게 풀스택 웹 앱을 구축하기 위한 라이브러리입니다. 두 라이브러리는 웹에서 유사한 목표를 공유하지만 몇 가지 주요 차이점이 있습니다:

- **반응성 모델**: Leptos는 기본 반응성으로 시그널을 사용하는 반면, Dioxus는 VirtualDom과 리렌더링을 선택합니다. 이론적으로 시그널이 더 효율적이지만, 실제로 Dioxus의 VirtualDom은 [block-dom 영감을 받은 템플릿](https://dioxuslabs.com/blog/templates-diffing) 덕분에 실제 차이점을 거의 수행하지 않으며 [실제로 Leptos보다 빠릅니다](https://krausest.github.io/js-framework-benchmark/2024/table_chrome_123.0.6312.59.html).

- **제어 흐름**: Leptos가 반응성으로 시그널을 사용하기 때문에, `for` 루프와 `if` 문과 같은 Leptos의 원시 구조에 제한됩니다. 이를 잘못 사용하면 앱의 반응성이 상실되어 디버깅이 어려운 UI 문제가 발생할 수 있습니다. Dioxus에서는 이터레이터, 일반 Rust의 `for` 루프 및 `if` 문을 사용할 수 있으며, 앱은 여전히 반응성을 유지합니다. 실제로, 목록에 카운터를 삽입하는 Dioxus 컴포넌트는 다음과 같을 수 있습니다:

```rust
fn Counters() -> Element {
    let mut counters = use_signal(|| vec![0; 10]);

    rsx! {
        button { onclick: move |_| counters.push(counters.len()), "Add Counter" }
        ul {
            for idx in 0..counters.len() {
                li {
                    button { onclick: move |_| counters.write()[idx] += 1, "{counters.index(idx)}" }
                    button { onclick: move |_| { counters.remove(idx); }, "Remove" }
                }
            }
        }
    }
}
```

[Leptos에서는 `<For>` 컴포넌트를 사용합니다.](https://book.leptos.dev/view/04_iteration.html#dynamic-rendering-with-the-for-component):

```rust
fn Counters() -> impl IntoView {
    let counters = RwSignal::new(vec![0; 10]);

    view! {
        <button on:click=move |_| counters.update(|n| n.push(n.len()))>"Add Counter"</button>
        <For
            each=move || 0..counters.with(Vec::len)
            key=|idx| *idx
            let:idx
        >
            <li>
                <button on:click=move |_| counters.update(|n| n[idx] += 1)>
                    {Memo::new(move |_| counters.with(|n| n[idx]))}
                </button>
                <button on:click=move |_| counters.update(|n| { n.remove(idx); })>
                    "Remove"
                </button>
            </li>
        </For>
    }
}
```

- **`Copy` 상태**: Dioxus 0.1부터 0.4까지는 Rust의 borrow 검사기의 규칙을 완화하기 위해 생명주기에 의존했습니다. 이는 이벤트 핸들러에는 잘 작동했지만, 비동기 작업에서는 어려움을 겪었습니다. Dioxus 0.5에서는 Leptos에서 차용한 [`Copy` 상태 모델](https://crates.io/crates/generational-box)로 전환했습니다.

- **범위 차이**: Dioxus는 웹, 데스크톱, 모바일, LiveView 등을 위한 렌더러를 제공합니다. 또한 커뮤니티 라이브러리와 크로스 플랫폼 SDK를 유지 관리합니다. 이 작업의 범위가 방대하여 역사적으로 Leptos보다 느린 주기로 릴리스되었습니다. Leptos는 전체 스택 웹에 중점을 두며, Dioxus에는 없는 islands, `<Form />` 컴포넌트 및 기타 웹 전용 기능과 같은 기능을 제공합니다. 일반적으로 Leptos로 구축한 웹 앱은 더 작은 용량을 가집니다.

- **다른 DSL**: 두 프레임워크 모두 웹을 목표로 하지만, Dioxus는 UI 구축을 위해 자체적인 Rust 유사 DSL을 사용하는 반면, Leptos는 더 HTML 유사한 구문을 사용합니다. 이는 코드 접기 및 구문 강조와 같은 IDE 기능과의 호환성을 유지하기 위해 선택한 것입니다. 일반적으로 Dioxus는 DSL에서 더 많은 "매직"을 활용합니다. 예를 들어, dioxus는 문자열을 자동으로 포맷해주지만, Leptos는 문자열을 정적 및 동적 세그먼트로 분할할 수 있습니다.

```rust
// dioxus
rsx! {
  div { class: "my-class", enabled: true, "Hello, {name}" }
}

// leptos
view! {
  <div class="my-class" enabled={true}>
    "Hello "
    {name}
  </div>
}
```

### Dioxus vs Yew

Yew는 싱글 페이지 웹 앱을 구축하기 위한 프레임워크로, 처음에는 Dioxus에 영감을 주었습니다. 불행히도 Yew의 아키텍처는 우리가 원하는 다양한 기능을 지원하지 않아 Dioxus가 탄생하게 되었습니다.

- **싱글 페이지 앱**: Yew는 싱글 페이지 웹 앱 전용으로 설계되었으며 웹 플랫폼에 본질적으로 묶여 있습니다. Dioxus는 풀스택 및 크로스 플랫폼으로, 웹, 데스크톱, 모바일, 서버 앱을 구축하는 데 적합합니다.

- **개발자 도구**: Dioxus는 자동 포맷팅, 핫 리로딩, 번들러 등 다양한 유틸리티를 제공합니다.

- **지속적인 지원**: Dioxus는 새로운 기능과 버그 수정이 매일 추가되며 매우 적극적으로 유지 관리되고 있습니다.

### Dioxus vs egui

egui는 [Rerun.io](https://www.rerun.io)와 같은 도구를 구동하는 Rust용 크로스 플랫폼 GUI 라이브러리입니다.

- **Immediate vs Retained**: egui는 매 프레임마다 리렌더링되도록 설계되었습니다. 이는 게임 및 기타 인터랙티브 애플리케이션에 적합하지만, 프레임 간에 스타일 및 레이아웃 상태를 유지하지 않습니다. Dioxus는 UI가 한 번 구축된 후 프레임 간에 수정되는 retained UI 프레임워크입니다. 이를 통해 Dioxus는 HTML 및 CSS와 같은 네이티브 웹 기술을 사용하여 더 나은 배터리 수명과 성능을 제공합니다.

- **커스터마이징 가능**: egui는 자체 스타일링 및 레이아웃 솔루션을 제공하는 반면, Dioxus는 내장된 HTML 및 CSS 사용을 기대합니다. 이를 통해 Dioxus 앱은 Tailwind 또는 Material UI와 같은 CSS 라이브러리를 사용할 수 있습니다.

- **상태 관리**: egui의 상태 관리는 단일 전역 상태 객체를 기반으로 합니다. Dioxus는 컴포넌트와 props를 사용하여 상태의 캡슐화를 장려하여 컴포넌트의 재사용성을 높입니다.

### Dioxus vs Iced

Iced는 Elm에서 영감을 받은 크로스 플랫폼 GUI 라이브러리입니다. Iced는 WGPU로 네이티브 렌더링을 지원하며, DOM 노드를 사용하여 웹을 지원합니다.

- **Elm 상태 관리**: Iced는 메시지 전달과 리듀서를 기반으로 하는 Elm의 상태 관리 모델을 사용합니다. 이는 Dioxus와는 단순히 다른 상태 관리 모델이며, 때로는 다소 장황할 수 있습니다.

- **네이티브 느낌**: Dioxus는 렌더러로 webview를 사용하기 때문에 네이티브 텍스트 입력, 붙여넣기 처리, 접근성과 같은 기타 네이티브 기능을 자동으로 지원합니다. 현재 Iced의 렌더러는 이러한 기능을 구현하지 않아 덜 네이티브하게 느껴집니다.

- **WGPU**: Dioxus의 WGPU 렌더러는 현재 상당히 미성숙하며 아직 프로덕션 사용에 준비되지 않았습니다. Iced의 WGPU 렌더러는 훨씬 더 성숙하며 프로덕션에서 사용되고 있습니다. 이는 GPU 접근이 필요한 특정 유형의 앱을 Iced로 구축할 수 있게 해주지만, 현재 Dioxus로는 구축할 수 없습니다.

### Dioxus vs Electron

Dioxus와 Electron은 유사한 목표를 가진 완전히 다른 두 프로젝트입니다. Electron은 개발자가 HTML, CSS, JavaScript와 같은 웹 기술을 사용하여 크로스 플랫폼 데스크톱 앱을 구축할 수 있도록 합니다.

- **경량화**: Dioxus는 시스템의 네이티브 WebView 또는 선택적으로 WGPU 렌더러를 사용하여 UI를 렌더링합니다. 이로 인해 일반적인 Dioxus 앱은 macOS에서 약 15MB인 반면, Electron은 100MB입니다. Electron은 또한 Dioxus와 동일한 방식으로 호스트 OS와 시스템 리소스를 공유할 수 없는 내장된 Chromium 인스턴스를 포함하고 있습니다.

- **성숙도**: Electron은 대규모 커뮤니티와 많은 도구를 갖춘 성숙한 프로젝트입니다. Dioxus는 Electron에 비해 아직 꽤 젊은 편입니다. deeplinking과 같은 기능을 구현하기 위해 추가 작업이 필요할 수 있습니다.

## 기여하기

- 웹사이트의 [기여 섹션](https://dioxuslabs.com/learn/0.7/beyond/contributing)을 확인하세요.
- [이슈 트래커](https://github.com/dioxuslabs/dioxus/issues)에 이슈를 보고하세요.
- [Discord](https://discord.gg/XgGxMSkvUM)에 참여하여 질문하세요!

<a href="https://github.com/dioxuslabs/dioxus/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=dioxuslabs/dioxus&max=30&columns=10" />
</a>

## 라이선스

이 프로젝트는 [MIT 라이선스] 또는 [Apache-2 라이선스] 중 하나로 라이선스됩니다.

[apache-2 라이선스]: https://github.com/DioxusLabs/dioxus/blob/master/LICENSE-APACHE
[mit 라이선스]: https://github.com/DioxusLabs/dioxus/blob/master/LICENSE-MIT

별도로 명시하지 않는 한, 귀하가 Dioxus에 포함되도록 의도적으로 제출한 모든 기여는 추가 조건 없이 MIT 또는 Apache-2 라이선스로 라이선스됩니다.
