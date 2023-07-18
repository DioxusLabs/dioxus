# Corrotinas

Outra boa ferramenta para manter em sua caixa de ferramentas assíncrona são as corrotinas. Corrotinas são `Futures` que podem ser interrompidos, iniciados, pausados e retomados manualmente.

Assim como os `Futures` regulares, o código em uma corrotina Dioxus será executado até o próximo ponto `await` antes do _render_. Esse controle de baixo nível sobre tarefas assíncronas é bastante poderoso, permitindo tarefas em _loop_ infinito, como pesquisa de WebSocket, temporizadores em segundo plano e outras ações periódicas.

## `use_coroutine`

A configuração básica para corrotinas é o _hook_ `use_coroutine`. A maioria das corrotinas que escrevemos serão _loops_ de pesquisa usando `async`/`await`.

```rust, no_run
fn app(cx: Scope) -> Element {
    let ws: &UseCoroutine<()> = use_coroutine(cx, |rx| async move {
        // Connect to some sort of service
        let mut conn = connect_to_ws_server().await;

        // Wait for data on the service
        while let Some(msg) = conn.next().await {
            // handle messages
        }
    });
}
```

Para muitos serviços, um _loop_ assíncrono simples lidará com a maioria dos casos de uso.

No entanto, se quisermos desabilitar temporariamente a corrotina, podemos "pausá-la" usando o método `pause` e "retomá-la" usando o método `resume`:

```rust, no_run
let sync: &UseCoroutine<()> = use_coroutine(cx, |rx| async move {
    // code for syncing
});

if sync.is_running() {
    cx.render(rsx!{
        button {
            onclick: move |_| sync.pause(),
            "Disable syncing"
        }
    })
} else {
    cx.render(rsx!{
        button {
            onclick: move |_| sync.resume(),
            "Enable syncing"
        }
    })
}
```

Esse padrão é onde as corrotinas são extremamente úteis – em vez de escrever toda a lógica complicada para pausar nossas tarefas assíncronas como faríamos com `Promises` de JavaScript, o modelo do Rust nos permite simplesmente não pesquisar nosso `Future`.

## Enviando valores

Você deve ter notado que o encerramento `use_coroutine` recebe um argumento chamado `rx`. O que é aquilo? Bem, um padrão comum em aplicativos complexos é lidar com vários códigos assíncronos de uma só vez. Com bibliotecas como o Redux Toolkit, gerenciar várias promessas ao mesmo tempo pode ser um desafio e uma fonte comum de _bugs_.

Usando corrotinas, temos a oportunidade de centralizar nossa lógica assíncrona. O parâmetro `rx` é um canal ilimitado para código externo à corrotina para enviar dados _para_ a corrotina. Em vez de fazer um _loop_ em um serviço externo, podemos fazer um _loop_ no próprio canal, processando mensagens de dentro de nosso aplicativo sem precisar gerar um novo `Future`. Para enviar dados para a corrotina, chamaríamos "send" no _handle_.

```rust, no_run
enum ProfileUpdate {
    SetUsername(String),
    SetAge(i32)
}

let profile = use_coroutine(cx, |mut rx: UnboundedReciver<ProfileUpdate>| async move {
    let mut server = connect_to_server().await;

    while let Ok(msg) = rx.next().await {
        match msg {
            ProfileUpdate::SetUsername(name) => server.update_username(name).await,
            ProfileUpdate::SetAge(age) => server.update_age(age).await,
        }
    }
});


cx.render(rsx!{
    button {
        onclick: move |_| profile.send(ProfileUpdate::SetUsername("Bob".to_string())),
        "Update username"
    }
})
```

Para aplicativos suficientemente complexos, poderíamos criar vários "serviços" úteis diferentes que fazem um _loop_ nos canais para atualizar o aplicativo.

```rust, no_run
let profile = use_coroutine(cx, profile_service);
let editor = use_coroutine(cx, editor_service);
let sync = use_coroutine(cx, sync_service);

async fn profile_service(rx: UnboundedReceiver<ProfileCommand>) {
    // do stuff
}

async fn sync_service(rx: UnboundedReceiver<SyncCommand>) {
    // do stuff
}

async fn editor_service(rx: UnboundedReceiver<EditorCommand>) {
    // do stuff
}
```

Podemos combinar corrotinas com `Fermi` para emular o sistema `Thunk` do **Redux Toolkit** com muito menos dor de cabeça. Isso nos permite armazenar todo o estado do nosso aplicativo _dentro_ de uma tarefa e, em seguida, simplesmente atualizar os valores de "visualização" armazenados em `Atoms`. Não pode ser subestimado o quão poderosa é essa técnica: temos todas as vantagens das tarefas nativas do Rust com as otimizações e ergonomia do estado global. Isso significa que seu estado _real_ não precisa estar vinculado a um sistema como `Fermi` ou `Redux` – os únicos `Atoms` que precisam existir são aqueles que são usados para controlar a interface.

```rust, no_run
static USERNAME: Atom<String> = Atom(|_| "default".to_string());

fn app(cx: Scope) -> Element {
    let atoms = use_atom_root(cx);

    use_coroutine(cx, |rx| sync_service(rx, atoms.clone()));

    cx.render(rsx!{
        Banner {}
    })
}

fn Banner(cx: Scope) -> Element {
    let username = use_read(cx, &USERNAME);

    cx.render(rsx!{
        h1 { "Welcome back, {username}" }
    })
}
```

Agora, em nosso serviço de sincronização, podemos estruturar nosso estado como quisermos. Só precisamos atualizar os valores da _view_ quando estiver pronto.

```rust, no_run
enum SyncAction {
    SetUsername(String),
}

async fn sync_service(mut rx: UnboundedReceiver<SyncAction>, atoms: AtomRoot) {
    let username = atoms.write(&USERNAME);
    let errors = atoms.write(&ERRORS);

    while let Ok(msg) = rx.next().await {
        match msg {
            SyncAction::SetUsername(name) => {
                if set_name_on_server(&name).await.is_ok() {
                    username.set(name);
                } else {
                    errors.make_mut().push("SetUsernameFailed");
                }
            }
        }
    }
}
```

## Valores de Rendimento

Para obter valores de uma corrotina, basta usar um identificador `UseState` e definir o valor sempre que sua corrotina concluir seu trabalho.

```rust, no_run
let sync_status = use_state(cx, || Status::Launching);
let sync_task = use_coroutine(cx, |rx: UnboundedReceiver<SyncAction>| {
    to_owned![sync_status];
    async move {
        loop {
            delay_ms(1000).await;
            sync_status.set(Status::Working);
        }
    }
})
```

## Injeção Automática na API de Contexto

Os identificadores de corrotina são injetados automaticamente por meio da API de contexto. `use_coroutine_handle` com o tipo de mensagem como genérico pode ser usado para buscar um _handle_.

```rust, no_run
fn Child(cx: Scope) -> Element {
    let sync_task = use_coroutine_handle::<SyncAction>(cx);

    sync_task.send(SyncAction::SetUsername);
}
```
