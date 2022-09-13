# Componente Filhos

Em alguns casos, você pode desejar criar um componente que atue como um contêiner para algum outro conteúdo, sem que o componente precise saber qual é esse conteúdo. Para conseguir isso, crie uma _prop_ do tipo `Element`:

```rust
{{#include ../../../examples/component_element_props.rs:Clickable}}
```

Então, ao renderizar o componente, você pode passar a saída de `cx.render(rsx!(...))`:

```rust
{{#include ../../../examples/component_element_props.rs:Clickable_usage}}
```

> Nota: Como `Element<'a>` é uma _prop_ emprestado, não haverá memoização.

> Atenção: Embora possa compilar, não inclua o mesmo `Element` mais de uma vez no RSX. O comportamento resultante não é especificado.

## O Campo `children`

Em vez de passar o `RSX` através de uma _prop_ regular, você pode querer aceitar filhos da mesma forma que os elementos podem ter filhos. O prop "mágico" `children` permite que você consiga isso:

```rust
{{#include ../../../examples/component_children.rs:Clickable}}
```

Isso torna o uso do componente muito mais simples: basta colocar o `RSX` dentro dos colchetes `{}` – e não há necessidade de uma chamada `render` ou outra macro!

```rust
{{#include ../../../examples/component_children.rs:Clickable_usage}}
```
