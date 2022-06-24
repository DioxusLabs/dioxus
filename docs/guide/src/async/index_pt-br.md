# Trabalhando com Async

Nem todos os aplicativos que você criar podem ser desenvolvidos com código síncrono. Muitas vezes, você precisará interagir com sistemas de arquivos, interfaces de rede, hardware ou temporizadores.

Até agora, falamos apenas sobre a criação de aplicativos com código síncrono, portanto, este capítulo se concentrará na integração de código assíncrono em seu aplicativo.

## A Execução (Runtime)

Por padrão, o Dioxus-Desktop vem com o runtime `Tokio` e configura tudo automaticamente para você. No momento, isso não é configurável, embora seja fácil escrever uma integração para o Desktop que use um tempo de execução assíncrono diferente.

## Send/Sync

Escrever aplicativos que lidam com `Send`/`Sync` pode ser frustrante às vezes. Nos bastidores, o Dioxus não é atualmente thread-safe, então qualquer código assíncrono que você escreve _não_ precisa ser `Send`/`Sync`. Isso significa que você pode usar `structs` não thread-safe como `Cell`, `Rc` e `RefCell`.

Todo código assíncrono em seu aplicativo é pesquisado em um `LocalSet`, então você também pode usar `tokio::spawn_local`.

## Gerando um Future

Atualmente, todos os `Futures` em Dioxus devem ser `'static`. Para gerar um `Future`, simplesmente chame `cx.spawn()`.

```rust
rsx!{
    button {
        onclick: move |_| cx.spawn(async move {
            let result = fetch_thing().await;
        })
    }
}
```

Chamar `spawn` lhe dará um `JoinHandle` que permite cancelar ou pausar o `Future`.

## Configurando o estado de dentro de um Future

Se você começar a escrever algum código assíncrono, será rapidamente recebido com o erro infame sobre `borrows`, `static closures` e `lifetimes`.

```
error[E0759]: `cx` has an anonymous lifetime `'_` but it needs to satisfy a `'static` lifetime requirement
  --> examples/tasks.rs:13:27
   |
12 | fn app(cx: Scope) -> Element {
   |            ----- this data with an anonymous lifetime `'_`...
13 |     let count = use_state(&cx, || 0);
   |                           ^^^ ...is used here...
14 |
15 |     use_future(&cx, (), move |_| {
   |     ---------- ...and is required to live as long as `'static` here
   |
note: `'static` lifetime requirement introduced by this bound
  --> /Users/jonkelley/Development/dioxus/packages/hooks/src/usefuture.rs:25:29
   |
25 |     F: Future<Output = T> + 'static,
   |                             ^^^^^^^

For more information about this error, try `rustc --explain E0759`.
error: could not compile `dioxus` due to previous error
```

`Rustc` tende a fornecer um ótimo feedback em seus erros, mas esse erro em particular é bastante inútil. Para referência, aqui está nosso código:

```rust
fn app(cx: Scope) -> Element {
    let count = use_state(&cx, || 0);

    cx.spawn(async move {
        tokio::time::sleep(Duration::from_millis(1000)).await;
        count += 1;
    });

    cx.render(rsx! {
        button {
            onclick: move |_| count.set(0),
            "Reset the count"
        }
    })
}
```

Simples, certo? Geramos um `Future` que atualiza o valor após um segundo. Bem, sim e não. Nosso valor `count` é válido apenas para o tempo de vida deste componente, mas nosso futuro ainda pode estar em execução mesmo após a renderização do componente.

Por padrão, o Dioxus exige que todos os `Futures` sejam `'static`, o que significa que eles não podem simplesmente emprestar o estado de nossos Hooks diretamente.

Para contornar isso, precisamos obter um identificador `'static` para o nosso estado. Todos os hooks do Dioxus implementam `Clone`, então você só precisa utilizar `clone()` antes de gerar seu `Future`. Sempre que você receber o erro específico acima, certifique-se de utilizar `Clone` ou `ToOwned`.

```rust
cx.spawn({
    let mut count = count.clone();
    async move {
        tokio::time::sleep(Duration::from_millis(1000)).await;
        count += 1;
    }
});
```

Para tornar isso um pouco mais fácil, o Dioxus exporta a macro `to_owned!` que criará uma ligação como mostrado acima, o que pode ser bastante útil ao lidar com muitos valores.

```rust
cx.spawn({
    to_owned![count, age, name, description, etc];
    async move {
        tokio::time::sleep(Duration::from_millis(1000)).await;
        count += 1;
    }
});
```
