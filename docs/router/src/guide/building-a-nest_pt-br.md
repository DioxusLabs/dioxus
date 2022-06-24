# Construindo um Ninho

Não é um ninho de pássaro! Um ninho de rotas!

Neste capítulo, começaremos a construir a parte do blog do nosso site, que incluirá links, URLs aninhados e parâmetros de URL. Também exploraremos o caso de uso de componentes de renderização fora das rotas.

### Navegação do site

Os visitantes do nosso site não conhecerão todas as páginas e blogs disponíveis em nosso site, portanto, devemos fornecer uma barra de navegação para eles.
Vamos criar um novo componente `navbar`:

```rs
fn navbar(cx: Scope) -> Element {
    cx.render(rsx! {
        ul {

        }
    })
}
```

Nossa barra de navegação será uma lista de links entre nossas páginas. Poderíamos sempre usar um elemento de âncora do HTML, mas isso faria com que nossa página fosse recarregada desnecessariamente. Em vez disso, queremos usar o componente `Link` fornecido pelo Dioxus `Router`.

O componente `Link` é muito semelhante ao componente `Route`. É preciso um caminho e um elemento. Adicione o componente `Link` em sua instrução de uso e adicione alguns links:

```rs
use dioxus::{
    prelude::*,
    router::{Route, Router, Link}, // UPDATED
};

...

fn navbar(cx: Scope) -> Element {
    cx.render(rsx! {
        ul {
            // NEW
            Link { to: "/", "Home"}
            br {}
            Link { to: "/blog", "Blog"}
        }
    })
}
```

> Por padrão, o componente `Link` só funciona para links dentro de seu aplicativo. Para vincular a sites externos, adicione a propriedade `external: true`.
>
> ```rs
> Link { to: "https://github.com", externo: true, "GitHub"}
> ```

E, finalmente, use o componente `navbar` no componente do seu aplicativo:

```rs
fn app(cx: Scope) -> Element {
    cx.render(rsx! {
        Router {
            p { "-- Dioxus Blog --" }
            self::navbar {} // NEW
            Route { to: "/", self::homepage {}}
            Route { to: "", self::page_not_found {}}
        }
    })
}
```

Agora você deve ver uma lista de links perto do topo da sua página. Clique em um e você deve navegar perfeitamente entre as páginas.

##### WIP: estilo de link ativo

### Parâmetros de URL e rotas aninhadas

Muitos sites, como o GitHub, colocam parâmetros em sua URL. Por exemplo, `github.com/DioxusLabs` utiliza o texto após o domínio para pesquisar e exibir dinamicamente conteúdo sobre uma organização.

Queremos armazenar nossos blogs em um banco de dados e carregá-los conforme necessário. Isso ajudará a evitar que nosso aplicativo fique inchado, proporcionando tempos de carregamento mais rápidos. Também queremos que nossos usuários possam enviar às pessoas um link para uma postagem de blog específica.
Poderíamos utilizar uma página de pesquisa que carrega um blog quando clicado, mas nossos usuários não poderão compartilhar nossos blogs facilmente. É aqui que entram os parâmetros de URL. E, finalmente, também queremos que nosso site informe aos usuários que eles estão em uma página de blog sempre que a URL começar com `/blog`.

O caminho para o nosso blog será parecido com `/blog/myBlogPage`. `myBlogPage` sendo o parâmetro de URL.
Dioxus `Router` usa o padrão `:name` para que nossa rota se pareça com `/blog/:post`.

Primeiro, vamos dizer aos usuários quando eles estão em uma página de blog. Adicione uma nova rota em seu componente de aplicativo.

```rs
fn app(cx: Scope) -> Element {
    cx.render(rsx! {
        Router {
            p { "-- Dioxus Blog --" }
            self::navbar {}
            Route { to: "/", self::homepage {}}
            // NEW
            Route {
                to: "/blog",
            }
            Route { to: "", self::page_not_found {}}
        }
    })
}
```

As rotas podem receber componentes como parâmetros e sabemos que uma rota é um componente. Aninhamos rotas fazendo exatamente como elas são chamadas, aninhando-as:

```rs
fn app(cx: Scope) -> Element {
    cx.render(rsx! {
        Router {
            p { "-- Dioxus Blog --" }
            self::navbar {}
            Route { to: "/", self::homepage {}}
            Route {
                to: "/blog",
                Route { to: "/:post", "This is my blog post!" } // NEW
            }
            Route { to: "", self::page_not_found {}}
        }
    })
}
```

Aninhar nossa rota dessa forma não é muito útil no início, mas lembre-se de que queremos informar aos usuários que eles estão em uma página de blog. Vamos mover nosso `p { "-- Dioxus Blog --" ` para dentro de nossa rota `/blog`.

```rs
fn app(cx: Scope) -> Element {
    cx.render(rsx! {
        Router {
            self::navbar {}
            Route { to: "/", self::homepage {}}
            Route {
                to: "/blog",
                p { "-- Dioxus Blog --" } // MOVED
                Route { to: "/:post", "This is my blog post!" }
            }
            Route { to: "", self::page_not_found {}}
        }
    })
}
```

Agora nosso texto `-- Dioxus Blog --` será exibido sempre que um usuário estiver em um caminho que comece com `/blog`. A exibição de conteúdo de forma independente de página é útil ao criar menus de navegação, rodapés e similares.

Tudo o que resta é manipular nosso parâmetro de URL. Começaremos criando uma função `get_blog_post`. Em um site real, essa função chamaria um endpoint de API para obter uma postagem de blog do banco de dados. No entanto, isso está fora do escopo deste guia, portanto, utilizaremos texto estático.

```rs
fn get_blog_post(id: &str) -> String {
    match id {
        "foo" => "Welcome to the foo blog post!".to_string(),
        "bar" => "This is the bar blog post!".to_string(),
        id => format!("Blog post '{id}' does not exist!")
    }
}

```

Now that we have established our helper function, lets create a new `blog_post` component.

```rs
fn blog_post(cx: Scope) -> Element {
    let blog_text = "";

    cx.render(rsx! {
        p { "{blog_text}" }
    })
}
```

Tudo o que resta é extrair o `id` do blog da URL e chamar nossa função auxiliar para obter o texto do blog. Para fazer isso, precisamos utilizar o hook `use_route` do Dioxus Router.
Primeiro comece adicionando `use_route` às suas importações e então utilize o hook em seu componente `blog_post`.

```rs
use dioxus::{
    prelude::*,
    router::{use_route, Link, Route, Router}, // UPDATED
};

...

fn blog_post(cx: Scope) -> Element {
    let route = use_route(&cx); // NEW
    let blog_text = "";

    cx.render(rsx! {
        p { "{blog_text}" }
    })
}
```

O Dioxus Router fornece métodos integrados para extrair informações de uma rota. Poderíamos utilizar o método `segments`, `nth_segment` ou `last_segment` para o nosso caso, mas usaremos o método `segment` que extrai um parâmetro de URL específico.
O método `segment` também analisa o parâmetro em qualquer tipo para nós. Usaremos uma expressão `match` que lida com a análise erros e, em caso de sucesso, usa nossa função auxiliar para obter a postagem do blog.

```rs
fn blog_post(cx: Scope) -> Element {
    let route = use_route(&cx);

    // NEW
    let blog_text = match route.segment::<String>("post").unwrap() {
        Ok(val) => get_blog_post(&val),
        Err(_) => "An unknown error occured".to_string(),
    };

    cx.render(rsx! {
        p { "{blog_text}" }
    })
}
```

E finalmente adicione o componente `blog_post` ao seu componente `app`:

```rs
fn app(cx: Scope) -> Element {
    cx.render(rsx! {
        Router {
            self::navbar {}
            Route { to: "/", self::homepage {}}
            Route {
                to: "/blog",
                p { "-- Dioxus Blog --" }
                Route { to: "/:post", self::blog_post {} } // UPDATED
            }
            Route { to: "", self::page_not_found {}}
        }
    })
}
```

É isso! Se você for para `/blog/foo` você deverá ver `Welcome to the foo blog post!`.

### Conclusão

Neste capítulo utilizamos a funcionalidade `Link`, URL Parameter e `use_route` do Dioxus `Router` para construir a parte do blog de nosso aplicativo. No próximo e último capítulo, veremos o componente `Redirect` para redirecionar usuários não autorizados para outra página.
