# VNodes com RSX, HTML, e NodeFactory

Muitas estruturas modernas fornecem uma linguagem específica de domínio para declarar interfaces de usuário.

No caso do React, esta extensão de linguagem é chamada `JSX` e deve ser tratada através de dependências adicionais e pré/pós processadores para transformar seu código fonte. Com Rust, podemos simplesmente fornecer um macro procedural na própria dependência do Dioxus que **imita** a linguagem `JSX`.

Com o Dioxus, nós enviamos duas macros diferentes - uma macro que imita `JSX` (a macro `html!`) e uma macro que imita a sintaxe de estrutura aninhada nativa do Rust (o macro `rsx!`). Esses macros simplesmente transformam suas entradas em `NodeFactory`.

Por exemplo, este macro `html!`:

```rust
html!(<div> "hello world" </div>)
```

torna-se este `NodeFactory`:

```rust
|f| f.element(
    dioxus_elements::div, // tag
    [], // listeners
    [], // attributes
    [f.static_text("hello world")], // children
    None // key
)
```

A API `NodeFactory` é bastante ergonômica, tornando-se uma opção viável para uso direto.

A API `NodeFactory` também aproveita a garantia de correções em tempo de compilação e tem um incrível suporte para destaque de sintaxe. Usamos o que Rust chama de "tipo de unidade" (Unit Type) - o `dioxus_elements::div` e métodos associados para garantir que um `div` possa ter apenas atributos associados a `div`s. Isso nos permite incluir documentação relevante, suporte de intellisense e salto para definição de métodos e atributos.

![Correção por Compilador](../images/compiletimecorrect.png)

## O Macro html!

O macro `html!` suporta um subconjunto limitado do padrão `HTML`. As ferramentas de análise de macro do Rust são um tanto limitadas, então todo texto entre as tags _deve ser citado_.

No entanto, escrever HTML manualmente é um pouco tedioso - as ferramentas de IDE para Rust não suportam linting/autocomplete/syntax highlighting. Sugerimos o uso de RSX - é mais natural para programas Rust e _se integra bem_ com ferramentas Rust na IDE.

```rust
let name = "jane";
let pending = false;
let count = 10;

dioxus::ssr::render_lazy(html! {
    <div>
        <p> "Hello, {name}!" </p>
        <p> "Status: {pending}!" </p>
        <p> "Count {count}!" </p>
    </div>
});
```

## O Macro rsx!

O macro `rsx!` é um construtor `VNode` projetado especialmente para programas Rust. Escrevê-los deve tão natural como montar uma estrutura (`struct`).

O VSCode também oferece suporte a isso com folding, bracket-tabbing, bracket highlighting, seleção de seção, documentação em linha, definição GOTO e suporte à refatoração.

A extensão Dioxus para VSCode fornece uma maneira de converter uma seleção de HTML diretamente para RSX, para que você possa importar modelos da Web diretamente para seu aplicativo existente.

Também é um pouco mais agradável aos olhos do que HTML.

```rust
dioxus::ssr::render_lazy(rsx! {
    div {
        p {"Hello, {name}!"}
        p {"Status: {pending}!"}
        p {"Count {count}!"}
    }
});
```

Na próxima seção, abordaremos a macro `rsx!` com mais profundidade.
