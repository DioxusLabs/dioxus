# Roteador

Em muitos de seus aplicativos, você desejará ter diferentes "cenas". Para uma página da Web, essas cenas podem ser as diferentes páginas da Web com seu próprio conteúdo.

Você pode escrever sua própria solução de gerenciamento de cena também. No entanto, para poupar o seu esforço, o Dioxus oferece suporte a uma solução de primeira linha para gerenciamento de cena chamada Dioxus Router.

## O que é isso?

Para um aplicativo como a página de destino do Dioxus (https://dioxuslabs.com), queremos ter páginas diferentes. Um esboço rápido de um aplicativo seria algo como:

- Pagina inicial
- Blogue
- Exemplo de vitrine

Cada uma dessas cenas é independente – não queremos renderizar a página inicial e o blog ao mesmo tempo.

É aqui que a `crates` do roteador são úteis. Para ter certeza de que estamos usando o roteador, basta adicionar o recurso `router` à sua dependência do Dioxus.

```toml
[dependencies]
dioxus = { version = "*" }
dioxus-router = { version = "*" }
```

## Usando o Roteador

Ao contrário de outros roteadores no ecossistema Rust, nosso roteador é construído de forma declarativa. Isso torna possível compor o layout do nosso aplicativo simplesmente organizando os componentes.

```rust
rsx!{
    Router {
        Route { to: "/home", Home {} }
        Route { to: "/blog", Blog {} }
    }
}
```

Sempre que visitamos este aplicativo, obteremos o componente Home ou o componente Blog renderizado, dependendo de qual rota entrarmos. Se nenhuma dessas rotas corresponder à localização atual, nada será renderizado.

Podemos corrigir isso de duas maneiras:

- Uma página 404 de _fallback_

```rust
rsx!{
    Router {
        Route { to: "/home", Home {} }
        Route { to: "/blog", Blog {} }
        Route { to: "", NotFound {} }
    }
}
```

- Redirecionar 404 para _Home_

```rust
rsx!{
    Router {
        Route { to: "/home", Home {} }
        Route { to: "/blog", Blog {} }
        Redirect { from: "", to: "/home" }
    }
}
```

## Links

Para que nosso aplicativo navegue nessas rotas, podemos fornecer elementos clicáveis chamados Links. Eles simplesmente envolvem elementos `<a>` que, quando clicados, navegam no aplicativo para o local determinado.

```rust
rsx!{
    Link {
        to: "/home",
        "Go home!"
    }
}
```

## Mais leitura

Esta página é apenas uma breve visão geral do roteador para mostrar que existe uma solução poderosa já construída para um problema muito comum. Para obter mais informações sobre o roteador, confira seu livro ou confira alguns dos exemplos.

O roteador tem sua própria documentação! [Disponível aqui](https://dioxuslabs.com/router/guide/).
