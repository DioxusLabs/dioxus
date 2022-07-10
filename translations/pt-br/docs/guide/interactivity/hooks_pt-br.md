# Hooks e Estado Interno

Na seção [Adicionando interatividade](./interactivity.md), abordamos brevemente o conceito de hooks e estado internamente armazenado aos componentes.

Nesta seção, vamos mergulhar um pouco mais fundo nos hooks, explorando tanto a teoria quanto a mecânica.

## Teoria dos Hooks

Ao longo das últimas décadas, cientistas e engenheiros da computação há muito buscam a "maneira certa" de projetar interfaces de usuário. Com cada nova linguagem de programação, novos recursos são desbloqueados que mudam o paradigma no qual as interfaces do usuário são codificadas.

De um modo geral, vários padrões surgiram, cada um com seus próprios pontos fortes e defeitos.

A grosso modo, existem dois tipos de estruturas GUI:

- GUIs imediatas: renderize novamente a tela inteira em cada atualização
- GUIs retidas: apenas renderize novamente a parte da tela que foi alterada

Normalmente, as GUIs de modo imediato são mais simples de escrever, mas podem ficar mais lentas à medida que mais recursos, como estilo, são adicionados.

Muitas GUIs hoje são escritas no _modo retido_ - seu código altera os dados da interface do usuário, mas o renderizador é responsável por realmente desenhar na tela. Nesses casos, o estado da nossa GUI permanece enquanto a interface do usuário é renderizada. Para ajudar a acomodar GUIs de modo retido, como o navegador da Web, o Dioxus fornece um mecanismo para manter o estado ao redor.

> Nota: Mesmo que os hooks sejam acessíveis, você deve preferir o fluxo de dados unidirecional e encapsulamento. Seu código de interface do usuário deve ser o mais previsível possível. Dioxus é muito rápido, mesmo para os maiores aplicativos.

## Mecânica dos Hooks

Para manter o estado entre as renderizações, o Dioxus fornece um `hook` através da API `use_hook`. Isso nos dá uma referência mutável aos dados retornados da função de inicialização.

```rust
fn example(cx: Scope) -> Element {
    let name: &mut String = cx.use_hook(|| "John Doe".to_string());

    //
}
```

Podemos até modificar esse valor diretamente de um manipulador de eventos:

```rust
fn example(cx: Scope) -> Element {
    let name: &mut String = cx.use_hook(|| "John Doe".to_string());

    cx.render(rsx!(
        button {
            onclick: move |_| name.push_str(".."),
        }
    ))
}
```

Mecanicamente, cada chamada para um `use_hook` nos fornece `&mut T` para um novo valor.

```rust
fn example(cx: Scope) -> Element {
    let name: &mut String = cx.use_hook(|| "John Doe".to_string());
    let age: &mut u32 = cx.use_hook(|| 10);
    let friends: &mut Vec<String> = cx.use_hook(|| vec!["Jane Doe".to_string()]);

    //
}
```

Internamente, o Dioxus está criando uma lista de valores para os Hooks com cada chamada para `use_hook` avançando o índice da lista para retornar o próximo valor.

Nossa HookList interna seria algo como:

```rust
[
    Hook<String>,
    Hook<u32>,
    Hook<String>,
]
```

É por isso que hooks chamados fora de ordem irão falhar - se tentarmos fazer `downcast` de um `Hook<String>` para `Hook<u32>`, o Dioxus não tem escolha a não ser entrar em pânico. Nós fornecemos um `try_use_hook`, mas você nunca deve precisar disso na prática.

Esse padrão pode parecer estranho no começo, mas pode ser uma atualização significativa sobre estruturas como blobs de estado, que tendem a ser difíceis de usar em [Rust, dado o sistema de propriedade](https://rust-lang.github.io/rfcs/2229-capture-disjoint-fields.html).

## Regras dos Hooks

Os hooks são sensíveis à forma como são usados. Para usar hooks, você deve respeitar as
"Regras dos Hooks" ([emprestado do react](https://reactjs.org/docs/hooks-rules.html)):

- Funções com "use\_" não devem ser chamadas em `callbacks`
- Funções com "use\_" não devem ser chamadas fora de ordem
- Funções com "use\_" não devem ser chamadas em `loops` ou condicionais

Exemplos de mal uso incluem:

### ❌ Hooks aninhados

```rust
// ❌ don't call use_hook or any `use_` function *inside* use_hook!
cx.use_hook(|_| {
    let name = cx.use_hook(|_| "ads");
})

// ✅ instead, move the first hook above
let name = cx.use_hook(|_| "ads");
cx.use_hook(|_| {
    // do something with name here
})
```

### ❌ Uso em condicionais

```rust
// ❌ don't call use_ in conditionals!
if do_thing {
    let name = use_state(&cx, || 0);
}

// ✅ instead, *always* call use_state but leave your logic
let name = use_state(&cx, || 0);
if do_thing {
    // do thing with name here
}
```

### ❌ Uso em loops

```rust
// ❌ Do not use hooks in loops!
let mut nodes = vec![];

for name in names {
    let age = use_state(&cx, |_| 0);
    nodes.push(cx.render(rsx!{
        div { "{age}" }
    }))
}

// ✅ Instead, consider refactoring your usecase into components
#[inline_props]
fn Child(cx: Scope, name: String) -> Element {
    let age = use_state(&cx, |_| 0);
    cx.render(rsx!{ div { "{age}" } })
}

// ✅ Or, use a hashmap with use_ref
let ages = use_ref(&cx, || HashMap::new());

names.iter().map(|name| {
    let age = ages.get(name).unwrap();
    cx.render(rsx!{ div { "{age}" } })
})
```

## Criando novos Hooks

No entanto, a maioria dos hooks com os quais você irá interagir _não_ retorna um `&mut T` já que isso não é muito útil em uma situação do mundo real.

Considere quando tentamos passar nosso `&mut String` em dois manipuladores diferentes:

```rust
fn example(cx: Scope) -> Element {
    let name: &mut String = cx.use_hook(|| "John Doe".to_string());

    cx.render(rsx!(
        button { onclick: move |_| name.push_str("yes"), }
        button { onclick: move |_| name.push_str("no"), }
    ))
}
```

Rust não permitirá que isso seja compilado! Não podemos 'Copiar' referências mutáveis únicas - elas são, por definição, únicas. No entanto, _podemos_ pegar emprestado novamente nosso `&mut T` como um `&T` que são referências não exclusivas e as compartilhamos entre os manipuladores:

```rust
fn example(cx: Scope) -> Element {
    let name: &String = &*cx.use_hook(|| "John Doe".to_string());

    cx.render(rsx!(
        button { onclick: move |_| log::info!("{}", name), }
        button { onclick: move |_| log::info!("{}", name), }
    ))
}
```

Portanto, para qualquer hook personalizado que queiramos projetar, precisamos habilitar a mutação por meio de [mutabilidade interior](https://doc.rust-lang.org/book/ch15-05-interior-mutability.html) - por exemplo: verificação em tempo de execução. [verificação de empréstimo](https://doc.rust-lang.org/1.8.0/book/references-and-borrowing.html). Pode ocorrer um pequeno custo de tempo de execução para cada vez que extraímos um novo valor do hook, mas esse custo é extremamente mínimo.

Este exemplo usa o tipo `Cell` para nos permitir substituir o valor por meio de mutabilidade interna. O `Cell` tem praticamente zero sobrecarga, mas é um pouco mais limitado que seu primo `RefCell`.

```rust
fn example(cx: Scope) -> Element {
    let name: &Cell<&'static str> = cx.use_hook(|| Cell::new("John Doe"));

    cx.render(rsx!(
        button { onclick: move |_| name.set("John"), }
        button { onclick: move |_| name.set("Jane"), }
    ))
}
```

## Atualizações de estado por meio de Hooks

Hooks como `use_state` e `use_ref` envolvem esta verificação de empréstimo em tempo de execução em um tipo que _não_ implementa `Copy`. Além disso, eles também marcam o componente como "sujo" sempre que um novo valor é definido. Desta forma, sempre que `use_state` tem um novo valor `set`, o componente sabe como atualizar.

```rust
fn example(cx: Scope) -> Element {
    let name = use_state(&cx, || "Jack");

    cx.render(rsx!(
        "Hello, {name}"
        button { onclick: move |_| name.set("John"), }
        button { onclick: move |_| name.set("Jane"), }
    ))
}
```

Internamente, nossa função `set` se parece com isso:

```rust
impl<'a, T> UseState<'a, T> {
    fn set(&self, new: T) {
        // Replace the value in the cell
        self.value.set(new);

        // Mark our component as dirty
        self.cx.needs_update();
    }
}
```

A maioria dos hooks que fornecemos implementam `Deref` em seus valores, pois são essencialmente ponteiras inteligentes. Para acessar o valor subjacente, muitas vezes você precisará usar o operador `deref`:

```rust
fn example(cx: Scope) -> Element {
    let name = use_state(&cx, || "Jack");

    match *name {
        "Jack" => {}
        "Jill" => {}
        _ => {}
    }

    // ..
}

```

## Hooks fornecidos pelo pacote `Dioxus-Hooks`

Por padrão, agrupamos um punhado de hooks no pacote Dioxus-Hooks. Sinta-se à vontade para clicar em cada hook para ver sua definição e documentação associada.

- [use_state](https://docs.rs/dioxus-hooks/latest/dioxus_hooks/fn.use_state.html) - armazena o estado com atualizações ergonômicas
- [use_ref](https://docs.rs/dioxus-hooks/latest/dioxus_hooks/fn.use_ref.html) - armazena o estado não clonado com uma refcell
- [use_future](https://docs.rs/dioxus-hooks/latest/dioxus_hooks/fn.use_future.html) - armazena um futuro a ser pesquisado após a inicialização
- [use_coroutine](https://docs.rs/dioxus-hooks/latest/dioxus_hooks/fn.use_coroutine.html) - armazene um futuro que pode ser interrompido/iniciado/comunicado com
- [use_context_provider](https://docs.rs/dioxus-hooks/latest/dioxus_hooks/fn.use_context_provider.html) - expor o estado aos componentes descendentes
- [use_context](https://docs.rs/dioxus-hooks/latest/dioxus_hooks/fn.use_context.html) - estado de consumo fornecido por `use_provide_context`
- [use_suspense](https://docs.rs/dioxus-hooks/latest/dioxus_hooks/fn.use_suspense.html)
