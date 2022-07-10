# Estado local

O primeiro passo para lidar com a complexidade em seu aplicativo é refatorar seu estado para que seja puramente local. Isso incentiva uma boa reutilização de código e evita o vazamento de abstrações entre os limites dos componentes.

## Como isso se parece?

Digamos que estamos gerenciando o estado para uma lista de `Todo`. Sempre que editamos o `Todo`, queremos que a lista seja atualizada. Podemos ter começado a construir nosso aplicativo, mas colocando tudo em um único `use_ref`.

```rust
struct Todo {
    contents: String,
    is_hovered: bool,
    is_editing: bool,
}

let todos = use_ref(&cx, || vec![Todo::new()]);

cx.render(rsx!{
    ul {
        todos.read().iter().enumerate().map(|(id, todo)| rsx!{
            li {
                onhover: move |_| *todos.write()[id].is_hovered = true,
                h1 { "{todo.contents}" }
            }
        })
    }
})
```

Como mostrado acima, sempre que passar o mouse sobre o `Todo`, queremos definir seu estado:

```rust
todos.write()[0].is_hovered = true;
```

À medida que a quantidade de interações aumenta, aumenta também a complexidade do nosso estado. O estado "hover" deve realmente ser incorporado ao nosso modelo de dados?

Em vez disso, vamos refatorar nosso componente `Todo` para lidar com seu próprio estado:

```rust
#[inline_props]
fn Todo<'a>(cx: Scope, todo: &'a Todo) -> Element {
    let is_hovered = use_state(&cx, || false);

    cx.render(rsx!{
        li {
            h1 { "{todo.contents}" }
            onhover: move |_| is_hovered.set(true),
        }
    })
}
```

Agora, podemos simplificar nosso modelo de dados `Todo` para se livrar do estado da interface do usuário local:

```rust
struct Todo {
    contents: String,
}
```

Isso não é apenas melhor para encapsulamento e abstração, mas é mais eficiente! Sempre que o mouse passar por um `Todo`, não precisaremos renderizar novamente _todo_ o `Todo` - apenas o `Todo` que está sendo realçado no momento.

Sempre que possível, você deve tentar refatorar a camada de "visualização" _para fora_ do seu modelo de dados.

## Imutabilidade

Armazenar todo o seu estado dentro de um `use_ref` pode ser tentador. No entanto, você rapidamente se deparará com um problema em Rust que o `Ref` fornecido por `read()` "não dura o suficiente". De fato - você não pode retornar um valor emprestado localmente para a árvore Dioxus.

Nesses cenários, considere dividir seu `use_ref` em `use_state`s individuais.

Você pode ter começado a modelar seu componente com um `struct` e `use_ref`

```rust
struct State {
    count: i32,
    color: &'static str,
    names: HashMap<String, String>
}

// in the component
let state = use_ref(&cx, State::new)
```

A abordagem "melhor" para esse componente específico seria separar o estado em diferentes valores:

```rust
let count = use_state(&cx, || 0);
let color = use_state(&cx, || "red");
let names = use_state(&cx, HashMap::new);
```

Você pode reconhecer que nosso valor de `names` é um `HashMap` - o que não é muito performático para clonar toda vez que atualizamos seu valor. Para resolver este problema, nós sugerimos _fortemente_ usar uma biblioteca como [`im`](https://crates.io/crates/im) que aproveitará o compartilhamento estrutural para tornar clones e mutações muito mais rápido.

Quando combinado com o método `make_mut` em `use_state`, você pode obter atualizações realmente sucintas nas coleções:

```rust
let todos = use_state(&cx, im_rc::HashMap::default);

todos.make_mut().insert("new todo", Todo {
    contents: "go get groceries",
});
```

## Seguind em Frente

Esses padrões locais específicos são poderosos, mas não são uma panacéia para os problemas de gestão do estado. Às vezes, seus problemas de estado são muito maiores do que apenas permanecer local para um componente.
