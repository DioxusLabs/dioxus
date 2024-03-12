<p align="center">
  <img src="../../notes/header.svg">
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
    <a href="https://github.com/DioxusLabs/example-projects"> 代码示例 </a>
    <span> | </span>
    <a href="https://dioxuslabs.com/learn/0.4/guide"> 开发指南 </a>
    <span> | </span>
    <a href="https://github.com/DioxusLabs/dioxus/blob/master/README.md"> English </a>
    <span> | </span>
    <a href="https://github.com/DioxusLabs/dioxus/blob/master/translations/pt-br/README.md"> PT-BR </a>
    <span> | </span>
    <a href="https://github.com/DioxusLabs/dioxus/blob/master/translations/ja-jp/README.md"> 日本語 </a>
  </h3>
</div>

<br/>

Dioxus 是一个可移植的、高性能的、符合人体工程学的框架，使用 Rust 语言构建跨平台的用户界面。

```rust
fn app() -> Element {
    let mut count = use_signal(|| 0);

    rsx! {
        h1 { "High-Five counter: {count}" }
        button { onclick: move |_| count += 1, "Up high!" }
        button { onclick: move |_| count -= 1, "Down low!" }
    })
}
```

Dioxus 可用于生成 网页前端、桌面应用、静态网站、移动端应用、TUI程序、 liveview程序等多类平台应用，Dioxus完全与渲染器无关，可以作用于任何渲染平台。

如果你能够熟悉使用 React 框架，那 Dioxus 对你来说将非常简单。

## 独特的特性:
- 只需不到10行代码就能原生运行桌面程序（并非 Electron 的封装）
- 符合人体工程学的设计以及拥有强大的状态管理
- 全面的内联文档 - 包含所有 HTML 元素、监听器 和 事件 指南。
- 极快的运行效率🔥🔥和极高的内存效率
- 智能项目热重载以便快速迭代
- 使用协程和Suspense来进行一流的异步支持
- 更多内容请查看 [版本发布信息](https://dioxuslabs.com/blog/introducing-dioxus/).

## 已支持的平台
<div align="center">
  <table style="width:100%">
    <tr>
      <td><em>网站项目</em></td>
      <td>
        <ul>
          <li>使用 WebAssembly 直接对 DOM 进行渲染</li>
          <li>为 SSR 提供预渲染或作为客户端使用</li>
          <li>简单的 "Hello World" 仅仅 65kb, 媲美 React 框架</li>
          <li>内置开发服务和热重载支持，方便项目快速迭代</li>
        </ul>
      </td>
    </tr>
    <tr>
      <td><em>桌面应用</em></td>
      <td>
        <ul>
          <li>使用 Webview 进行渲染 或 使用 WGPU 和 Skia（试验性的）</li>
          <li>无多余配置，简单的使用 `cargo build` 即可快速构建</li>
          <li>对原生系统的全面支持</li>
          <li>支持 Macos、Linux、Windows 等系统，极小的二进制文件</li>
        </ul>
      </td>
    </tr>
    <tr>
      <td><em>移动端应用</em></td>
      <td>
        <ul>
          <li>使用 Webview 进行渲染 或 使用 WGPU 和 Skia（试验性的）</li>
          <li>支持 IOS 和 安卓系统</li>
          <li><em>显著的</em> 性能强于 React Native 框架 </li>
        </ul>
      </td>
    </tr>
    <tr>
      <td><em>Liveview</em></td>
      <td>
        <ul>
          <li>完全使用服务器渲染应用程序或单个组件</li>
          <li>与受欢迎的后端框架进行融合（Axum、Wrap）</li>
          <li>及低的延迟,可以同时支持10000个以上的终端程序</li>
        </ul>
      </td>
    </tr>
    <tr>
      <td><em>终端程序</em></td>
      <td>
        <ul>
          <li>在终端程序中渲染，类似于： <a href="https://github.com/vadimdemedes/ink"> ink.js</a></li>
          <li>支持 CSS 相关模型（类似于浏览器内的）</li>
          <li>内置类似文字输入，按钮和焦点系统的小组件</li>
        </ul>
      </td>
    </tr>
  </table>
</div>

## 为什么选择Dioxus?

目前有非常多的应用开发选择，为什么偏偏要选择 Dioxus 呢？

首先，Dioxus将开发者的体验放在首位。这体现在 Dioxus 特有的各种功能上。

- 自动格式化 RSX 格式代码，并拥有 VSCode 插件作为支持。
- 热加载基于 RSX 代码解析器，同时支持桌面程序和网页程序。
- 强调文档的重要性--我们的指南是完整的，并且我们对所有 HTML 元素都提供文档支持。

Dioxus 也是一个可扩展化的平台。

- 通过实现一个非常简单的优化堆栈机，轻松构建新的渲染器。
- 构建并分享开发者自定义的组件代码。

Dioxus 那么优秀，但什么时候它不适合我呢？
- 它还没有完全成熟。api仍在变化，可能会出现故障（尽管我们试图避免）
- 您需要运行在 no-std 的环境之中。
- 你不喜欢使用 React-like 的方式构建 UI 项目。


## 贡献代码
- 在我们的 [问题追踪](https://github.com/dioxuslabs/dioxus/issues) 中汇报你遇到的问题。
- [加入](https://discord.gg/XgGxMSkvUM)我们的 Discord 与我们交流。


<a href="https://github.com/dioxuslabs/dioxus/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=dioxuslabs/dioxus&max=30&columns=10" />
</a>

## 开源协议

本项目使用 [MIT license].

[mit license]: https://github.com/DioxusLabs/dioxus/blob/master/LICENSE-MIT

除非您另有明确声明，否则有意提交的任何贡献将被授权为 MIT 协议，没有任何附加条款或条件。
