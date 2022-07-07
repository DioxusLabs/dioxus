# Inscrições

Inscrições Dioxus são usadas para agendar componentes no futuro. O objeto `Context` pode criar inscrições:

```rust
fn Component(cx: Component) -> DomTree {
    let update = cx.schedule();

    // Now, when the subscription is called, the component will be re-evaluated
    update.consume();
}
```

Toda vez que o um componente é inscrito, ele irá então ser re-avaliado.

Você pode considerar a entrada de propriedades do componente sendo apenas outro formulário de inscrição.

Por padrão, o Dioxus fará um `diff` no sistema de propriedades dos componentes automaticamente quando a função pai (acima na hierarquia) for chamada. Se as propriedades forem diferentes, então o componente filho executa uma nova inscrição.

A API de inscrições expõe esta funcionalidade permitindo aos Hooks e soluções de gerenciamento de memória a habilidade de atualizarem seus componentes toda vez que algum estado ou evento ocorrer fora do componente.

Por exemplo, o `use_context` Hook usa isso para inscrever componentes que usem um contexto em particular.

```rust
fn use_context<I>(cx: Scope<T>) -> I {

}







```
