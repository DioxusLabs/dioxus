# RSX à Fundo

A macro RSX facilita muito a montagem de interfaces de usuário complexas com uma sintaxe Rust muito natural:

```rust
rsx!(
    div {
        button {
            onclick: move |e| todos.write().new_todo(),
            "Add todo"
        }
        ul {
            class: "todo-list",
            todos.iter().map(|(key, todo)| rsx!(
                li {
                    class: "beautiful-todo"
                    key: "f"
                    h3 { "{todo.title}" }
                    p { "{todo.contents}"}
                }
            ))
        }
    }
)
```

Nesta seção, abordaremos a macro `rsx!` em profundidade. Se você preferir aprender através de exemplos, o guia `referência de código` tem muitos exemplos sobre como usar `rsx!` efetivamente.

### Estrutura do elemento

Os atributos devem vir antes dos elementos filhos

```rust
div {
    hidden: "false",
    "some text"
    child {}
    Component {} // uppercase
    component() // lowercase is treated like a function call
    (0..10).map(|f| rsx!{ "hi {f}" }) // arbitrary rust expressions
}
```

Cada elemento usa uma lista de expressões separadas por vírgulas para construir o nó. A grosso modo, veja como eles funcionam:

- `name: value` define uma propriedade neste elemento.
- `text` adiciona um novo elemento de texto
- `tag {}` adiciona um novo elemento filho
- `CustomTag {}` adiciona um novo componente filho
- `{expr}` cola os tokens `expr` literalmente. Eles devem ser `IntoIterator<T> where T: IntoVnode` para funcionar corretamente

As vírgulas são totalmente opcionais, mas podem ser úteis para delinear entre elementos e atributos.

A função `render` fornece um alocador **extremamente eficiente** para `VNodes` e `text`, então tente não usar a macro `format!` em seus componentes. Os métodos `ToString` padrão do Rust passam pelo alocador global, mas todo o texto nos componentes é alocado dentro de uma ""arena Bump"" gerenciada manualmente. Para levá-lo na direção certa, todos os atributos baseados em texto recebem `std::fmt::Arguments` diretamente, então você vai querer usar `format_args!` quando a interpolação interna `f-string` simplesmente não funcionar.

### Ignorando `cx.render` com `render!(...)`

Às vezes, escrever `cx.render` é um aborrecimento. O `rsx!` macro aceitará qualquer token seguido por uma vírgula como destino para chamar "render" em:

```rust
cx.render(rsx!( div {} ))
// becomes
render!(div {})
```

### Renderização Condicional

Às vezes, você pode não querer renderizar um elemento dada uma condição. O `rsx!` macro aceitará quaisquer tokens contidos diretamente com chaves, desde que resolvam para um tipo que implemente `IntoIterator<VNode>`. Isso nos permite escrever qualquer expressão Rust que resolva para um `VNode`:

```rust
rsx!({
    if enabled {
        render!(div {"enabled"})
    } else {
        render!(li {"disabled"})
    }
})
```

Uma maneira conveniente de ocultar/mostrar um elemento é retornar um `Option<VNode>`. Quando combinado com `and_then`, podemos controlar sucintamente o estado de exibição dado alguns booleanos:

```rust
rsx!({
    a.and_then(rsx!(div {"enabled"}))
})
```

É importante notar que a expressão `rsx!()` é tipicamente tardia - esta expressão deve ser _renderizada_ para produzir um `VNode`. Ao usar declarações de `match`, devemos renderizar todos os braços para evitar a regra 'não há dois fechamentos idênticos' que o Rust impõe:

```rust
// this will not compile!
match case {
    true => rsx!(div {}),
    false => rsx!(div {})
}

// the nodes must be rendered first
match case {
    true => render!(div {}),
    false => render!(div {})
}
```

### Listas

Novamente, porque qualquer coisa que implemente `IntoIterator<VNode>` é válida, podemos usar listas diretamente em nosso `rsx!`:

```rust
let items = vec!["a", "b", "c"];

cx.render(rsx!{
    ul {
        {items.iter().map(|f| rsx!(li { "a" }))}
    }
})
```

Às vezes, faz sentido renderizar `VNodes` em uma lista:

```rust
let mut items = vec![];

for _ in 0..5 {
    items.push(render!(li {} ))
}

render!({items} )
```

#### Listas e chaves

Ao renderizar o `VirtualDom` na tela, o Dioxus precisa saber quais elementos foram adicionados e quais foram removidos. Essas mudanças são determinadas através de um processo chamado "diffing" - um antigo conjunto de elementos é comparado a um novo conjunto de elementos. Se um elemento for removido, ele não aparecerá nos novos elementos, e Dioxus sabe removê-lo.

No entanto, com listas, Dioxus não sabe exatamente como determinar quais elementos foram adicionados ou removidos se a ordem mudar ou se um elemento for adicionado ou removido do meio da lista.

Nesses casos, é de vital importância especificar uma "chave" ao lado do elemento. As chaves devem ser persistentes entre as renderizações.

```rust
fn render_list(cx: Scope, items: HashMap<String, Todo>) -> DomTree {
    render!(ul {
        {items.iter().map(|key, item| {
            li {
                key: key,
                h2 { "{todo.title}" }
                p { "{todo.contents}" }
            }
        })}
    })
}
```

Existem muitos guias feitos para chaves no React, então recomendamos a leitura para entender sua importância:

- [React guide on keys](https://reactjs.org/docs/lists-and-keys.html)
- [Importância das chaves (Média)](https://kentcdodds.com/blog/understanding-reacts-key-prop)

### Referência Completa

```rust
let text = "example";

cx.render(rsx!{
    div {
        h1 { "Example" },

        {title}

        // fstring interpolation
        "{text}"

        p {
            // Attributes
            tag: "type",

            // Anything that implements display can be an attribute
            abc: 123,

            enabled: true,

            // attributes also supports interpolation
            // `class` is not a restricted keyword unlike JS and ClassName
            class: "big small wide short {text}",

            class: format_args!("attributes take fmt::Arguments. {}", 99),

            tag: {"these tokens are placed directly"}

            // Children
            a { "abcder" },

            // Children with attributes
            h2 { "hello", class: "abc-123" },

            // Child components
            CustomComponent { a: 123, b: 456, key: "1" },

            // Child components with paths
            crate::components::CustomComponent { a: 123, b: 456, key: "1" },

            // Iterators
            { (0..3).map(|i| rsx!( h1 {"{:i}"} )) },

            // More rsx!, or even html!
            { rsx! { div { } } },
            { html! { <div> </div> } },

            // Matching
            // Requires rendering the nodes first.
            // rsx! is lazy, and the underlying closures cannot have the same type
            // Rendering produces the VNode type
            {match rand::gen_range::<i32>(1..3) {
                1 => render!(h1 { "big" })
                2 => render!(h2 { "medium" })
                _ => render!(h3 { "small" })
            }}

            // Optionals
            {true.and_then(|f| rsx!( h1 {"Conditional Rendering"} ))}

            // Child nodes
            {cx.props.children}

            // Any expression that is `IntoVNode`
            {expr}
        }
    }
})
```
