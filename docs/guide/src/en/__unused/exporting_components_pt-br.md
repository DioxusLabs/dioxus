# Reutilizando, Importando e Exportando Componentes

À medida que seu aplicativo cresce em tamanho, você precisará começar a dividir sua interface do usuário em componentes e, eventualmente, em arquivos diferentes. Essa é uma ótima ideia para encapsular a funcionalidade de sua interface do usuário e dimensionar sua equipe.

Neste capítulo abordaremos:

- Módulos de Rust
- Componentes Pub/Privado
- Estrutura para grandes componentes

## Decompondo

Digamos que nosso aplicativo seja algo assim:

```shell
├── Cargo.toml
└── src
    └── main.rs
```

```rust
// main.rs
use dioxus::prelude::*;

fn main() {
    dioxus::desktop::launch(App);
}

fn App(Scope) -> Element {}

#[derive(PartialEq, Props)]
struct PostProps{}
fn Post(Scope<PostProps>) -> Element {}

#[derive(PartialEq, Props)]
struct VoteButtonsProps {}
fn VoteButtons(Scope<VoteButtonsProps>) -> Element {}

#[derive(PartialEq, Props)]
struct TitleCardProps {}
fn TitleCard(Scope<TitleCardProps>) -> Element {}

#[derive(PartialEq, Props)]
struct MetaCardProps {}
fn MetaCard(Scope<MetaCardProps>) -> Element {}

#[derive(PartialEq, Props)]
struct ActionCardProps {}
fn ActionCard(Scope<ActionCardProps>) -> Element {}
```

São muitos componentes para um arquivo! Refatoramos com sucesso nosso aplicativo em componentes, mas provavelmente deveríamos começar a dividi-lo em um arquivo para cada componente.

## Quebrando em Arquivos Diferentes

Felizmente, Rust tem um sistema de módulos embutido que é muito mais limpo do que o que você pode estar acostumado em JavaScript. Como `VoteButtons`, `TitleCard`, `MetaCard` e `ActionCard` pertencem ao componente `Post`, vamos colocá-los todos em uma pasta chamada "post". Faremos um arquivo para cada componente e moveremos os adereços e a função de renderização.

```rust
// src/post/action.rs

use dioxus::prelude::*;

#[derive(PartialEq, Props)]
struct ActionCardProps {}
fn ActionCard(Scope<ActionCardProps>) -> Element {}
```

Devemos também criar um arquivo `mod.rs` na pasta `post` para que possamos usá-lo em nosso `main.rs`. Nosso componente `Post` e seus adereços irão para este arquivo.

```rust
// src/post/mod.rs

use dioxus::prelude::*;

#[derive(PartialEq, Props)]
struct PostProps {}
fn Post(Scope<PostProps>) -> Element {}
```

```shell
├── Cargo.toml
└── src
    ├── main.rs
    └── post
        ├── vote.rs
        ├── title.rs
        ├── meta.rs
        ├── action.rs
        └── mod.rs
```

Em nosso `main.rs`, vamos declarar o módulo `post` para que possamos acessar nosso componente `Post`.

```rust
// main.rs
use dioxus::prelude::*;

fn main() {
    dioxus::desktop::launch(App);
}

mod post;

fn App(Scope) -> Element {
    cx.render(rsx!{
        post::Post {
            id: Uuid::new_v4(),
            score: 10,
            comment_count: 10,
            post_time: std::Instant::now(),
            url: "example".to_string(),
            title: "Title".to_string(),
            original_poster: "me".to_string()
        }
    })
}
```

Se você tentar criar este aplicativo agora, receberá uma mensagem de erro dizendo que 'A postagem é privada, tente alterá-la para pública'. Isso ocorre porque não exportamos corretamente nosso componente! Para corrigir isso, precisamos ter certeza de que `Props` e `Component` são declarados como "públicos":

```rust
// src/post/mod.rs

use dioxus::prelude::*;

#[derive(PartialEq, Props)]
pub struct PostProps {}
pub fn Post(Scope<PostProps>) -> Element {}
```

Enquanto estamos aqui, também precisamos garantir que cada um de nossos subcomponentes seja incluído como módulos e exportado.

Nosso arquivo "post/mod.rs" ficará assim:

```rust
use dioxus::prelude::*;

mod vote;
mod title;
mod meta;
mod action;

#[derive(Props, PartialEq)]
pub struct PostProps {
    id: uuid::Uuid,
    score: i32,
    comment_count: u32,
    post_time: std::time::Instant,
    url: String,
    title: String,
    original_poster: String
}

pub fn Post(Scope<PostProps>) -> Element {
    cx.render(rsx!{
        div { class: "post-container"
            vote::VoteButtons {
                score: props.score,
            }
            title::TitleCard {
                title: props.title,
                url: props.url,
            }
            meta::MetaCard {
                original_poster: props.original_poster,
                post_time: props.post_time,
            }
            action::ActionCard {
                post_id: props.id
            }
        }
    })
}
```

Em última análise, a inclusão e exportação de componentes é regida pelo sistema de módulos d Rust. [O livro Rust é um ótimo recurso para aprender sobre esses conceitos com mais detalhes](https://doc.rust-lang.org/book/ch07-00-managing-growing-projects-with-packages-crates-and-modules.html)

## Estrutura Final:

```shell
├── Cargo.toml
└── src
    ├── main.rs
    └── post
        ├── vote.rs
        ├── title.rs
        ├── meta.rs
        ├── action.rs
        └── mod.rs
```

```rust
// main.rs:
use dioxus::prelude::*;

fn main() {
    dioxus::desktop::launch(App);
}

mod post;

fn App(Scope) -> Element {
    cx.render(rsx!{
        post::Post {
            id: Uuid::new_v4(),
            score: 10,
            comment_count: 10,
            post_time: std::Instant::now(),
            url: "example".to_string(),
            title: "Title".to_string(),
            original_poster: "me".to_string()
        }
    })
}
```

```rust
// src/post/mod.rs
use dioxus::prelude::*;

mod vote;
mod title;
mod meta;
mod action;

#[derive(Props, PartialEq)]
pub struct PostProps {
    id: uuid::Uuid,
    score: i32,
    comment_count: u32,
    post_time: std::time::Instant,
    url: String,
    title: String,
    original_poster: String
}

pub fn Post(Scope<PostProps>) -> Element {
    cx.render(rsx!{
        div { class: "post-container"
            vote::VoteButtons {
                score: props.score,
            }
            title::TitleCard {
                title: props.title,
                url: props.url,
            }
            meta::MetaCard {
                original_poster: props.original_poster,
                post_time: props.post_time,
            }
            action::ActionCard {
                post_id: props.id
            }
        }
    })
}
```

```rust
// src/post/vote.rs
use dioxus::prelude::*;

#[derive(PartialEq, Props)]
pub struct VoteButtonsProps {}
pub fn VoteButtons(Scope<VoteButtonsProps>) -> Element {}
```

```rust
// src/post/title.rs
use dioxus::prelude::*;

#[derive(PartialEq, Props)]
pub struct TitleCardProps {}
pub fn TitleCard(Scope<TitleCardProps>) -> Element {}
```

```rust
// src/post/meta.rs
use dioxus::prelude::*;

#[derive(PartialEq, Props)]
pub struct MetaCardProps {}
pub fn MetaCard(Scope<MetaCardProps>) -> Element {}
```

```rust
// src/post/action.rs
use dioxus::prelude::*;

#[derive(PartialEq, Props)]
pub struct ActionCardProps {}
pub fn ActionCard(Scope<ActionCardProps>) -> Element {}
```
