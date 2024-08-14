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
    <a href="https://dioxuslabs.com"> 官方网站 </a>
    <span> | </span>
    <a href="https://github.com/DioxusLabs/example-projects"> 示例 </a>
    <span> | </span>
    <a href="https://dioxuslabs.com/learn/0.5/guide"> 指南 </a>
    <span> | </span>
    <a href="https://github.com/DioxusLabs/dioxus/blob/main/README.md"> English </a>
    <span> | </span>
    <a href="https://github.com/DioxusLabs/dioxus/blob/main/translations/pt-br/README.md"> PT-BR </a>
    <span> | </span>
    <a href="https://github.com/DioxusLabs/dioxus/blob/main/translations/ja-jp/README.md"> 日本語 </a>
    <span> | </span>
    <a href="https://github.com/DioxusLabs/dioxus/blob/main/translations/tr-tr"> Türkçe </a>
  </h3>
</div>
<br>
<br>

使用单个代码库构建 Web、桌面和移动应用，以及更多。零配置启动、集成的热重载和基于信号的状态管理。使用服务器功能添加后端功能，并使用我们的 CLI 进行捆绑。

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

## ⭐️ 独特功能：

- 三行代码即可实现跨平台应用（Web、桌面、移动、服务器等）
- [人体工程学的状态管理](https://dioxuslabs.com/blog/release-050)，结合了 React、Solid 和 Svelte 的优点
- 非常高效，由 Rust 最快的 wasm 框架 [sledgehammer](https://dioxuslabs.com/blog/templates-diffing) 提供支持
- 集成的捆绑包，可部署到 Web、macOS、Linux 和 Windows
- 还有更多！阅读 [Dioxus 漫游之旅](https://dioxuslabs.com/learn/0.5/)。

## 瞬时热重载

使用一个命令 `dx serve`，您的应用程序即可运行。编辑您的标记和样式，并实时查看结果。目前 Rust 代码热重载尚未达到一流水平，但也许可以使用 [hot-lib-reloader](https://docs.rs/hot-lib-reloader/latest/hot_lib_reloader/) 实现。

<div align="center">
  <img src="../../notes/hotreload.gif">
</div>

## 用于 Web 和桌面部署的捆绑包

只需运行 `dx bundle`，您的应用程序将被构建和捆绑，并进行最大化的优化。在 Web 上，利用 [`.avif` 生成，`.wasm` 压缩，代码缩小](https://dioxuslabs.com/learn/0.5/reference/assets)，等等。构建的 Web 应用程序大小 [不到 50kb](https://github.com/ealmloff/tiny-dioxus/)，桌面/移动应用程序小于 15mb。

<div align="center">
  <img src="../../notes/bundle.gif">
</div>

## 出色的文档

我们花费了大量精力构建清晰、易读和全面的文档。所有 HTML 元素和监听器都使用 MDN 文档进行了记录，并且我们的文档站点通过 Dioxus 本身进行持续集成，以确保文档始终保持最新。查看 [Dioxus 网站](https://dioxuslabs.com/learn/0.5/) 获取指南、参考资料、示例和更多信息。有趣的事实：我们将 Dioxus 网站用作新特性的测试平台 - [来看看吧！](https://github.com/dioxusLabs/docsite)

<div align="center">
  <img src="../../notes/docs.avif">
</div>

## 开发者体验的重点

Dioxus 优先考虑开发者体验，我们为端到端工具链付出了大量努力。我们构建了一个 [VSCode 扩展](https://marketplace.visualstudio.com/items?itemName=DioxusLabs.dioxus)，可以自动格式化您的 RSX 代码，将 HTML 转换为 RSX 等等。我们还建立了一个非常强大的 [CLI](https://dioxuslabs.com/learn/0.5/CLI)，支持创建新应用程序、提供服务和跨平台捆绑，并且部署正在路上。

<div align="center">
  <img src="./notes/autofmt.gif">
</div>

## 社区

Dioxus 是一个社区驱动的项目，拥有非常活跃的 [Discord](https://discord.gg/XgGxMSkvUM) 和 [GitHub](https://github.com/DioxusLabs/dioxus/issues) 社区。我们一直在寻求帮助，乐意回答问题并帮助您入门。 [我们的 SDK](https://github.com/DioxusLabs/dioxus-std) 由社区运行，我们甚至有一个 [GitHub 组织](https://github.com/dioxus-community/)，用于最好的 Dioxus crates，这些 crate 可以获得免费升级和支持。

<div align="center">
  <img src="../../notes/dioxus-community.avif">
</div>

## 全职核心团队

Dioxus 已经从一个副业项目发展成为一个由全职工程师组成的小团队。由于 FutrueWei、Satellite.im 和 GitHub Accelerator 项目的慷慨支持，我们能够全职工作在 Dioxus 上。我们的长期目标是通过提供高质量的付费企业工具使 Dioxus 自我维持。如果您的公司有兴趣采用 Dioxus，并希望与我们合作，请联系我们！

## 支持的平台

<div align="center">
  <table style="width:100%">
    <tr>
      <td>
      <b>Web</b>
      <br />
      <em>一级支持</em>
      </td>
      <td>
        <ul>
          <li>使用WebAssembly直接渲染到DOM</li>
          <li>在服务器端预渲染，然后在客户端重新hydrate</li>
          <li>简单的 "hello world" 约为50kb，与React相当</li>
          <li>内置的开发服务器和热重载以进行快速迭代</li>
        </ul>
      </td>
    </tr>
    <tr>
      <td>
      <b>Fullstack</b>
      <br />
      <em>一级支持</em>
      </td>
      <td>
        <ul>
          <li>Suspense、hydration 和服务器端渲染</li>
          <li>快速添加后端功能与服务器功能</li>
          <li>提取器、中间件和路由集成</li>
          <li>兼容桌面和移动设备！</li>
        </ul>
      </td>
    </tr>
    <tr>
      <td>
      <b>桌面</b>
      <br />
      <em>一级支持</em>
      </td>
      <td>
        <ul>
          <li>使用Webview渲染，或者实验性地使用WGPU或 <a href="https://freyaui.dev">Freya</a>（skia） </li>
          <li>零配置设置。只需 `cargo run` 或 `dx serve` 即可构建您的应用程序 </li>
          <li>完全支持无IPC的本机系统访问 </li>
          <li>支持macOS、Linux和Windows。便携式 <3mb 二进制文件 </li>
        </ul>
      </td>
    </tr>
    <tr>
      <td>
      <b>Liveview</b>
      <br />
      <em>一级支持</em>
      </td>
      <td>
        <ul>
          <li>完全在服务器上渲染应用程序 - 或仅渲染一个组件</li>
          <li>与流行的Rust框架（如Axum和Warp）集成</li>
          <li>极低的延迟和支持10,000个以上的同时应用程序</li>
        </ul>
      </td>
    </tr>
    <tr>
      <td>
      <b>移动</b>
      <br />
      <em>二级支持</em>
      </td>
      <td>
        <ul>
          <li>使用Webview渲染，或者实验性地使用WGPU或Skia </li>
          <li>支持iOS和Android </li>
          <li>目前还处于相当实验性阶段，2024年将有大量改进 </li>
        </ul>
      </td>
    </tr>
    <tr>
      <td>
      <b>终端</b>
      <br />
      <em>二级支持</em>
      </td>
      <td>
        <ul>
          <li>直接将应用程序渲染到终端，类似于 <a href="https://github.com/vadimdemedes/ink"> ink.js</a></li>
          <li>由浏览器的熟悉的flexbox和CSS模型提供支持</li>
          <li>内置小部件，如文本输入、按钮和焦点系统</li>
        </ul>
      </td>
    </tr>
  </table>
</div>

## 运行示例

该存储库顶层的示例可以通过 `cargo run --example <example>` 运行。但是，我们建议您下载 dioxus-cli 并使用 `dx serve` 运行示例，因为许多示例还支持 web。在运行 web 时，您可以修改 Cargo.toml 以禁用默认的 desktop 功能，或者使用

## Dioxus 与其他框架

我们喜欢 Rust 生态系统中的所有框架，并享受着 Rust 生态系统中的创新。事实上，我们的许多项目都与其他框架共享。例如，我们的 flex-box 库 [Taffy](https://github.com/DioxusLabs/taffy) 被 [Bevy](https://bevyengine.org/)、[Zed](https://zed.dev/)、[Lapce](https://lapce.dev/)、[Iced](https://github.com/iced-rs/iced) 等项目使用。

Dioxus 侧重于几个与其他框架不同的重点，这些重点使其与其他框架不同：

- **类似 React**：我们依赖于组件、props 和 hooks 等概念来构建 UI，我们的状态管理更接近于 Svelte 而不是 SolidJS。
- **HTML 和 CSS**：我们完全依赖 HTML 和 CSS，包括怪癖和全部。
- **渲染器不可知**：您可以根据需要将渲染器替换为任何平台，感谢我们的快速 VirtualDOM。
- **合作**：只要有可能，我们就会拆分出像[Taffy](https://github.com/DioxusLabs/taffy)、[magnanis](https://github.com/DioxusLabs/manganis)、[include_mdbook](https://github.com/DioxusLabs/include_mdbook)和[blitz](http://github.com/dioxusLabs/blitz)这样的 crate，以便生态系统能够共同成长。

### Dioxus 与 Tauri

Tauri 是一个用于构建桌面（和即将到来的移动）应用程序的框架，其中前端使用诸如 React、Vue、Svelte 等的基于 web 的框架编写。每当需要进行本机工作时，您可以编写 Rust 函数并从前端调用它们。

- **纯 Rust**：Tauri 的架构将您的 UI 限制为 JavaScript 或 WebAssembly。使用 Dioxus，您的 Rust 代码在用户的计算机上本地运行，使您可以执行诸如生成线程、访问文件系统等操作，而无需任何 IPC。这大大简化了应用程序的架构，并使其更易于构建。您可以使用 Dioxus-Web 作为前端构建 Tauri 应用程序。

- **不同的范围**：Tauri 需要支持 JavaScript 及其复杂的构建工具链，从而限制了您可以使用的范围。由于 Dioxus 专注于 Rust，我们能够提供额外的实用工具，如服务器函数、高级捆绑和本机渲染器。

- **共享 DNA**：虽然 Tauri 和 Dioxus 是独立的项目，但它们确实共享库，如 Tao 和 Wry：由 Tauri 团队维护的窗口和 webview 库。

### Dioxus 与 Leptos

Leptos 是用于构建全栈 web 应用程序的库，类似于 SolidJS 和 SolidStart。两个库在 web 上有着相似的目标，但有几个关键区别：

- **响应模型**：Leptos 使用信号作为其基础的响应性，而 Dioxus 选择使用 VirtualDom 和重新渲染。虽然从理论上讲，信号更高效，但实际上，由于我们的[block-dom](https://dioxuslabs.com/blog/templates-diffing)模板并没有进行实际的 diffing，而是直接重新渲染了 UI，Dioxus 的 VirtualDom 的表现与 Leptos 相比几乎没有什么差异，并且[实际上比 Leptos 更快](https://krausest.github.io/js-framework-benchmark/2024/table_chrome_123.0.6312.59.html)。

- **控制流**：因为 Leptos 使用信号来实现响应性，所以您只能使用 Leptos 的原语来进行 `for` 循环和 `if` 语句。如果您搞错了这一点，您的应用程序将失去响应性，导致难以调试的 UI 问题。使用 Dioxus，您可以使用迭代器、常规的 Rust `for` 循环和 `if` 语句，您的应用程序仍然是响应式的。在实践中，一个向列表中插入计数器的 Dioxus 组件可能如下所示：

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

[在 Leptos 中，您将使用`<For>`组件。](https://book.leptos.dev/view/04_iteration.html#dynamic-rendering-with-the-for-component):

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

- **`Copy` 状态**: Dioxus 0.1 到 0.4 版本依赖于生命周期来放宽 Rust 的借用检查规则。这对于事件处理程序效果很好，但在异步方面表现不佳。在 Dioxus 0.5 中，我们已经转向了从 Leptos 借鉴来的 [`Copy` 状态模型](https://crates.io/crates/generational-box)。

- **不同的范围**: Dioxus 提供了用于 web、桌面、移动、LiveView 等的渲染器。我们还维护着社区库和跨平台 SDK。这项工作的范围很大，意味着我们的发布速度历来比 Leptos 慢。Leptos 专注于全栈 web，具有 Dioxus 没有的功能，如基于 `<Suspense />` 的流式 HTML、islands、`<Form />` 组件和其他特定于 web 的功能。通常情况下，您使用 Leptos 构建的 web 应用程序将占用较小的空间。

- **不同的 DSL**: 虽然两个框架都针对 web，但 Dioxus 使用自己的定制 Rust 风格的 DSL 来构建 UI，而 Leptos 使用更类似 HTML 的语法。我们选择了这样做以保留与代码折叠和语法高亮等 IDE 功能的兼容性。通常情况下，Dioxus 倾向于在其 DSL 中使用更多的 "魔法"。例如，dioxus 将自动为您格式化字符串，而 Leptos 可以将字符串拆分为静态和动态段。

### Dioxus vs Yew

Yew 是用于构建单页面 web 应用程序的框架，最初是 Dioxus 的灵感来源。不幸的是，Yew 的架构不支持我们想要的各种功能，因此 Dioxus 应运而生。

- **单页面应用程序**: Yew 专门设计用于单页面 web 应用程序，并与 web 平台紧密相关联。Dioxus 是全栈和跨平台的，适用于构建 web、桌面、移动和服务器应用程序。

- **开发工具**: Dioxus 提供了许多实用工具，如自动格式化、热重载和打包工具。

- **持续支持**: Dioxus 在日常基础上积极维护，不断添加新功能和修复错误。

### Dioxus vs egui

egui 是 Rust 的跨平台 GUI 库，支持像 [Rerun.io](https://www.rerun.io) 这样的工具。

- **Immediate vs Retained**: egui 被设计为在每一帧上重新呈现。这适用于游戏和其他交互式应用程序，但它不会在帧之间保留样式和布局状态。Dioxus 是一个保留的 UI 框架，意味着 UI 只构建一次，然后在帧之间进行修改。这使得 Dioxus 可以使用本地 web 技术，如 HTML 和 CSS，具有更好的电池寿命和性能。

- **可定制性**: egui 带有自己的样式和布局解决方案，而 Dioxus 期望您使用内置的 HTML 和 CSS。这使得 dioxus 应用程序可以使用任何 CSS 库，如 Tailwind 或 Material UI。

- **状态管理**: egui 的状态管理基于单个全局状态对象。Dioxus 鼓励通过使用组件和属性封装状态，使组件更可重用。

### Dioxus vs Iced

Iced 是受 Elm 启发的跨平台 GUI 库。Iced 使用 WGPU 进行本机渲染，并支持使用 DOM 节点的 web。

- **Elm 状态管理**: Iced 使用 Elm 的状态管理模型，该模型基于消息传递和减速器。这与 Dioxus 的状态管理模型完全不同，有时可能相当冗长。

- **本地感觉**: 由于 Dioxus 使用 webview 作为其渲染器，因此它自动获得本地文本输入、粘贴处理和其他本地功能，如可访问性。Iced 的渲染器目前不实现这些功能，使其感觉不太本地。

- **WGPU**: Dioxus 的 WGPU 渲染器目前还不太成熟，尚未准备好供生产使用。Iced 的 WGPU 渲染器要成熟得多，并且正在生产中使用。这使得某些需要 GPU 访问的应用程序可以使用 Iced 构建，而目前无法使用 Dioxus 构建。

### Dioxus vs Electron

Dioxus 和 Electron 是两个完全不同的项目，目标相似。Electron 使开发人员能够使用 HTML、CSS 和 JavaScript 等 web 技术构建跨平台桌面应用程序。

- **轻量级**: Dioxus 使用系统的本地 WebView - 或者选择性地，一个 WGPU 渲染器 - 来渲染 UI。这使得典型的 Dioxus 应用程序在 macOS 上约为 15mb，而 Electron 则为 100mb。Electron 还提供了一个嵌入式的 chromium 实例，无法像 Dioxus 那样与主机操作系统共享系统资源。

- **成熟度**: Electron 是一个成熟的项目，拥有庞大的社区和大量的工具。相比之下，Dioxus 相对于 Electron 来说还很年轻。期望遇到需要额外工作来实现的功能，如 deeplinking 等特性。

## 贡献

- 查看网站上有关[贡献的部分](https://dioxuslabs.com/learn/0.5/contributing)。
- 在我们的 [问题跟踪器](https://github.com/dioxuslabs/dioxus/issues) 上报告问题。
- [加入](https://discord.gg/XgGxMSkvUM) discord 并提出问题！

<a href="https://github.com/dioxuslabs/dioxus/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=dioxuslabs/dioxus&max=30&columns=10" />
</a>

## 许可证

此项目根据 [MIT 许可证] 许可。

[mit 许可证]: https://github.com/DioxusLabs/dioxus/blob/master/LICENSE-MIT

除非您明确说明，否则您提交的任何贡献都将由您自己有意提交，以 MIT 许可证许可，不附加任何其他条款或条件。
