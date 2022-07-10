# UseFuture

Ao lidar com código assíncrono, talvez seja necessário aguardar a conclusão de alguma ação antes de renderizar seu componente. Se você mesmo tivesse que construir essa abstração, provavelmente acabaria com algum espaguete de código com `use_state`.

Um dos Hooks principais que o Dioxus fornece é `use_future` - um hook simples que permite que você ancore em uma tarefa em execução.

## Casos de Uso

O caso de uso mais simples de `use_future` é impedir a renderização até que algum código assíncrono seja concluído.

Atualmente, o Dioxus não possui uma biblioteca tão sofisticada quanto o React Query para tarefas de pré-busca, mas chegaremos lá com `use_future`. Em um dos exemplos do Dioxus, usamos `use_future` para baixar alguns dados de pesquisa antes de renderizar o restante do aplicativo:

```rust
fn app(cx: Scope) -> Element {
    // set "breeds" to the current value of the future
    let breeds = use_future(&cx, (), |_| async move {
        reqwest::get("https://dog.ceo/api/breeds/list/all")
            .await
            .unwrap()
            .json::<ListBreeds>()
            .await
    });

    let status = match breeds.value() {
        Some(Ok(val)) => "ready!",
        Some(Err(err)) => "errored!",
        None => "loading!",
    }
}
```

Na primeira execução, o código dentro de `use_future` será submetido ao agendador do Dioxus assim que o componente for renderizado. Como não há dados prontos quando o componente é carregado pela primeira vez, seu "valor" será `None`.

No entanto, uma vez finalizado o `Future`, o componente será renderizado novamente e uma nova tela será exibida - `Ok` ou `Err`, dependendo do resultado de nossa busca.

## Reiniciando o Future

O exemplo que mostramos acima só será executado uma vez. O que acontece se algum valor for alterado no servidor e precisarmos atualizar o valor do nosso `Future`?

Bem, o `UseFuture` fornece um método prático de "reinicialização". Podemos conectar isso a um botão ou algum outro código de comparação para obter um `Future` regenerativo.

```rust
fn app(cx: Scope) -> Element {
    // set "breeds" to the current value of the future
    let dog = use_future(&cx, (), |_| async move {
        reqwest::get("https://dog.ceo/api/breeds/image/random")
            .await
            .unwrap()
            .json::<RandomDog>()
            .await
    });

    cx.render(match breeds.value() {
        Some(Ok(val)) => rsx!(div {
            img { src: "{val.message}"}
            button {
                onclick: move |_| dog.restart(),
                "Click to fetch a new dog"
            }
        }),
        Some(Err(err)) => rsx!("Failed to load dog"),
        None => rsx!("Loading dog image!"),
    })
}
```

## Com Dependências

Mostramos que o `UseFuture` pode ser regenerado manualmente, mas como podemos fazer com que ele seja atualizado automaticamente sempre que algum valor de entrada for alterado?

É aqui que o `tuple` "dependências" entra em ação. Só precisamos adicionar um valor ao nosso argumento do `tuple` e ele será clonado automaticamente em nosso `Future` quando for iniciado.

```rust
#[inline_props]
fn RandomDog(cx: Scope, breed: String) -> Element {
    let dog = use_future(&cx, (breed,), |(breed)| async move {
        reqwest::get(format!("https://dog.ceo/api/breed/{breed}/images/random"))
            .await
            .unwrap()
            .json::<RandomDog>()
            .await
    });

    // some code as before
}
```
