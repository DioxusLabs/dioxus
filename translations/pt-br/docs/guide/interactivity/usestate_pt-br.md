# Hook Fundamental: `use_state`

O primeiro hook fundamental para gerenciamento de estado é `use_state`. Este hook em particular foi projetado para funcionar bem com todo o ecossistema Dioxus, incluindo `Futures`, filhos e memoização.

## Uso Básico

O caso de uso mais simples do `use_state` é um contador simples. O `handle` retornado por `use_state` implementa `Add` e `AddAssign`. Escrever através de um `AddAssign` marcará automaticamente o componente como sujo, forçando uma atualização.

```rust
fn app(cx: Scope) -> Element {
    let count = use_state(&cx, || 0);

    rsx!(cx, button { onclick: move |_| count += 1, })
}
```

## Referência

### Operações comuns

O hook `use_state` é muito semelhante ao seu homólogo React. Quando o usamos, obtemos uma ponteira inteligente - essencialmente um `Rc<T>` que rastreia quando o atualizamos.

```rust
let mut count = use_state(&cx, || 0);

// then to set the count
count += 1;
```

### Valor atual

Nosso valor `count` é mais do que apenas um inteiro. Por ser uma ponteira inteligente, ele também implementa outros métodos úteis em vários contextos.

Por exemplo, podemos obter um identificador para o valor atual a qualquer momento:

```rust
let current: Rc<T> = count.current();
```

Ou podemos obter o identificador interno para definir o valor:

```rust
let setter: Rc<dyn Fn(T)> = count.setter();
```

### Modificar

Ou podemos definir um novo valor usando o valor atual como referência:

```rust
count.modify(|i| i + 1);
```

### `with_mut` e `make_mut`

Se o valor for clonável de forma simples, podemos chamar `with_mut` para obter uma referência mutável para o valor:

```rust
count.with_mut(|i| *i += 1);
```

Alternativamente, podemos chamar `make_mut` que nos dá um `RefMut` para o valor subjacente:

```rust
*count.make_mut() += 1;
```

### Use em Assincronia

Além disso, o identificador `set_count` pode ser clonado em contextos assíncronos, para que possamos usá-lo em `Futures`.

```rust
// count up infinitely
cx.spawn({
    let count = count.clone();
    async move {
        loop {
            wait_ms(100).await;
            count += 1;
        }
    }
})
```

## Usando UseState para dados simples

O melhor caso de uso do `use_state` é gerenciando dados "simples" locais para seu componente. Isto é, como números, strings, mapas pequenos e tipos de clonagem simples.

```rust
let val = use_state(&cx, || 0);
let val = use_state(&cx, || "Hello!");
let val = use_state(&cx, || vec![1, 3, 3, 7]);
```

`UseState` também pode ser enviado para componentes filhos.

Você pode passar por referência para sempre forçar o filho a atualizar quando o valor for alterado ou pode clonar o identificador para aproveitar a memoização automática. Nesses casos, chamar "get" retornará dados obsoletos - sempre prefira "current" ao "clonar" o `UseState`. Essa memoização automática do `UseState` resolve um problema de desempenho comum no React.

```rust

fn app(cx: Scope) -> Element {
    let val = use_state(&cx, || 0);

    cx.render(rsx!{
        Child { val: val.clone() }
    })
}

fn Child(cx: Scope, val: UseState<i32>) -> Element {
    // ...
}
```
