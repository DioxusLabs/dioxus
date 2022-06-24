# Criando nossa primeira rota

Neste capítulo, continuaremos com nosso novo projeto Dioxus para criar uma página inicial e começar a utilizar o Dioxus `Router`!

### Fundamentos

Dioxus `Router` funciona baseado em um roteador e componente de rota. Se você já usou o [React Router](https://reactrouter.com/), deve se sentir em casa com o Dioxus `Router`.

Para começar, importe os componentes `Router` e `Route`.

```rs
use dioxus::{
    prelude::*,
    router::{Route, Router}
}
```

Também precisamos de uma página real para direcionar! Adicione um componente de página inicial:

```rs
fn homepage(cx: Scope) -> Element {
    cx.render(rsx! {
        p { "Welcome to Dioxus Blog!" }
    })
}
```

### Rotear ou não rotear

Queremos usar o Dioxus `Router` para separar nosso aplicativo em diferentes "páginas". O Dioxus `Router` determinará então qual página renderizar com base no caminho da URL.

Para começar a usar o Dioxus `Router`, precisamos usar o componente `Router`.
Substitua o `p { "Olá, wasm!" }` em seu componente `app` com um componente `Router`:

```rs
fn app(cx: Scope) -> Element {
    cx.render(rsx! {
        Router {} // NEW
    })
}
```

Agora estabelecemos um roteador e podemos criar nossa primeira rota. Estaremos criando uma rota para nosso componente de página inicial que criamos anteriormente.

```rs
fn app(cx: Scope) -> Element {
    cx.render(rsx! {
        Router {
            Route { to: "/", self::homepage {}} // NEW
        }
    })
}
```

Se você for para a aba do navegador do seu aplicativo, você deverá ver o texto `Welcome to Dioxus Blog!` quando estiver na URL raiz (`http://localhost:8080/`). Se você inserir um caminho diferente para a URL, nada deverá ser exibido.

Isso ocorre porque dissemos ao Dioxus `Router` para renderizar o componente `homepage` somente quando o caminho da URL for `/`. Você pode dizer ao Dioxus `Router` para renderizar qualquer tipo de componente, como um `div {}`.

### E se uma rota não existir?

Em nosso exemplo, o Dioxus `Router` não renderiza nada. Se quiséssemos, poderíamos dizer ao Dioxus `Router` para renderizar um componente o tempo todo! Experimente:

```rs
fn app(cx: Scope) -> Element {
    cx.render(rsx! {
        Router {
            p { "-- Dioxus Blog --" } // NEW
            Route { to: "/", self::homepage {}}
        }
    })
}
```

Entraremos em mais detalhes sobre isso no próximo capítulo.

Muitos sites também têm uma página "404" para quando um caminho de URL não leva a lugar nenhum. Dioxus `Router` pode fazer isso também! Crie um novo componente `page_not_found`.

```rs
fn page_not_found(cx: Scope) -> Element {
    cx.render(rsx! {
        p { "Oops! The page you are looking for doesn't exist!" }
    })
}
```

Agora, diga ao Dioxus `Router` para renderizar nosso novo componente quando não houver rota. Crie uma nova rota com um caminho de não encontrado:

```rs
fn app(cx: Scope) -> Element {
    cx.render(rsx! {
        Router {
            p { "-- Dioxus Blog --" }
            Route { to: "/", self::homepage {}}
            Route { to: "", self::page_not_found {}} // NEW
        }
    })
}
```

Agora, quando você for para uma rota que não existe, deverá ver o texto da página não encontrada e o texto que dissemos ao Dioxus `Router` para renderizar o tempo todo.

```
// localhost:8080/abc

-- Dioxus Blog --
Oops! The page you are looking for doesn't exist!
```

> Certifique-se de colocar sua rota vazia na parte inferior ou ela substituirá todas as rotas abaixo dela!

### Conclusão

Neste capítulo aprendemos como criar uma rota e dizer ao Dioxus `Router` qual componente renderizar quando o caminho da URL for igual ao que especificamos. Também criamos uma página 404 para tratar quando uma rota não existe. Em seguida, criaremos a parte do blog do nosso site. Utilizaremos rotas aninhadas e parâmetros de URL.
