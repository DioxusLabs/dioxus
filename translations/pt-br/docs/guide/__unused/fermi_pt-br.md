# Fermi

Depois de ter coberto o estado local e global, você está definitivamente pronto para começar a criar alguns aplicativos Dioxus complexos. Antes de ir muito longe, confira a `crate` `Fermi`. O `Fermi` facilita o gerenciamento do estado global e é dimensionado para o maior dos aplicativos.

## Para que serve o Fermi?

Se você estiver criando um aplicativo que ultrapassou alguns componentes pequenos, vale a pena esboçar e organizar o estado do seu aplicativo. O `Fermi` facilita a transição de um aplicativo simples que depende de `use_state` para um aplicativo com dezenas de componentes.

> Simplificando - Fermi é a caixa ideal para gerenciar o estado global em seu aplicativo Dioxus.

## Como eu uso isso?

Para adicionar `Fermi` ao seu projeto, basta adicionar o recurso "fermi" à sua dependência do Dioxus.

```toml
[dependencies]
dioxus = { version = "0.2", features = ["desktop", "fermi"] }
```

Fermi is built on the concept of "atoms" of global state. Instead of bundling all our state together in a single mega struct, we actually chose to implement it piece-by-piece with functions.

Each piece of global state in your app is represented by an "atom."

```rust
static TITLE: Atom<String> = |_| "Defualt Title".to_string();
```

Este átomo pode ser usado com o hook `use_atom` como um substituto para `use_state`.

```rust
static TITLE: Atom<String> = |_| "Defualt Title".to_string();

fn Title(cx: Scope) -> Element {
    let title = use_atom(&cx, TITLE);

    cx.render(rsx!{
        button { onclick: move |_| title.set("new title".to_string()) }
    })
}
```

No entanto, `Fermi` realmente se torna útil quando queremos compartilhar o valor entre dois componentes diferentes.

```rust
static TITLE: Atom<String> = |_| "Defualt Title".to_string();

fn TitleBar(cx: Scope) -> Element {
    let title = use_atom(&cx, TITLE);

    rsx!{cx, button { onclick: move |_| title.set("titlebar title".to_string()) } }
}

fn TitleCard(cx: Scope) -> Element {
    let title = use_atom(&cx, TITLE);

    rsx!{cx, button { onclick: move |_| title.set("title card".to_string()) } }
}
```

Esses dois componentes podem obter e definir o mesmo valor!

## Uso em Coleções

`Fermi` fica _realmente_ poderoso quando usado para gerenciar coleções de dados. Nos bastidores, o `Fermi` usa coleções imutáveis e rastreia leituras e gravações de chaves individuais. Isso facilita a implementação de coisas como listas e postagens infinitas com pouca perda de desempenho. Também torna muito fácil refatorar nosso aplicativo e adicionar novos campos.

```rust
static NAMES: AtomRef<Uuid, String> = |builder| {};
static CHECKED: AtomRef<Uuid, bool> = |builder| {};
static CREATED: AtomRef<Uuid, Instant> = |builder| {};
```

To use these collections:

```rust
#[inline_props]
fn Todo(cx: Scope, id: Uuid) -> Element {
    let name = use_atom(&cx, NAMES.select(id));
    let checked = use_atom(&cx, CHECKED.select(id));
    let created = use_atom(&cx, CREATED.select(id));

    // or

    let (name, checked, created) = use_atom(&cx, (NAMES, CHECKED, CREATED).select(id));
}
```

Esse padrão específico pode parecer estranho à primeira vista - "por que todo o nosso estado não está sob uma estrutura?" - mas eventualmente mostra seu valor ao lidar com grandes quantidades de dados. Ao compor coleções juntos, podemos obter as vantagens da arquitetura `Entity-Component-System` (`ECS`) em nossos aplicativos Dioxus. O desempenho é bastante previsível aqui e fácil de rastrear.

## AtomRef

Assim como `use_ref` pode ser usado para gerenciar estados complexos localmente, `AtomRef` pode ser usado para gerenciar estados globais complexos. `AtomRef` é basicamente um `Rc<RefCell<T>>` global com rastreamento de mutação.

Ele também serve como um substituto básico para `use_ref`:

```rust
fn Todo(cx: Scope) -> Element {
    let cfg = use_atom_ref(&cx, CFG);

    cfg.write().enable_option();
}

```

## Leitura futura

Este guia destina-se apenas a ser uma visão geral do `Fermi`. Esta página é apenas uma pequena demonstração do que consideramos a melhor solução de gerenciamento de estado disponível hoje para aplicativos Dioxus.

Para ler mais, confira a `crate` [Fermi](https://github.com/DioxusLabs/dioxus/tree/master/packages/fermi)!
