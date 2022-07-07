# UseRef

Você deve ter notado uma limitação fundamental para `use_state`: para modificar o valor no local, ele deve ser clonável de forma barata. Mas e se o seu tipo não for barato para clonar?

Nesses casos, você deve usar `use_ref` que é essencialmente apenas um `Rc<RefCell<T>>` (Rust [smart pointers](https://doc.rust-lang.org/book/ch15-04 -rc.html)).

Isso nos fornece alguns bloqueios de tempo de execução em torno de nossos dados, trocando confiabilidade por desempenho. Para a maioria dos casos, porém, você achará difícil fazer o `use_ref` travar por falta de desempenho.

> Nota: este `use_ref` _não_ é exatamente igual ao React já que chamar `write` causará uma re-renderização. Para mais paridade do React, use o método `write_silent`.

Para usar o hook:

```rust
let names = use_ref(&cx, || vec!["susie", "calvin"]);
```

Para ler os valores do hook, use o método `read()`:

```rust
cx.render(rsx!{
    ul {
        names.read().iter().map(|name| {
            rsx!{ "{name}" }
        })
    }
})
```

E então para escrever no valor do hook, use o método `write`:

```rust
names.write().push("Tiger");
```

If you don't want to re-render the component when names is updated, then we can use the `write_silent` method:

```rust
names.write_silent().push("Transmogrifier");
```

Novamente, como `UseState`, o identificador `UseRef` é clonável em contextos assíncronos:

```rust
// infinitely push calvin into the list
cx.spawn({
    to_owned![names];
    async move {
        loop {
            wait_ms(100).await;
            names.write().push("Calvin");
        }
    }
})
```

## Usando UseRef para estado complexo

O melhor caso de uso do `use_ref` é gerenciando dados "complexos" locais para seu componente. Isso seria como grandes estruturas, coleções, filas, máquinas de estado e outros dados em que a clonagem seria muito custosa e não performática.

```rust
let val = use_ref(&cx, || vec![1, 3, 3, 7]);
let val = use_ref(&cx, || (0..10000).collect::<Vec<_>x>());
let val = use_ref(&cx, || Configuration {
    a: "asdasd",
    // .. more complex fields
});
```

`UseRef` também pode ser enviado para componentes filhos.

`UseRef` memoiza consigo mesmo, realizando uma comparação de ponteira simples. Se o identificador `UseRef` for o mesmo, o componente poderá ser memoizado.

```rust

fn app(cx: Scope) -> Element {
    let val = use_ref(&cx, || 0);

    cx.render(rsx!{
        Child { val: val.clone() }
    })
}

#[inline_props]
fn Child(cx: Scope, val: UseRef<i32>) -> Element {
    // ...
}
```

## Usando UseRef com "models"

Uma opção para gerenciamento de estado que o `UseRef` permite é o desenvolvimento de um "modelo" para seus componentes. Esse padrão específico permite modelar seu estado com estruturas regulares.

Por exemplo, nosso exemplo de calculadora usa uma estrutura para modelar o estado.

```rust

struct Calculator {
    display_value: String,
    operator: Option<Operator>,
    waiting_for_operand: bool,
    cur_val: f64,
}
```

Nosso componente é muito simples - nós apenas chamamos `use_ref` para obter um estado inicial da calculadora.

```rust
fn app(cx: Scope) -> Element {
    let state = use_ref(&cx, Calculator::new);

    cx.render(rsx!{
        // the rendering code
    })
}
```

Em nossa interface do usuário, podemos usar `read` e um método auxiliar para obter dados do modelo.

```rust
// Our accessor method
impl Calculator {
    fn formatted_display(&self) -> String {
        self.display_value
            .parse::<f64>()
            .unwrap()
            .separated_string()
    }
}

// And then in the UI
cx.render(rsx!{
    div { [state.read().formatted_display()] }
})
```

Para modificar o estado, podemos configurar um método auxiliar e anexá-lo a um retorno de chamada.

```rust
// our helper
impl Calculator {
    fn clear_display(&mut self) {
        self.display_value = "0".to_string()
    }
}

// our callback
cx.render(rsx!{
    button {
        onclick: move |_| state.write().clear_display(),
    }
})
```
