# `UseFuture`

[`use_future`](https://docs.rs/dioxus-hooks/latest/dioxus_hooks/fn.use_future.html) permite executar um encerramento assíncrono e fornece seu resultado.

Por exemplo, podemos fazer uma solicitação de API dentro de `use_future`:

```rust, no_run
{{#include ../../../examples/use_future.rs:use_future}}
```

O código dentro de `use_future` será enviado ao agendador do Dioxus assim que o componente for renderizado.

Podemos usar `.value()` para obter o resultado do `Future`. Na primeira execução, como não há dados prontos quando o componente é carregado, seu valor será `None`. No entanto, uma vez finalizado o `Future`, o componente será renderizado novamente e o valor agora será `Some(...)`, contendo o valor de retorno do encerramento.

Podemos então renderizar esse resultado:

```rust, no_run
{{#include ../../../examples/use_future.rs:render}}
```

## Reiniciando o `Future`

O identificador `UseFuture` fornece um método `restart`. Ele pode ser usado para executar o `Future` novamente, produzindo um novo valor.

## Dependências

Muitas vezes, você precisará executar o `Future` novamente toda vez que algum valor (por exemplo, uma prop) mudar. Ao invés de `.restart` manualmente, você pode fornecer uma tupla de "dependências" para o gancho. Ele executará automaticamente o `Future` quando qualquer uma dessas dependências for alterada. Exemplo:

```rust, no_run
{{#include ../../../examples/use_future.rs:dependency}}
```
