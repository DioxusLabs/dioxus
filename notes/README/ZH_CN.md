<div align="center">
  <h1>🌗🚀 Dioxus</h1>
  <p>
    <strong>Frontend that scales.</strong>
  </p>
</div>

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
</div>

<div align="center">
  <!--Awesome -->
  <a href="https://github.com/dioxuslabs/awesome-dioxus">
    <img src="https://cdn.rawgit.com/sindresorhus/awesome/d7305f38d29fed78fa85652e3a63e154dd8e8829/media/badge.svg" alt="Awesome Page" />
  </a>
  <!-- Discord -->
  <a href="https://discord.gg/XgGxMSkvUM">
    <img src="https://badgen.net/discord/members/XgGxMSkvUM" alt="Awesome Page" />
  </a>
</div>


<div align="center">
  <h3>
    <a href="https://dioxuslabs.com"> 官网 </a>
    <span> | </span>
    <a href="https://dioxuslabs.com/guide"> 手册 </a>
    <span> | </span>
    <a href="https://github.com/DioxusLabs/example-projects"> 示例 </a>
  </h3>
</div>

<div align="center">
  <h4>
    <a href="https://github.com/DioxusLabs/dioxus/blob/master/README.md"> English </a>
    <span> | </span>
    <a href="https://github.com/DioxusLabs/dioxus/blob/master/README.md"> 中文 </a>
  </h3>
</div>


<br/>

Dioxus 是一个可移植、高性能的框架，用于在 Rust 中构建跨平台的用户界面。

```rust
fn app(cx: Scope) -> Element {
    let mut count = use_state(&cx, || 0);

    cx.render(rsx!(
        h1 { "High-Five counter: {count}" }
        button { onclick: move |_| count += 1, "Up high!" }
        button { onclick: move |_| count -= 1, "Down low!" }
    ))
}
```

Dioxus 可用于制作 网页程序、桌面应用、静态站点、移动端应用。

Dioxus 为不同的平台都提供了很好的开发文档。

如果你会使用 React ，那 Dioxus 对你来说会很简单。 

### 项目特点:
- 对桌面应用的原生支持。
- 强大的状态管理工具。
- 支持所有 HTML 标签，监听器和事件。
- 超高的内存使用率，稳定的组件分配器。
- 多通道异步调动器，超强的异步支持。
- 更多信息请查阅： [版本发布文档](https://dioxuslabs.com/blog/introducing-dioxus/).

### 示例

本项目中的所有例子都是 `桌面应用` 程序，请使用 `cargo run --example XYZ` 运行这些例子。

```
cargo run --example EXAMPLE
```

## 进入学习

<table style="width:100%" align="center">
    <tr >
        <th><a href="https://dioxuslabs.com/guide/">教程</a></th>
        <th><a href="https://dioxuslabs.com/reference/web">网页端</a></th>
        <th><a href="https://dioxuslabs.com/reference/desktop/">桌面端</a></th>
        <th><a href="https://dioxuslabs.com/reference/ssr/">SSR</a></th>
        <th><a href="https://dioxuslabs.com/reference/mobile/">移动端</a></th>
        <th><a href="https://dioxuslabs.com/guide/concepts/managing_state.html">状态管理</a></th>
    <tr>
</table>


## Dioxus 项目

| 文件浏览器 (桌面应用)                                                                                                                                                        | WiFi 扫描器 (桌面应用)                                                                                                                                                                 | Todo管理 (所有平台)                                                                                                                                                 | 商城系统 (SSR/liveview)                                                                                                                                                 |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [![File Explorer](https://github.com/DioxusLabs/example-projects/raw/master/file-explorer/image.png)](https://github.com/DioxusLabs/example-projects/blob/master/file-explorer) | [![Wifi Scanner Demo](https://github.com/DioxusLabs/example-projects/raw/master/wifi-scanner/demo_small.png)](https://github.com/DioxusLabs/example-projects/blob/master/wifi-scanner) | [![TodoMVC example](https://github.com/DioxusLabs/example-projects/raw/master/todomvc/example.png)](https://github.com/DioxusLabs/example-projects/blob/master/todomvc) | [![E-commerce Example](https://github.com/DioxusLabs/example-projects/raw/master/ecommerce-site/demo.png)](https://github.com/DioxusLabs/example-projects/blob/master/ecommerce-site) |


查看 [awesome-dioxus](https://github.com/DioxusLabs/awesome-dioxus) 查看更多有趣（~~NiuBi~~）的项目！

## 为什么使用 Dioxus 和 Rust ？

TypeScript 是一个不错的 JavaScript 拓展集，但它仍然算是 JavaScript。

TS 代码运行效率不高，而且有大量的配置项。

相比之下，Dioxus 使用 Rust 编写将大大的提高效能。

使用 Rust 开发，我们能获得：

- 静态类型支持。
- 变量默认不变性。
- 简单直观的模块系统。
- 内部集成的文档系统。
- 先进的模式匹配系统。
- 简洁、高效、强大的迭代器。
- 内置的 单元测试 / 集成测试。
- 优秀的异常处理系统。
- 强大且健全的标准库。
- 灵活的 `宏` 系统。
- 使用 `crates.io` 管理包。

Dioxus 能为开发者提供的：

- 安全使用数据结构。
- 安全的错误处理结果。
- 拥有原生移动端的性能。
- 直接访问系统的IO层。

Dioxus 使 Rust 应用程序的编写速度和 React 应用程序一样快，但提供了更多的健壮性，让团队能在更短的时间内做出强大功能。

### 不建议使用 Dioxus 的情况？

您不该在这些情况下使用 Dioxus ：

- 您不喜欢 React Hooks 的前端开发方案。
- 您需要一个 `no-std` 的渲染器。
- 您希望应用运行在 `不支持 Wasm 或 asm.js` 的浏览器。
- 您需要一个 `Send + Sync` UI 解决方案（目前不支持）。


## 协议

这个项目使用 [MIT 协议].

[MIT 协议]: https://github.com/dioxuslabs/dioxus/blob/master/LICENSE
