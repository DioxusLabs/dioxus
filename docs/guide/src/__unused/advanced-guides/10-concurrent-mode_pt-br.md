# Modo Simultâneo

O modo simultâneo providencia um mecanismo para construir componentes assíncronos eficientes. Com esta função, componentes não precisam renderizar imediatamente, eles podem ser agendados no futuro para retornando um tipo `Future`.

Para fazer um componente assíncrono, simplesmente mude a assinatura das funções para async.

Por exemplo, a função:

```rust
fn Example(cx: Scope) -> Vnode {
    rsx!{ <div> "Hello world!" </div> }
}
```

Se torna:

```rust
async fn Example(cx: Scope) -> Vnode {
    rsx!{ <div> "Hello world!" </div> }
}
```

Agora, a lógica de dentro dos componentes podem ser adiadas com `await` adiando a atualização dos componentes e seus filhos desta forma:

```rust
async fn Example(cx: Scope) -> Vnode {
    let name = fetch_name().await;
    rsx!{ <div> "Hello {name}" </div> }
}

async fetch_name() -> String {
    // ...
}
```

Este componente irá ser agendado para renderização apenas quando todas as suas declarações finalizarem. No entanto, nós _não_ recomendamos usar `async/await` diretamente no seus componentes.

`Async` é notoriamente desafiador ainda que recompensador por suas ferramentas eficientes. Se tiver cuidado, o aspecto de bloqueio/desbloqueio do contexto dos componentes podem levar a uma concorrência de dados e por fim um `panic!`.

Se um recurso estiver bloqueando um componente enquanto ele estiver esperando, então outros componentes pode ser bloqueados ou causarem `panic!` por tentar acessar o mesmo recurso.

Estas regras são especialmente importantes quando referências de um estado compartilhado são acessadas usando o contexto de tempo-de-vida (`lifetime`) do objeto.

Se houver referências mutáveis para o recurso imutável capturado dentro do contexto, então o componente irá sofrer um `panic!`, levando à uma confusão.

Ao contrário disso, nós sugerimos usar Hooks e combinadores `Future` que podem ser utilizados de forma segura para salvaguardar o `Context` do componente enquanto interage com tarefas assíncronas.

Como parte do nosso Dioxus Hooks, nós providenciamos o Hook carregador de dados que pausa um componente até todas as suas dependências estarem prontas. Isso armazena requisições na memória, re-executa a lista de dependências que mudaram e providencia a opção de renderizar algo enquanto o componente assíncrono está sendo carregado.

```rust
async fn ExampleLoader(cx: Scope) -> Vnode {
    /*
    Fetch, pause the component from rendering at all.

    The component is locked while waiting for the request to complete
    While waiting, an alternate component is scheduled in its place.

    This API stores the result on the Context object, so the loaded data is taken as reference.
    */
    let name: &Result<SomeStructure> = use_fetch_data("http://example.com/json", ())
                                        .place_holder(|cx| rsx!{<div> "loading..." </div>})
                                        .delayed_place_holder(1000, |cx| rsx!{ <div> "still loading..." </div>})
                                        .await;

    match name {
        Ok(name) => rsx! { <div> "Hello {something}" </div> },
        Err(e) => rsx! { <div> "An error occurred :(" </div>}
    }
}
```

```rust
async fn Example(cx: Scope) -> DomTree {
    // Diff this set between the last set
    // Check if we have any outstanding tasks?
    //
    // Eventually, render the component into the VDOM when the future completes
    <div>
        <Example />
    </div>

    // Render a div, queue a component
    // Render the placeholder first, then when the component is ready, then render the component
    <div>
        <Suspense placeholder={html!{<div>"Loading"</div>}}>
            <Example />
        </Suspense>
    </div>
}
```
