# Manipuladores de eventos

Eventos são ações interessantes que acontecem, geralmente relacionadas a algo que o usuário fez. Por exemplo, alguns eventos acontecem quando o usuário clica, rola, move o mouse ou digita um caractere. Para responder a essas ações e tornar a interface do usuário interativa, precisamos lidar com esses eventos.

Os eventos estão associados a elementos. Por exemplo, geralmente não nos importamos com todos os cliques que acontecem em um aplicativo, apenas aqueles em um botão específico. Para lidar com eventos que acontecem em um elemento, devemos anexar o manipulador de eventos desejado a ele.

Os manipuladores de eventos são semelhantes aos atributos regulares, mas seus nomes geralmente começam com `on` - e aceitam encerramentos como valores. O encerramento será chamado sempre que o evento for acionado e será passado esse evento.

Por exemplo, para manipular cliques em um elemento, podemos especificar um manipulador `onclick`:

```rust, no_run
{{#include ../../../examples/event_click.rs:rsx}}
```

## O Objeto `Evento`

Os manipuladores de eventos recebem um objeto [`UiEvent`](https://docs.rs/dioxus-core/latest/dioxus_core/struct.UiEvent.html) contendo informações sobre o evento. Diferentes tipos de eventos contêm diferentes tipos de dados. Por exemplo, eventos relacionados ao mouse contêm [`MouseData`](https://docs.rs/dioxus/latest/dioxus/events/struct.MouseData.html), que informa coisas como onde o mouse foi clicado e quais botões do mouse foram usados.

No exemplo acima, esses dados de evento foram registrados no terminal:

```
Clicked! Event: UiEvent { data: MouseData { coordinates: Coordinates { screen: (468.0, 109.0), client: (73.0, 25.0), element: (63.0, 15.0), page: (73.0, 25.0) }, modifiers: (empty), held_buttons: EnumSet(), trigger_button: Some(Primary) } }
Clicked! Event: UiEvent { data: MouseData { coordinates: Coordinates { screen: (468.0, 109.0), client: (73.0, 25.0), element: (63.0, 15.0), page: (73.0, 25.0) }, modifiers: (empty), held_buttons: EnumSet(), trigger_button: Some(Primary) } }
```

Para saber o que os diferentes tipos de eventos fornecem, leia os [documentos do módulo de eventos](https://docs.rs/dioxus/latest/dioxus/events/index.html).

### Parando a propagação

Quando você tem, por exemplo um `button` dentro de um `div`, qualquer clique no `button` também é um clique no `div`. Por esta razão, Dioxus propaga o evento click: primeiro, ele é acionado no elemento alvo, depois nos elementos pai. Se você quiser evitar esse comportamento, você pode chamar `cancel_bubble()` no evento:

```rust, no_run
{{#include ../../../examples/event_click.rs:rsx}}
```

## Prevenir o Padrão

Alguns eventos têm um comportamento padrão. Para eventos de teclado, isso pode ser inserir o caractere digitado. Para eventos de mouse, isso pode estar selecionando algum texto.

Em alguns casos, você pode querer evitar esse comportamento padrão. Para isso, você pode adicionar o atributo `prevent_default` com o nome do manipulador cujo comportamento padrão você deseja interromper. Este atributo é especial: você pode anexá-lo várias vezes para vários atributos:

```rust, no_run
{{#include ../../../examples/event_prevent_default.rs:prevent_default}}
```

Quaisquer manipuladores de eventos ainda serão chamados.

> Normalmente, em React ou JavaScript, você chamaria "preventDefault" no evento no retorno de chamada. Dioxus atualmente _não_ suporta este comportamento. Observação: isso significa que você não pode impedir condicionalmente o comportamento padrão.

## Manipulador de Props

Às vezes, você pode querer criar um componente que aceite um manipulador de eventos. Um exemplo simples seria um componente `FancyButton`, que aceita um manipulador `on_click`:

```rust, no_run
{{#include ../../../examples/event_handler_prop.rs:component_with_handler}}
```

Então, você pode usá-lo como qualquer outro manipulador:

```rust, no_run
{{#include ../../../examples/event_handler_prop.rs:usage}}
```

> Nota: assim como qualquer outro atributo, você pode nomear os manipuladores como quiser! Embora eles devam começar com `on`, para que o prop seja automaticamente transformado em um `EventHandler` no local da chamada.
>
> Você também pode colocar dados personalizados no evento, em vez de, por exemplo, `MouseData`

> Nota sobre formulários: se um manipulador de evento está anexado ao evento `onsubmit` em um formulário, o comportamento padrão é de **não submetê-lo**. Portanto, especificar `prevent_default: "onsubmit"` irá submetê-lo.