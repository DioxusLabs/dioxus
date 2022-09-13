# Atributos Especiais

Enquanto a maioria dos atributos são simplesmente passados para o HTML, alguns têm comportamentos especiais.

## A Escotilha de Escape do HTML

Se você estiver trabalhando com itens pré-renderizados, modelos ou uma biblioteca JS, convém passar o HTML diretamente em vez de passar pelo Dioxus. Nesses casos, use `dangerous_inner_html`.

Por exemplo, enviar um conversor de markdown para Dioxus pode aumentar significativamente o tamanho final do aplicativo. Em vez disso, você desejará pré-renderizar sua remarcação para HTML e, em seguida, incluir o HTML diretamente em sua saída. Usamos essa abordagem para a [página inicial do Dioxus](https://dioxuslabs.com):

```rust
{{#include ../../../examples/dangerous_inner_html.rs:dangerous_inner_html}}
```

> Nota! Esse atributo é chamado de "dangerous_inner_html" porque é **perigoso** passar dados que você não confia. Se você não for cuidadoso, poderá facilmente expor ataques de [cross-site scripting (XSS)](https://en.wikipedia.org/wiki/Cross-site_scripting) aos seus usuários.
>
> Se você estiver lidando com entradas não confiáveis, certifique-se de higienizar seu HTML antes de passá-lo para `dangerous_inner_html` – ou apenas passe-o para um elemento de texto para escapar de qualquer tag HTML.

## Atributos Booleanos

A maioria dos atributos, quando renderizados, serão renderizados exatamente como a entrada que você forneceu. No entanto, alguns atributos são considerados atributos "booleanos" e apenas sua presença determina se eles afetam a saída. Para esses atributos, um valor fornecido de `"false"` fará com que eles sejam removidos do elemento de destino.

Portanto, este RSX não renderizaria o atributo `hidden`:

```rust
{{#include ../../../examples/boolean_attribute.rs:boolean_attribute}}
```

```html
<div>hello</div>
```

No entanto, nem todos os atributos funcionam assim. _Apenas os seguintes atributos_ têm este comportamento:

- `allowfullscreen`
- `allowpaymentrequest`
- `async`
- `autofocus`
- `autoplay`
- `checked`
- `controls`
- `default`
- `defer`
- `disabled`
- `formnovalidate`
- `hidden`
- `ismap`
- `itemscope`
- `loop`
- `multiple`
- `muted`
- `nomodule`
- `novalidate`
- `open`
- `playsinline`
- `readonly`
- `required`
- `reversed`
- `selected`
- `truespeed`

Para quaisquer outros atributos, um valor de `"false"` será enviado diretamente para o DOM.
