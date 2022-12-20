# Manipulação de Erros

Um ponto forte do Rust para desenvolvimento Web é a confiabilidade de sempre saber onde os erros podem ocorrer e ser forçado a lidar com eles

No entanto, não falamos sobre tratamento de erros neste guia! Neste capítulo, abordaremos algumas estratégias para lidar com erros para garantir que seu aplicativo nunca falhe.

## O mais simples – retornando None

Observadores astutos podem ter notado que `Element` é na verdade um alias de tipo para `Option<VNode>`. Você não precisa saber o que é um `VNode`, mas é importante reconhecer que não poderíamos retornar nada:

```rust
fn App(cx: Scope) -> Element {
    None
}
```

Isso nos permite adicionar um pouco de açúcar sintático para operações que achamos que _não devem_ falhar, mas ainda não estamos confiantes o suficiente para "desempacotar".

> A natureza de `Option<VNode>` pode mudar no futuro à medida que a característica `try` for atualizada.

```rust
fn App(cx: Scope) -> Element {
    // immediately return "None"
    let name = cx.use_hook(|_| Some("hi"))?;
}
```

## Retorno Antecipado do Resultado

Como o Rust não pode aceitar opções e resultados com a infraestrutura _try_ existente, você precisará manipular os resultados manualmente. Isso pode ser feito convertendo-os em `Option` ou manipulando-os explicitamente.

```rust
fn App(cx: Scope) -> Element {
    // Convert Result to Option
    let name = cx.use_hook(|_| "1.234").parse().ok()?;


    // Early return
    let count = cx.use_hook(|_| "1.234");
    let val = match count.parse() {
        Ok(val) => val
        Err(err) => return cx.render(rsx!{ "Parsing failed" })
    };
}
```

Observe que enquanto os ganchos no Dioxus não gostam de ser chamados em condicionais ou loops, eles _estão_ bem com retornos antecipados. Retornar um estado de erro antecipadamente é uma maneira completamente válida de lidar com erros.

## Resultados usando `match`

A próxima "melhor" maneira de lidar com erros no Dioxus é combinar (`match`) o erro localmente. Essa é a maneira mais robusta de lidar com erros, embora não seja dimensionada para arquiteturas além de um único componente.

Para fazer isso, simplesmente temos um estado de erro embutido em nosso componente:

```rust
let err = use_state(cx, || None);
```

Sempre que realizarmos uma ação que gere um erro, definiremos esse estado de erro. Podemos então combinar o erro de várias maneiras (retorno antecipado, elemento de retorno etc.).

```rust
fn Commandline(cx: Scope) -> Element {
    let error = use_state(cx, || None);

    cx.render(match *error {
        Some(error) => rsx!(
            h1 { "An error occured" }
        )
        None => rsx!(
            input {
                oninput: move |_| error.set(Some("bad thing happened!")),
            }
        )
    })
}
```

## Passando Estados de Erro Através de Componentes

Se você estiver lidando com alguns componentes com um mínimo de aninhamento, basta passar o identificador de erro para componentes filhos.

```rust
fn Commandline(cx: Scope) -> Element {
    let error = use_state(cx, || None);

    if let Some(error) = **error {
        return cx.render(rsx!{ "An error occured" });
    }

    cx.render(rsx!{
        Child { error: error.clone() }
        Child { error: error.clone() }
        Child { error: error.clone() }
        Child { error: error.clone() }
    })
}
```

Assim como antes, nossos componentes filhos podem definir manualmente o erro durante suas próprias ações. A vantagem desse padrão é que podemos isolar facilmente os estados de erro para alguns componentes por vez, tornando nosso aplicativo mais previsível e robusto.

## Tornando Global

Uma estratégia para lidar com erros em cascata em aplicativos maiores é sinalizar um erro usando o estado global. Esse padrão específico envolve a criação de um contexto de "erro" e, em seguida, defini-lo sempre que relevante. Este método em particular não é tão "sofisticado" quanto o controle de erros do React, mas é mais adequado para Rust.

Para começar, considere usar um _hook_ embutido como `use_context` e `use_context_provider` ou `Fermi`. Claro, é muito fácil criar seu próprio _hook_ também.

No "topo" de nossa arquitetura, queremos declarar explicitamente um valor que pode ser um erro.

```rust
enum InputError {
    None,
    TooLong,
    TooShort,
}

static INPUT_ERROR: Atom<InputError> = |_| InputError::None;
```

Então, em nosso componente de nível superior, queremos tratar explicitamente o possível estado de erro para esta parte da árvore.

```rust
fn TopLevel(cx: Scope) -> Element {
    let error = use_read(cx, INPUT_ERROR);

    match error {
        TooLong => return cx.render(rsx!{ "FAILED: Too long!" }),
        TooShort => return cx.render(rsx!{ "FAILED: Too Short!" }),
        _ => {}
    }
}
```

Agora, sempre que um componente _downstream_ tiver um erro em suas ações, ele pode simplesmente definir seu próprio estado de erro:

```rust
fn Commandline(cx: Scope) -> Element {
    let set_error = use_set(cx, INPUT_ERROR);

    cx.render(rsx!{
        input {
            oninput: move |evt| {
                if evt.value.len() > 20 {
                    set_error(InputError::TooLong);
                }
            }
        }
    })
}
```

Essa abordagem de tratamento de erros é melhor em aplicativos que têm estados de erro "bem definidos". Considere usar uma `crate` como `thiserror` ou `anyhow` para simplificar a geração dos tipos de erro.

Esse padrão é amplamente popular em muitos contextos e é particularmente útil sempre que seu código gera um erro irrecuperável. Você pode capturar esses estados de erro "globais" resultar em `panic!` ou estragar o estado.
