<p>
    <p align="center" >
      <!-- <img src="./notes/header-light-updated.svg#gh-light-mode-only" >
      <img src="./notes/header-dark-updated.svg#gh-dark-mode-only" > -->
      <!-- <a href="https://dioxuslabs.com">
          <img src="./notes/flat-splash.avif">
      </a> -->
      <img src="../../notes/splash-header-darkmode.svg#gh-dark-mode-only" style="width: 80%; height: auto;">
      <img src="../../notes/splash-header.svg#gh-light-mode-only" style="width: 80%; height: auto;">
      <img src="../../notes/image-splash.avif">
      <br>
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
    <a href="https://dioxuslabs.com"> 官网 </a>
    <span> | </span>
    <a href="https://github.com/DioxusLabs/dioxus/tree/main/examples"> 示例 </a>
    <span> | </span>
    <a href="https://dioxuslabs.com/learn/0.7/tutorial"> 指南 </a>
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
<p align="center">
  <a href="https://dioxuslabs.com/blog/release-060/">✨ Dioxus 0.6 已经发布 - 在此查看! ✨</a>
</p>
<br>

使用单一代码库构建 Web、桌面端、移动端以及更多平台的应用。零配置、集成热重载以及基于信号 (Signals) 的状态管理。通过服务器函数添加后端功能，并使用我们的 CLI 进行打包。

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

## ⭐️ 独特功能:

- 三行代码即可构建跨平台应用（Web、桌面、移动端、服务器等）
- [符合人体工程学的状态管理](https://dioxuslabs.com/blog/release-050)，结合了 React、Solid 和 Svelte 的优点
- 类型安全的路由和服务器函数，由 Rust 强大的编译时能力保证
- 集成 Web、MacOS、Linux 和 Windows 的打包工具
- 还有更多！ [开启 Dioxus 之旅](https://dioxuslabs.com/learn/0.7/).

## 即时热重载

只需一个命令 `dx serve`，你的应用即可运行。编辑标记（markup）和样式并立刻看到结果。

<div align="center">
  <img src="https://raw.githubusercontent.com/DioxusLabs/screenshots/refs/heads/main/blitz/hotreload-video.webp">
  <!-- <video src="https://private-user-images.githubusercontent.com/10237910/386919031-6da371d5-3340-46da-84ff-628216851ba6.mov" width="500"></video> -->
  <!-- <video src="https://private-user-images.githubusercontent.com/10237910/386919031-6da371d5-3340-46da-84ff-628216851ba6.mov" width="500"></video> -->
</div>

## 一流的 Android 和 iOS 支持

Dioxus 是使用 Rust 构建原生移动应用的最快方式。只需运行 `dx serve --platform android`，你的应用就会几秒钟在模拟器或实机设备上运行，并能够直接调用 JNI 和原生 API。

<div align="center">
  <img src="../../notes/android_and_ios2.avif" width="500">
</div>

## 为 Web、桌面和移动端打包

只需运行 `dx bundle`，你的应用将被构建和打包，并进行最大优化。
在 Web 端，得益于 [`.avif` 生成, `.wasm` 压缩, 代码最小化](https://dioxuslabs.com/learn/0.7/tutorial/assets)等功能， Web 端的构建[体积小于 50kb](https://github.com/ealmloff/tiny-dioxus/) ，桌面/移动端体积小于 5mb。

<div align="center">
  <img src="../../notes/bundle.gif">
</div>

## 出色的文档

我们投入了大量精力来构建清晰、易读且全面的文档。所有 HTML 元素和监听器都附有 MDN 文档，并且我们的文档网站与 Dioxus 本身进行持续集成，以确保文档始终保持最新。请查看 [Dioxus 网站](https://dioxuslabs.com/learn/0.7/) 获取指南、参考、示例代码等。
有趣的是：我们使用 Dioxus 网站来测试 Dioxus 的新功能 - [快来看看吧!](https://github.com/dioxusLabs/docsite)

<div align="center">
  <img src="../../notes/docs.avif">
</div>

## 社区

Dioxus 是一个社区驱动的项目，拥有非常活跃的 [Discord](https://discord.gg/XgGxMSkvUM) 和 [GitHub](https://github.com/DioxusLabs/dioxus/issues) 社区。 我们一直在寻求帮助，乐于回答问题并帮助你入门。[我们的 SDK](https://github.com/DioxusLabs/dioxus-std) 由社区运营，我们甚至有一个 [GitHub 组织](https://github.com/dioxus-community/) 用于那些获得免费升级和支持的最佳 Dioxus crates。

<div align="center">
  <img src="../../notes/dioxus-community.avif">
</div>

## 全职核心团队

Dioxus 已从一个业余项目发展成为一个小型的全职工程师团队。感谢 FutureWei、Satellite.im 和 GitHub Accelerator 项目的慷慨支持，我们能够全职投入 Dioxus 的开发。我们的长远目标是通过提供高质量的付费企业级工具，使 Dioxus 实现自我维持。如果您的公司有兴趣使用 Dioxus 并希望与我们合作，请联系我们！

## 支持的平台

<div align="center">
  <table style="width:100%">
    <tr>
      <td>
      <b>Web 端</b>
      </td>
      <td>
        <ul>
          <li>使用 WebAssembly 直接渲染到 DOM</li>
          <li>通过 SSR 进行预渲染并在客户端进行水合 (rehydrate)</li>
          <li>简单的 "hello world" 约 50kb，与 React 相当</li>
          <li>内置的开发服务器和热重载以实现快速迭代</li>
        </ul>
      </td>
    </tr>
    <tr>
      <td>
      <b>桌面端</b>
      </td>
      <td>
        <ul>
          <li>使用 Webview 渲染，或实验性地使用 WGPU 或  <a href="https://freyaui.dev">Freya</a> (Skia) 渲染</li>
          <li>零配置，只需 `cargo run` 或 `dx serve` 即可构建您的应用 </li>
          <li>完全支持原生系统访问，无需 IPC </li>
          <li>支持 MacOS, Linux 和 Windows，或者便携式的 <3mb 的二进制文件 </li>
        </ul>
      </td>
    </tr>
    <tr>
      <td>
      <b>移动端</b>
      </td>
      <td>
        <ul>
          <li>使用 Webview 渲染，或实验性地使用 WGPU 或 Skia 渲染</li>
          <li>为 iOS 和 Android 构建 .ipa 和 .apk 文件</li>
          <li>以最小的开销直接调用 Java 和 Objective-C</li>
          <li>从 "hello world" 到在实机设备上运行仅需几秒钟</li>
        </ul>
      </td>
    </tr>
    <tr>
      <td>
      <b>服务器端渲染</b>
      </td>
      <td>
        <ul>
          <li>Suspense，水合 (hydration) 和服务器端渲染</li>
          <li>通过服务器函数快速添加后端功能</li>
          <li>集成提取器（extractor）, 中间件（middleware）和路由</li>
          <li>静态站点生成和增量生成</li>
        </ul>
      </td>
    </tr>
  </table>
</div>

## 运行示例

> 本仓库 main 分支中的示例针对的是 Dioxus 的 git 版本和 CLI。如果您正在寻找适用于 Dioxus 最新稳定版本的示例，请查看 [0.6 分支](https://github.com/DioxusLabs/dioxus/tree/v0.6/examples)。

本仓库顶层目录中的示例可以通过以下命令运行：

```sh
cargo run --example <example>
```

但是，我们鼓励你下载 dioxus-cli。如果你在使用 Dioxus 的 git 版本，可以使用以下命令安装相应版本的 CLI：

```sh
cargo install --git https://github.com/DioxusLabs/dioxus dioxus-cli --locked
```

你也使用 CLI 在 Web 平台上运行示例。只需通过如下命令禁用默认的桌面特性并启用 Web 特性：

```sh
dx serve --example <example> --platform web -- --no-default-features
```

## Dioxus 与其他框架的比较

我们热爱 Rust 生态系统中的所有框架，并享受着他们所带来的创新。实际上，我们的许多项目都与其他框架共享。例如，我们的 flex-box 库 [Taffy](https://github.com/DioxusLabs/taffy) 被 [Bevy](https://bevyengine.org/), [Zed](https://zed.dev/), [Lapce](https://lapce.dev/), [Iced](https://github.com/iced-rs/iced) 等许多项目使用。

Dioxus 强调以下几个关键点，使其与其他框架有所不同：

- **类 React**: 我们依赖组件、props 和 hooks 等概念来构建 UI，我们的状态管理更接近 Svelte 而非 SolidJS。
- **HTML 和 CSS**: 我们完全拥抱 HTML 和 CSS，包括其所有特性和怪癖。
- **渲染器无关**: 得益于[我们快速的虚拟 DOM](https://dioxuslabs.com/blog/templates-diffing)，你可以为任何你想要的平台替换渲染器。
- **协作性**: 只要有可能，我们就会将像 [Taffy](https://github.com/DioxusLabs/taffy), [manganis](https://github.com/DioxusLabs/manganis), [include_mdbook](https://github.com/DioxusLabs/include_mdbook), 和 [blitz](http://github.com/dioxusLabs/blitz) 这样的 crates 分离出来，以便生态系统能够共同成长。

### Dioxus vs Tauri

Tauri 是一个用于构建桌面移动应用的框架，其前端使用基于 Web 的框架（如 React、Vue、Svelte 等）。当你需要进行原生操作时，可以编写 Rust 函数并从前端调用它们。

- **原生 Rust**: Tauri 的架构将您的 UI 限制在 JavaScript 或 WebAssembly。使用 Dioxus，你的 Rust 代码可以在用户的设备上原生运行，在你执行像创建线程、访问文件系统等操作时，无需任何 IPC 桥接。这极大地简化了您的应用架构，使其更易于构建。如果您愿意，也可以使用 Dioxus-Web 作为前端来构建 Tauri 应用。

- **不同的范围**: Tauri 需要支持 JavaScript 及其复杂的构建工具链，这限制了您可以用它做的事情的范围。由于 Dioxus 专注于 Rust，我们能够提供额外的实用工具，如服务器函数、高级打包和原生渲染器。

- **共享 DNA**: 虽然 Tauri 和 Dioxus 是独立的项目，但它们确实共享诸如 Tao 和 Wry 这样的库：由 Tauri 团队维护的窗口和 Webview 库。

### Dioxus vs Leptos

Leptos 是一个用于构建全栈 Web 应用的库，类似于 SolidJS 和 SolidStart。这两个库在 Web 端有着相似的目标，但存在一些关键差异：

- **响应式模型**: Leptos 使用信号 (signals) 来驱动响应和渲染，而 Dioxus 仅将信号用于响应。为了管理重新渲染，Dioxus 使用了高度优化的虚拟 DOM 来支持桌面和移动端架构。Dioxus 和 Leptos 都非常快。

- **不同的范围**: Dioxus 为 Web、桌面端、移动端、LiveView 等提供渲染器。我们还维护社区库和跨平台 SDK。 Leptos 更专注于全栈 Web，并拥有一些 Dioxus 所没有的功能，比如 islands、`<Form />` 组件和其他 Web 特有的实用工具.

- **不同的 DSL**: Dioxus 使用其自定义的类似 Rust 的 DSL 来构建 UI，而 Leptos 使用类似 HTML 的语法。我们选择这样做是为了保持与 IDE 功能（如代码折叠和语法高亮）的兼容性。通常，Dioxus 的 DSL 倾向于更多的“魔法”，包括字符串的自动格式化和简单 Rust 表达式的热重载。

```rust
// dioxus
rsx! {
  div {
    class: "my-class",
    enabled: true,
    "Hello, {name}"
  }
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

Yew 是一个用于构建单页 Web 应用的框架，最初是 Dioxus 的灵感来源。不幸的是，Yew 的架构不支持我们想要的各种功能，因此 Dioxus 应运而生。

- **单页应用**: Yew 专为单页 Web 应用设计，并与 Web 平台紧密绑定。Dioxus 是全栈和跨平台的，使其适用于构建 Web、桌面、移动端和服务器应用。

- **开发者工具**: Dioxus 提供了许多实用工具，如自动格式化、热重载和打包工具。

- **持续支持**: Dioxus 得到非常积极的维护，每天都在添加新功能和修复错误。

### Dioxus vs egui

egui 是一个跨平台 GUI 库，为 Rust 驱动的工具提供支持，比如 [Rerun.io](https://www.rerun.io)。

- **立即模式 (Immediate) vs 保留模式 (Retained)**: egui 设计为每一帧都重新渲染。这更适用于游戏和其他交互式应用，但它不会在帧之间保留样式和布局状态。Dioxus 是一个保留模式的 UI 框架，这意味着 UI 构建一次后，在帧之间进行修改。这使得 Dioxus 能够使用原生的 Web 技术（如 HTML 和 CSS），并具有更好的电池寿命和性能。

- **可定制性**: egui 带来了自己的样式和布局解决方案，而 Dioxus 则希望您使用内置的 HTML 和 CSS。这使得 Dioxus 应用可以使用任何 CSS 库，例如 Tailwind 或 Material UI。

- **状态管理**: egui 的状态管理基于单个全局状态对象。Dioxus 鼓励通过使用组件和 props 来封装状态，使组件更具可复用性。

### Dioxus vs Iced

Iced 是一个受 Elm 启发的跨平台 GUI 库。Iced 使用 WGPU 进行原生渲染，并使用 DOM 节点支持 Web。

- **Elm 状态管理**: Iced 使用 Elm 的状态管理模型，该模型基于消息传递和 reducers。这不同于 Dioxus 的状态管理模型，它有时可能会相当冗长。

- **原生体验**: 由于 Dioxus 使用 Webview 作为其渲染器，它自动获得了原生文本输入、粘贴处理以及其他原生功能（比如无障碍）。Iced 的渲染器目前尚未实现这些功能，使其感觉不那么原生。

- **WGPU**: Dioxus 的 WGPU 渲染器目前还很不成熟，尚未准备好用于生产环境。Iced 的 WGPU 渲染器则成熟得多，并已在生产中使用。这使得某些需要 GPU 访问的应用可以使用 Iced 构建，而目前无法使用 Dioxus 构建。

### Dioxus vs Electron

Dioxus 和 Electron 是两个目标相似但完全不同的项目。Electron 使开发人员能够使用 HTML、CSS 和 JavaScript 等 Web 技术构建跨平台桌面应用。

- **轻量级**: Dioxus 使用系统原生 Webview（或者可选 WGPU 渲染器）来渲染 UI。这使得典型的 Dioxus 应用在 macOS 上大约为 15MB，而 Electron 则为 100MB。Electron 还捆绑了一个嵌入式的 Chromium 实例，它无法像 Dioxus 那样与宿主操作系统共享系统资源。

- **成熟度**: Electron 是一个成熟的项目，拥有庞大的社区和许多工具。相比，Dioxus 与 Electron 相比还很年轻。如果遇到诸如深层链接 (deep-linking) 之类的功能，这些功能需要额外的工作来实现。

## 贡献

- 查看网站上[关于贡献的部分](https://dioxuslabs.com/learn/0.7/beyond/contributing).
- 在我们的 [issues](https://github.com/dioxuslabs/dioxus/issues) 上报告问题.
- [加入](https://discord.gg/XgGxMSkvUM) Discord 并提问！

<a href="https://github.com/dioxuslabs/dioxus/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=dioxuslabs/dioxus&max=30&columns=10" />
</a>

## 开源许可

此项目基于 [MIT license] 或 [Apache-2 License] 发布。

[apache-2 license]: https://github.com/DioxusLabs/dioxus/blob/master/LICENSE-APACHE
[mit license]: https://github.com/DioxusLabs/dioxus/blob/master/LICENSE-MIT

除非您明确声明，否则您有意提交给 Dioxus 的任何贡献，均应按 MIT 或 Apache-2 授权，不附带任何附加条款或条件。
