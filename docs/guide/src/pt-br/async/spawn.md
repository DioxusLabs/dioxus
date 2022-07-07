# Gerando Futures

Os **"hooks"** `use_future` e `use_coroutine` são úteis se você quiser gerar incondicionalmente o `Future`. Às vezes, porém, você desejará apenas gerar um `Future` em resposta a um evento, como um clique do mouse. Por exemplo, suponha que você precise enviar uma solicitação quando o usuário clicar em um botão "log in". Para isso, você pode usar `cx.spawn`:

```rust
{{#include ../../examples/spawn.rs:spawn}}
```

> Nota: `spawn` sempre gerará um _novo_ `Future`. Você provavelmente não quer chamá-lo em cada renderização.

O `Future` deve ser `'static` – então quaisquer valores capturados pela tarefa não podem carregar nenhuma referência a `cx`, como um `UseState`.

No entanto, como você normalmente precisa de uma maneira de atualizar o valor de um gancho, você pode usar `to_owned` para criar um clone do _handle_ do _hook_. Você pode então usar esse clone no encerramento assíncrono.

Para tornar isso um pouco menos detalhado, o Dioxus exporta a macro `to_owned!` que criará uma ligação como mostrado acima, o que pode ser bastante útil ao lidar com muitos valores.

```rust
{{#include ../../examples/spawn.rs:to_owned_macro}}
```

Calling `spawn` will give you a `JoinHandle` which lets you cancel or pause the future.

## Gerando Tarefas do Tokio

Às vezes, você pode querer gerar uma tarefa em segundo plano que precise de vários _threads_ ou conversar com o hardware que pode bloquear o código do seu aplicativo. Nesses casos, podemos gerar diretamente uma tarefa Tokio do nosso `Future`. Para Dioxus-Desktop, sua tarefa será gerada no tempo de execução Multi-Tarefado do Tokio:

```rust
{{#include ../../examples/spawn.rs:tokio}}
```
