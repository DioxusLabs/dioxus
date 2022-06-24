# Manipuladores de eventos

Para tornar nossas interfaces menos estáticas e mais interessantes, devemos adicionar a capacidade de interagir com uma ação do usuário. Para fazer isso, precisamos adicionar alguns manipuladores de eventos.

## Os eventos mais básicos: cliques

Se você já deu uma olhada nos exemplos do Dioxus, você definitivamente notou o suporte para botões e cliques. Para adicionar alguma ação básica quando um botão é clicado, precisamos definir um botão e anexar um manipulador "onclick" a ele.

```rust
fn app(cx: Scope) -> Element {
    cx.render(rsx!{
        button {
            onclick: move |evt| println!("I've been clicked!"),
            "click me!"
        }
    })
}
```

Se você estiver usando o padrão construtor do Rust, também é bem simples. `onclick` é um método para qualquer construtor com elementos HTML.

```rust
fn app(cx: Scope) -> Element {
    button(&cx)
        .onclick(move |evt| println!("I've been clicked!"))
        .text("click me!")
        .build()
}
```

O manipulador de eventos é diferente no Dioxus do que em outras bibliotecas. Os manipuladores de eventos no Dioxus podem emprestar quaisquer dados que tenham um tempo de vida que corresponda ao escopo do componente. Isso significa que podemos salvar um valor com `use_hook` e depois usá-lo em nosso manipulador.

```rust
fn app(cx: Scope) -> Element {
    let val = cx.use_hook(|_| 10);

    button(&cx)
        .onclick(move |evt| println!("Current number {val}"))
        .text("click me!")
        .build()
}
```

## O objeto `Event`

Quando o ouvinte é acionado, o Dioxus passará qualquer dado relacionado através do parâmetro `event`. Isso mantém o estado útil relevante para o evento. Por exemplo, nos formulários, o Dioxus preencherá o campo "valores".

```rust
// the FormEvent is roughly
struct FormEvent {
    value: String,
    values: HashMap<String, String>
}

fn app(cx: Scope) -> Element {
    cx.render(rsx!{
        form {
            onsubmit: move |evt| {
                println!("Values of form are {evt.values:#?}");
            }
            input { id: "password", name: "password" }
            input { id: "username", name: "username" }
        }
    })
}
```

## Parando a propagação

Com uma interface de usuário complexa o suficiente, você pode perceber que os ouvintes podem realmente ser aninhados.

```rust
div {
    onclick: move |evt| {},
    "outer",
    div {
        onclick: move |evt| {},
        "inner"
    }
}
```

Nesse layout específico, um clique no `div` interno é também um clique no `div` externo. Se não quisermos que o `div` externo seja acionado toda vez que acionarmos o `div` interno, então devemos chamar `cancel_bubble()`.

Isso impedirá que quaisquer ouvintes acima do ouvinte atual sejam acionados.

```rust
div {
    onclick: move |evt| {},
    "outer",
    div {
        onclick: move |evt| {
            // now, outer won't be triggered
            evt.cancel_bubble();
        },
        "inner"
    }
}
```

## Prevenindo o comportamento padrão

Com renderizadores baseados em HTML, o navegador executará automaticamente alguma ação. Para entradas de texto, isso seria inserir a chave fornecida. Para formulários, isso pode envolver a navegação da página.

Em alguns casos, você não deseja esse comportamento padrão. Nesses casos, em vez de manipular o evento diretamente, você deseja evitar qualquer manipulador padrão.

Normalmente, em React ou JavaScript, você chamaria "preventDefault" no evento no retorno de chamada. Dioxus atualmente _não_ suporta este comportamento. Em vez disso, você precisa adicionar um atributo ao elemento que gera o evento.

```rust
form {
    prevent_default: "onclick",
    onclick: move |_|{
        // handle the event without navigating the page.
    }
}
```
