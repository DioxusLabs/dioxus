# Renderização condicional

Seus componentes geralmente precisarão exibir coisas diferentes dependendo de diferentes condições. Com o Dioxus, podemos usar o fluxo de controle do Rust para ocultar, mostrar e modificar condicionalmente a estrutura de nossa marcação.

Neste capítulo, você aprenderá:

- Como retornar elementos diferentes dependendo de uma condição
- Como incluir condicionalmente um elemento em sua estrutura
- Padrões comuns como correspondência e mapeamento bool

## Elementos de Retorno Condicional

Em alguns componentes, você pode querer renderizar uma marcação diferente dada alguma condição. O exemplo típico de renderização condicional mostra uma tela "Log In" para usuários que não estão conectados ao seu aplicativo. Para quebrar essa condição, podemos considerar dois estados:

- Logado: mostre o aplicativo
- Desconectado: mostra a tela de login

Usando o conhecimento da seção anterior sobre componentes, começaremos fazendo os adereços do aplicativo:

```rust
#[derive(Props, PartialEq)]
struct AppProps {
    logged_in: bool
}
```

Agora que temos um sinalizador "logged_in" acessível em nossas `props`, podemos renderizar duas telas diferentes:

```rust
fn App(cx: Scope<AppProps>) -> Element {
    if cx.props.logged_in {
        cx.render(rsx!{
            DashboardScreen {}
        })
    } else {
        cx.render(rsx!{
            LoginScreen {}
        })
    }
}
```

Quando o usuário estiver logado, esse componente retornará o `DashboardScreen`. Se eles não estiverem logados, o componente renderizará a `LoginScreen`.

## Usando Declarações de Correspondência

Rust nos fornece tipos de dados algébricos: `enums` que podem conter valores. Usando a palavra-chave `match`, podemos executar diferentes ramificações de código dada uma condição.

Por exemplo, poderíamos executar uma função que retorna um Result:

```rust
fn App(cx: Scope)-> Element {
    match get_name() {
        Ok(name) => cx.render(rsx!( "Hello, {name}!" )),
        Err(err) => cx.render(rsx!( "Sorry, I don't know your name, because an error occurred: {err}" )),
    }
}
```

Podemos até corresponder a valores:

```rust
fn App(cx: Scope)-> Element {
    match get_name() {
        "jack" => cx.render(rsx!( "Hey Jack, how's Diane?" )),
        "diane" => cx.render(rsx!( "Hey Diane, how's Jack?" )),
        name => cx.render(rsx!( "Hello, {name}!" )),
    }
}
```

Observe: o macro `rsx!` não retorna um `Element`, mas sim uma `struct` encapsulado para um `Closure` (uma função anônima). Para transformar nosso `rsx!` em um elemento, precisamos chamar `cx.render`.

Para tornar padrões como esses menos detalhados, a macro `rsx!` aceita um primeiro argumento opcional no qual chamará `render`. Nosso componente anterior pode ser encurtado com esta sintaxe alternativa:

```rust
fn App(cx: Scope)-> Element {
    match get_name() {
        "jack" => rsx!(cx, "Hey Jack, how's Diane?" ),
        "diane" => rsx!(cx, "Hey Diana, how's Jack?" ),
        name => rsx!(cx, "Hello, {name}!" ),
    }
}
```

Alternativamente, para instruções `match`, podemos apenas retornar o próprio construtor e passá-lo para uma única chamada final para `cx.render`:

```rust
fn App(cx: Scope)-> Element {
    let greeting = match get_name() {
        "jack" => rsx!("Hey Jack, how's Diane?" ),
        "diane" => rsx!("Hey Diana, how's Jack?" ),
        name => rsx!("Hello, {name}!" ),
    };
    cx.render(greeting)
}
```

## Aninhamento de RSX

Observando outros exemplos, você deve ter notado que é possível incluir chamadas `rsx!` dentro de outras chamadas `rsx!`. Podemos incluir qualquer coisa em nosso `rsx!` que implemente `IntoVnodeList`: uma `trait` de marcador para iteradores que produzem `Elements`. O próprio `rsx!` implementa essa `trait`, então podemos incluí-la diretamente:

```rust
rsx!(
    div {
        rsx!(
            "more rsx!"
        )
    }
)
```

Como você pode esperar, podemos refatorar essa estrutura em duas chamadas separadas usando variáveis:

```rust
let title = rsx!( "more rsx!" );

rsx!(
    div {
        title
    }
)
```

No caso de uma tela de login, talvez queiramos exibir a mesma `NavBar` e `Footer` para usuários conectados e desconectados. Podemos modelar isso inteiramente atribuindo uma variável `screen` a um elemento diferente dependendo de uma condição:

```rust
let screen = match logged_in {
    true => rsx!(DashboardScreen {}),
    false => rsx!(LoginScreen {})
};

cx.render(rsx!{
    Navbar {}
    screen,
    Footer {}
})
```

## Renderizando Nada

Às vezes, você não quer que seu componente retorne nada. Nos bastidores, o tipo `Element` é apenas um `alias` para `Option<VNode>`, então você pode simplesmente retornar `None`.

Isso pode ser útil em certos padrões em que você precisa executar alguns efeitos colaterais lógicos, mas não deseja renderizar nada.

```rust
fn demo(cx: Scope) -> Element {
    None
}
```

## Mapeamento booleano

No espírito de aplicativos altamente funcionais, sugerimos usar o padrão "mapeamento booleano" ao tentar ocultar/mostrar condicionalmente um elemento.

Por padrão, o Rust permite converter qualquer `booleano` em uma opção de qualquer outro tipo com [`then()`](https://doc.rust-lang.org/std/primitive.bool.html#method.then). Podemos usar isso em Componentes mapeando para algum `Element`.

```rust
let show_title = true;
rsx!(
    div {
        // Renders nothing by returning None when show_title is false
        show_title.then(|| rsx!{
            "This is the title"
        })
    }
)
```

Podemos usar esse padrão para muitas coisas, incluindo opções:

```rust
let user_name = Some("bob");
rsx!(
    div {
        // Renders nothing if user_name is None
        user_name.map(|name| rsx!("Hello {name}"))
    }
)
```

## Seguindo em Frente:

Neste capítulo, aprendemos como renderizar diferentes Elementos de um Componente dependendo de uma condição. Este é um bloco de construção muito poderoso para montar interfaces de usuário complexas!

No próximo capítulo, abordaremos como renderizar listas dentro do seu `rsx!`.
