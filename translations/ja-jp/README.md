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
    <a href="https://dioxuslabs.com"> Web サイト </a>
    <span> | </span>
    <a href="https://github.com/DioxusLabs/example-projects"> 例 </a>
    <span> | </span>
    <a href="https://dioxuslabs.com/learn/0.4/"> ガイド </a>
    <span> | </span>
    <a href="https://github.com/DioxusLabs/dioxus/blob/master/notes/README/ZH_CN.md"> 中文 </a>
    <span> | </span>
    <a href="https://github.com/DioxusLabs/dioxus/blob/master/translations/pt-br/README.md"> PT-BR </a>
  </h3>
</div>

<br/>

Dioxus は、Rust でクロスプラットフォームのユーザー・インターフェースを構築するための、移植性が高く、パフォーマンスと人間工学に基づいたフレームワークです。

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

Dioxus は、ウェブアプリ、デスクトップアプリ、静的サイト、モバイルアプリ、TUI アプリ、ライブビューアプリなどの配信に使用できます。Dioxus は完全にレンダラーに依存せず、あらゆるレンダラーのプラットフォームとして使用可能です。

React を知っているのであれば、Dioxus はすでにご存知でしょう。

## ユニークな特徴:

- 10 行以下のコードでネイティブに動作するデスクトップアプリ（Electron は不要！）。
- 人間工学に基づいたパワフルな状態管理。
- 包括的なインラインドキュメント - すべての HTML 要素、リスナー、イベントのホバーとガイド。
- 驚異的な速さ 🔥 🔥 と極めて高いメモリ効率
- 高速反復のための統合されたホットリロード
- コルーチンとサスペンスによるファーストクラスの非同期サポート
- などなど！[リリース記事全文](https://dioxuslabs.com/blog/introducing-dioxus/)を読む。

## 対応プラットフォーム

<div align="center">
  <table style="width:100%">
    <tr>
      <td><em>Web</em></td>
      <td>
        <ul>
          <li>WebAssembly を使って DOM に直接レンダリングする</li>
          <li>SSR でプレレンダリングし、クライアントの上で再ハイドレーション</li>
          <li>シンプルな "hello world "は約 65kb で、React と同等</li>
          <li>開発サーバーを内蔵し、ホットリロードで迅速なイテレーションを実現</li>
        </ul>
      </td>
    </tr>
    <tr>
      <td><em>デスクトップ</em></td>
      <td>
        <ul>
          <li>ウェブビューを使ったレンダリング、あるいは実験的に WGPU や Skia を使ったレンダリング</li>
          <li>設定不要。cargo-run を実行するだけで、アプリをビルド可能</li>
          <li>電子的な IPC を必要としないネイティブシステムアクセスの完全サポート</li>
          <li>macOS、Linux、Windows に対応。ポータブルで 3 mb より小さなバイナリ</li>
        </ul>
      </td>
    </tr>
    <tr>
      <td><em>モバイル</em></td>
      <td>
        <ul>
          <li>ウェブビューを使ったレンダリング、あるいは実験的に WGPU や Skia を使ったレンダリング</li>
          <li>iOS と Android をサポート</li>
          <li>React ネイティブよりも<em>大幅に</em>パフォーマンスが高い</li>
        </ul>
      </td>
    </tr>
    <tr>
      <td><em>ライブビュー</em></td>
      <td>
        <ul>
          <li>アプリケーション、または単一のコンポーネントを完全にサーバー上でレンダリング</li>
          <li>Axum や Warp のような人気のある Rust フレームワークとの統合</li>
          <li>極めて低いレイテンシーと 10,000 以上の同時アプリをサポートする能力</li>
        </ul>
      </td>
    </tr>
    <tr>
      <td><em>ターミナル</em></td>
      <td>
        <ul>
          <li><a href="https://github.com/vadimdemedes/ink">ink.js</a> のように、アプリケーションをターミナルに直接レンダリング</li>
          <li>ブラウザでおなじみのフレックスボックスと CSS モデルを採用</li>
          <li>テキスト入力、ボタン、フォーカスシステムなどの組み込みウィジェット</li>
        </ul>
      </td>
    </tr>
  </table>
</div>

## なぜ Dioxus？

アプリを構築するための選択肢は山ほどあるのに、なぜ Dioxus を選ぶのでしょうか？

まず第一に、Dioxus は開発者の体験を優先しています。これは Dioxus 独自のさまざまな機能に反映されています:

- メタ言語（RSX）とそれに付随する VSCode 拡張機能のオートフォーマット機能
- デスクトップとウェブの両方で RSX のインタプリタを使用したホットリロード
- 優れたドキュメントの重視 - 私たちのガイドは完全で、私たちの HTML 要素は文書化されています。
- 簡素化のための多大な研究

Dioxus は非常に拡張性の高いプラットフォームでもあります。

- 非常にシンプルに最適化されたスタックマシンを実装することで、新しいレンダラーを簡単に構築できます。
- コンポーネントやカスタム・エレメントもビルドして共有できます

Dioxus は素晴らしいが、なぜ私には使えないのか？

- まだ完全に成熟していません。API はまだ移行中で、いろいろなものが壊れるかもしれません（私たちはそれを避けようとしていますが）。
- no-std 環境で動作させる必要がある。
- UI を構築する React-hooks モデルが好きではない場合

## コントリビュート

- 私たちの [issue tracker](https://github.com/dioxuslabs/dioxus/issues) で問題を報告してください。
- ディスコードに参加して質問してください！

<a href="https://github.com/dioxuslabs/dioxus/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=dioxuslabs/dioxus&max=30&columns=10" />
</a>

## ライセンス

このプロジェクトのライセンスは [MIT ライセンス] です。

[MIT ライセンス]: https://github.com/DioxusLabs/dioxus/blob/master/LICENSE-MIT

あなたが明示的に別段の定めをしない限り、あなたによって Dioxus に含めるために意図的に提出されたコントリビュートは、
いかなる追加条件もなく、MIT としてライセンスされるものとします。
