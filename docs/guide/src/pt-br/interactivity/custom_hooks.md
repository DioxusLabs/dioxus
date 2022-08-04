# Hooks Personalizados

_Hooks_ são uma ótima maneira de encapsular a lógica de negócio. Se nenhum dos _hooks_ existentes funcionar para o seu problema, você pode escrever o seu próprio.

## Hooks de Composição

Para evitar a repetição, você pode encapsular a lógica de negócios com base em _hooks_ existentes para criar um novo gancho.

Por exemplo, se muitos componentes precisam acessar uma _struct_ `AppSettings`, você pode criar um gancho de "atalho":

```rust
{{#include ../../../examples/hooks_composed.rs:wrap_context}}
```

## Lógica de Hook Personalizada

Você pode usar [`cx.use_hook`](https://docs.rs/dioxus/latest/dioxus/prelude/struct.Scope.html#method.use_hook) para construir seus próprios _hooks_. Na verdade, é nisso que todos os _hooks_ padrão são construídos!

`use_hook` aceita um único encerramento para inicializar o _hook_. Ele será executado apenas na primeira vez que o componente for renderizado. O valor de retorno desse encerramento será usado como o valor do _hook_ – o Dioxus o pegará e o armazenará enquanto o componente estiver vivo. Em cada renderização (não apenas na primeira!), você receberá uma referência a esse valor.

> Nota: Você pode implementar [`Drop`](https://doc.rust-lang.org/std/ops/trait.Drop.html) para o valor do seu _hook_ – ele será descartado e o componente será desmontado (não mais na interface do usuário)

Dentro do encerramento de inicialização, você normalmente fará chamadas para outros métodos `cx`. Por exemplo:

- O _hook_ `use_state` rastreia o estado no valor do _hook_ e usa [`cx.schedule_update`](https://docs.rs/dioxus/latest/dioxus/prelude/struct.Scope.html#method.schedule_update) para o Dioxus renderizar novamente o componente sempre que ele for alterado.
- O _hook_ `use_context` chama [`cx.consume_context`](https://docs.rs/dioxus/latest/dioxus/prelude/struct.Scope.html#method.consume_context) (que seria custoso chamar em cada render) para obter algum contexto do escopo
