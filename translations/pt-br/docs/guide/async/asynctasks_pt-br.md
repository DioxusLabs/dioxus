# Tarefas

Todo código assíncrono no Dioxus deve ser explícito e tratado através do sistema de tarefas do Dioxus.

Neste capítulo, aprenderemos como gerar novas tarefas através do nosso `Scope`.

## Gerando uma Tarefa

Você pode enviar qualquer `Future` `'static` para a fila de `Futures` do Dioxus simplesmente chamando `cx.spawn` para gerar uma nova tarefa. Empilhar um `Future` retorna um `TaskId` que pode ser usado para cancelar se for preciso:

```rust
fn App(cx: Scope) -> Element {
    cx.spawn(async {
        let mut count = 0;
        loop {
            tokio::time::delay(std::instant::Duration::from_millis(500)).await;
            count += 1;
            println!("Current count is {}", count);
        }
    });

    None
}
```

O `Future` deve ser `'static` - então quaisquer valores capturados pela tarefa não devem conter nenhuma referência a `cx`. Todos os hooks do Dioxus têm um método chamado `for_async` que criará um `handle` um pouco mais limitado para o hook para você usar em seu código assíncrono.

```rust
fn App(cx: Scope) -> Element {
    let mut count = use_state(&cx, || 0);

    let taskid = cx.spawn({
        let mut count = count.for_async();
        async {
            loop {
                tokio::time::delay(std::instant::Duration::from_millis(500)).await;
                count += 1;
                println!("Current count is {}", count);
            }
        }
    });
}
```

A tarefa será executada em segundo plano até que seja concluída.

> Nota: `spawn` sempre gerará um _novo_ `Future`. Você provavelmente terá de chamá-lo por um Hook inicializador em vez do corpo principal do seu componente.

Ao trazer muitos valores para sua tarefa, nós fornecemos a macro `for_async!` que poderá aplicar `for_async` em todos os valores passados. Para tipos que implementam `ToOwned`, `for_async!` simplesmente chamará `ToOwned` para esse valor .

```rust
fn App(cx: Scope) -> Element {
    let mut age = use_state(&cx, || 0);
    let mut name = use_state(&cx, || "Bob");
    let mut description = use_state(&cx, || "asd".to_string());

    let taskid = cx.spawn({
        for_async![count, name, description]
        async { /* code that uses count/name/description */ }
    });
}
```

## Detalhes de Tarefas

Chamar `spawn` _não_ é o mesmo que chamar um Hook e _sempre_ gerará uma nova tarefa. Certifique-se de gerar tarefas apenas quando precisar. Você deve _provavelmente_ não usar `spawn` no corpo principal do seu componente, já que uma nova tarefa será gerada em cada renderização.

## Gerando Tarefas com o Tokio (para casos de multitarefas)

Às vezes, você pode querer gerar uma tarefa em segundo plano que precise de vários processos ou lidar com o hardware que pode bloquear o código do seu aplicativo. Nesses casos, podemos gerar diretamente uma `tarefa tokio` do nosso `Future`. Para Dioxus-Desktop, sua tarefa será gerada no tempo de execução multitarefas do Tokio:

```rust
cx.spawn({
    tokio::spawn(async {
        // some multithreaded work
    }).await;

    tokio::spawn_blocking(|| {
        // some extremely blocking work
    }).await;

    tokio::spawn_local(|| {
        // some !Send work
    }).await;
})
```

> Nota: As tarefas do Tokio devem ser `Send`. A maioria dos hooks são compatíveis com `Send`, mas se não forem, então você pode usar `spawn_local` para gerar no `localset` do Dioxus-Desktop.
