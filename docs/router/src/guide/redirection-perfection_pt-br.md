# Perfeição de redirecionamento

Você está no caminho certo para se tornar um mestre de roteamento!

Neste capítulo, abordaremos a utilização do componente `Redirect` para que você possa levar seu [Rickrolling](https://en.wikipedia.org/wiki/Rickrolling) para o próximo nível. Também forneceremos alguns desafios opcionais no final se você quiser continuar sua prática não apenas com o Dioxus `Router`, mas com o Dioxus em geral.

### O que é essa coisa de redirecionamento?

O componente `Redirect` é simples! Quando o Dioxus determinar que ele deve ser renderizado, ele redirecionará o visitante do seu aplicativo para onde você quiser.
Neste exemplo, digamos que você adicionou uma página secreta ao seu site, mas não teve tempo de programar no sistema de permissão. Como uma solução rápida, você adiciona um redirecionamento.

Como sempre, vamos primeiro criar um novo componente chamado `secret_page`.

```rs
fn secret_page(cx: Scope) -> Element {
    cx.render(rsx! {
        p { "This page is not to be viewed!" }
    })
}
```

Para redirecionar nossos visitantes, tudo o que temos que fazer é renderizar o componente `Redirect`. O componente `Redirect` é muito semelhante ao componente `Link`. A principal diferença é que não exibe nada de novo.
Primeiro importe o componente `Redirect` e então atualize seu componente `secret_page`:

```rs
use dioxus::{
    prelude::*,
    router::{use_route, Link, Redirect, Route, Router}, // UPDATED
};

...

fn secret_page(cx: Scope) -> Element {
    cx.render(rsx! {
        p { "This page is not to be viewed!" }
        Redirect { to: "/" } // NEW
    })
}
```

É isso! Agora seus usuários serão redirecionados para fora da página secreta.

> Semelhante ao componente `Link`, o componente `Redirect` precisa ser definido explicitamente para redirecionar para um site externo. Para vincular a sites externos, adicione a propriedade `external: true`.
>
> ```rs
> Redirecionar { para: "https://github.com", externo: true}
> ```

### Conclusão

Ótimo! Você concluiu o guia do roteador Dioxus. Você construiu um pequeno aplicativo e aprendeu sobre as muitas coisas que pode fazer com o Dioxus `Router`. Para continuar sua jornada, você pode encontrar uma lista de desafios abaixo ou conferir a [referência](../reference/index.md).

### Desafios

- Organize seus componentes em arquivos separados para melhor manutenção.
- Dê um pouco de estilo ao seu aplicativo, se ainda não o fez.
- Construa uma página sobre para que seus visitantes saibam quem você é.
- Adicione um sistema de usuário que usa parâmetros de URL.
- Crie um sistema de administração simples para criar, excluir e editar blogs.
- Se você quiser ir ao máximo, conecte seu aplicativo a uma API REST e a um banco de dados.
