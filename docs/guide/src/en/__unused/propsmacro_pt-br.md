# Propriedades do Componente

Componentes no Dioxus são funções que aceitam `Props` como entrada e saída de um `Element`. Na verdade, a função `App` que você viu no capítulo anterior era um componente sem `Props`!

A maioria dos componentes, no entanto, precisará de alguns `Props` para renderizar algo útil – então, nesta seção, aprenderemos sobre `props`:

- Derivando a `trait` `Props`
- Memoização através do `PartialEq`
- Campos opcionais em `props`
- O macro `inline_props`

## Props

A entrada do seu `Component` deve ser passada em um único `structu`, que deve implementar o `trait` `Props`. Podemos derivar essa `trait` automaticamente com `#[derive(Props)]`.

> Dioxus `Props` é muito semelhante ao [@idanarye](https://github.com/idanarye) [TypedBuilder crate](https://github.com/idanarye/rust-typed-builder) e suporta muitos dos mesmos parâmetros.

Existem 2 tipos de `Props`: próprios e emprestados.

- Todos os `props` próprios devem implementar `PartialEq`
- Valores de `props` emprestados [emprestados](https://doc.rust-lang.org/beta/rust-by-example/scope/borrow.html) do componente pai

### Props Próprios

`Props` próprios são muito simples – eles não emprestam nada. Exemplo:

```rust
// Remember: owned props must implement PartialEq!
#[derive(PartialEq, Props)]
struct VoteButtonProps {
    score: i32
}

fn VoteButton(cx: Scope<VoteButtonProps>) -> Element {
    cx.render(rsx!{
        div {
            div { "+" }
            div { "{cx.props.score}"}
            div { "-" }
        }
    })
}
```

Agora, podemos usar o componente `VoteButton` como usaríamos um elemento HTML normal:

```rust
fn main() {
    dioxus::desktop::launch(App);
}

fn App(cx: Scope) -> Element {
    cx.render(rsx! (
        VoteButton { score: 42 }
    ))
}
```

E podemos ver que o Component realmente é renderizado:

![Captura de tela do aplicativo em execução. Texto: "+ \ 42 \ -"](component_example_votes.png)

> Os `props` próprios mais simples que você pode ter é `()` - ou nenhum valor. Isto é o que o Componente `App` toma como props. `Scope` aceita um genérico para os Props cujo padrão é `()`.
>
> ```rust
> // this scope
> Scope<()>
>
> // is the same as this scope
> Scope
> ```

### Props Emprestados

Possuir `props` funciona bem se seus `props` forem fáceis de copiar - como um único número. Mas e se precisarmos passar um tipo de dados maior, como uma `String` de um componente `App` para um subcomponente `TitleCard`?

Uma solução ingênua pode ser [`.clone()`](https://doc.rust-lang.org/std/clone/trait.Clone.html) a `String`, criando uma cópia dela para o subcomponente – mas isso seria ineficiente, especialmente para `Strings` maiores.

Rust permite algo mais eficiente – emprestar a `String` como um `&str`. Em vez de criar uma cópia, isso nos dará uma referência à `String` original – é para isso que servem os Props Emprestados!

No entanto, se criarmos uma referência a uma `String`, o Rust exigirá que mostremos que a `String` não desaparecerá enquanto estivermos usando a referência. Caso contrário, se fizermos referência a algo que não existe, coisas ruins podem acontecer. Para evitar isso, o Rust nos pede para definir um tempo de vida para a referência:

```rust
#[derive(Props)]
struct TitleCardProps<'a> {
    title: &'a str,
}

fn TitleCard<'a>(cx: Scope<'a, TitleCardProps<'a>>) -> Element {
    cx.render(rsx!{
        h1 { "{cx.props.title}" }
    })
}
```

Este tempo de vida, ou `lifetime, ` `'a` diz ao compilador que enquanto o `title` existir, a `String` a partir da qual ele foi criado também deve existir. Dioxus aceitará contente tal componente - agora podemos renderizá-lo ao lado de nosso `VoteButton`!

```rust
fn App(cx: Scope) -> Element {
    // For the sake of an example, we create the &str here.
    // But you might as well borrow it from an owned String type.
    let hello = "Hello Dioxus!";

    cx.render(rsx! (
        VoteButton { score: 42 },
        TitleCard { title: hello }
    ))
}
```

![Nova captura de tela do aplicativo em execução, agora incluindo um "Hello Dioxus!"](component_example_title.png)

## Memoização

Dioxus usa Memoization para uma interface de usuário mais eficiente. Memoization é o processo no qual verificamos se um componente realmente precisa ser renderizado novamente quando seus `props` mudam. Se as propriedades de um componente mudarem, mas não afetarem a saída, não precisamos renderizar novamente o componente, economizando tempo!

Por exemplo, digamos que temos um componente que tem dois filhos:

```rust
fn Demo(cx: Scope) -> Element {
    // don't worry about these 2, we'll cover them later
    let name = use_state(&cx, || String::from("bob"));
    let age = use_state(&cx, || 21);

    cx.render(rsx!{
        Name { name: name }
        Age { age: age }
    })
}
```

Se `name` muda mas `age` não, então não há razão para renderizar novamente nosso componente `Age` já que o conteúdo de suas props não mudou significativamente.

Dioxus memoiza componentes próprios. Ele usa `PartialEq` para determinar se um componente precisa ser renderizado novamente ou se ele permaneceu o mesmo. É por isso que você deve derivar `PartialEq`!

> Isso significa que você sempre pode contar com `props` com `PartialEq` ou sem `props` para atuar como barreiras em seu aplicativo. Isso pode ser extremamente útil ao criar aplicativos maiores em que as propriedades mudam com frequência. Ao mover nosso estado para uma solução global de gerenciamento de estado, podemos obter re-renderizações cirúrgicas precisas, melhorando o desempenho de nosso aplicativo.

Props emprestados não podem ser memoizados com segurança. No entanto, isso não é um problema - Dioxus depende da memoização de seus pais para determinar se eles precisam ser renderizados novamente.

## Props Opcionais

Você pode criar facilmente campos opcionais usando o tipo `Option<…>` para um campo:

```rust
#[derive(Props, PartialEq)]
struct MyProps {
    name: String,

    description: Option<String>
}

fn Demo(cx: MyProps) -> Element {
    let text = match cx.props.description {
        Some(d) => d,             // if a value is provided
        None => "No description"  // if the prop is omitted
    };

    cx.render(rsx! {
        "{name}": "{text}"
    })
}
```

Neste exemplo, `name` é um prop obrigatório e `description` é opcional.
Isso significa que podemos omitir completamente o campo de descrição ao chamar o componente:

```rust
rsx!{
    Demo {
        name: "Thing".to_string(),
        // description is omitted
    }
}
```

Além disso, se fornecermos um valor, não precisamos envolvê-lo com `Some(…)`. Isso é feito automaticamente para nós:

```rust
rsx!{
    Demo {
        name: "Thing".to_string(),
        description: "This is explains it".to_string(),
    }
}
```

Se você quiser tornar um `prop` obrigatório mesmo que seja do tipo `Option`, você pode fornecer o modificador `!optional`:

```rust
#[derive(Props, PartialEq)]
struct MyProps {
    name: String,

    #[props(!optional)]
    description: Option<String>
}
```

Isso pode ser especialmente útil se você tiver um `alias` de tipo chamado `Option` no escopo atual.

Para obter mais informações sobre como as tags funcionam, confira [TypedBuilder](https://github.com/idanarye/rust-typed-builder).

No entanto, todos os atributos para `props` no Dioxus são achatados (sem necessidade de sintaxe `setter`) e o campo `opcional` é novo. O modificador `opcional` é uma combinação de dois modificadores separados: `default` e `strip_option` e é detectado automaticamente nos tipos `Option<…>`.

A lista completa dos modificadores de Dioxus inclui:

- `default` - adiciona automaticamente o campo usando sua implementação `Default`
- `opcional` - alias para `default` e `strip_option`
- `into` - chama automaticamente `into` no valor no site de chamada

## A macro `inline_props`

Até agora, todas as funções `Component` que vimos tinham uma estrutura `ComponentProps` correspondente para passar em `props`. Isso foi bastante detalhado... Não seria legal ter props como argumentos de função simples? Então não precisaríamos definir uma estrutura Props, e ao invés de digitar `cx.props.whatever`, poderíamos usar `whatever` diretamente!

`inline_props` permite que você faça exatamente isso. Em vez de digitar a versão "completa":

```rust
#[derive(Props, PartialEq)]
struct TitleCardProps {
    title: String,
}

fn TitleCard(cx: Scope<TitleCardProps>) -> Element {
    cx.render(rsx!{
        h1 { "{cx.props.title}" }
    })
}
```

...você pode definir uma função que aceita props como argumentos. Então, basta anotá-lo com `#[inline_props]`, e a macro irá transformá-lo em um componente regular para você:

```rust
#[inline_props]
fn TitleCard(cx: Scope, title: String) -> Element {
    cx.render(rsx!{
        h1 { "{title}" }
    })
}
```

> Embora o novo Componente seja mais curto e fácil de ler, essa macro não deve ser usada por autores de bibliotecas, pois você tem menos controle sobre a documentação do Prop.
