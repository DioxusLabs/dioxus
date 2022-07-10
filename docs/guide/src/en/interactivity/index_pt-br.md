# Adicionando interatividade

Até agora, aprendemos como descrever a estrutura e as propriedades de nossas interfaces de usuário. Infelizmente, eles são estáticos e bastante desinteressantes. Neste capítulo, vamos aprender como adicionar interatividade por meio de eventos, estado e tarefas.

## Cartilha sobre interatividade

Antes de nos aprofundarmos muito na mecânica da interatividade, devemos primeiro entender como a Dioxus escolhe exatamente lidar com a interação do usuário e as atualizações do seu aplicativo.

### O que é um estado?

Todo aplicativo que você construir tem algum tipo de informação que precisa ser renderizada na tela. Dioxus é responsável por traduzir sua interface de usuário desejada para o que é renderizado na tela. _Você_ é responsável por fornecer o conteúdo.

Os dados dinâmicos em sua interface de usuário são chamados de `State`.

Ao iniciar seu aplicativo pela primeira vez com `dioxus::web::launch_with_props`, você fornecerá o estado inicial. Você precisa declarar o estado inicial _antes_ de iniciar o aplicativo.

```rust
fn main() {
    // declare our initial state
    let props = PostProps {
        id: Uuid::new_v4(),
        score: 10,
        comment_count: 0,
        post_time: std::time::Instant::now(),
        url: String::from("dioxuslabs.com"),
        title: String::from("Hello, world"),
        original_poster: String::from("dioxus")
    };

    // start the render loop
    dioxus::desktop::launch_with_props(Post, props);
}
```

Quando o Dioxus renderizar seu aplicativo, ele passará uma referência imutável para `PostProps` em seu componente `Post`. Aqui, você pode passar o estado para as crianças.

```rust
fn App(cx: Scope<PostProps>) -> Element {
    cx.render(rsx!{
        Title { title: &cx.props.title }
        Score { score: &cx.props.score }
        // etc
    })
}
```

O estado em Dioxus segue um padrão chamado "fluxo de dados unidirecional". À medida que seus componentes criam novos componentes como seus filhos, a estrutura do seu aplicativo eventualmente crescerá em uma árvore onde o estado é transmitido do componente raiz para as "folhas" da árvore.

Você provavelmente já viu a árvore de componentes da interface do usuário representada usando um gráfico acíclico direcionado:

![imagem](../images/component_tree.png)

Com o Dioxus, seu estado sempre fluirá dos componentes pai para os componentes filho.

### Como faço para alterar o estado do meu aplicativo?

Já falamos sobre o fluxo de dados do estado, mas ainda não falamos sobre como alterar esse estado dinamicamente. O Dioxus oferece várias maneiras de alterar o estado do seu aplicativo enquanto ele está em execução.

Para começar, nós _poderíamos_ usar o método `update_root_props` no `VirtualDom` para fornecer um estado raiz totalmente novo do seu aplicativo. No entanto, para a maioria dos aplicativos, você provavelmente não deseja gerar novamente todo o aplicativo apenas para atualizar algum texto ou um sinalizador.

Em vez disso, você desejará armazenar o estado internamente em seus componentes e deixar _isso_ fluir pela árvore. Para armazenar o estado em seus componentes, você usará algo chamado `hook`. Hooks são funções especiais que reservam um `slot` de estado na memória do seu componente e fornecem alguma funcionalidade para atualizar esse estado.

O hook mais comum que você usará para armazenar o estado é `use_state`. `use_state` fornece um slot para alguns dados que permitem que você leia e atualize o valor sem alterá-lo acidentalmente.

```rust
fn App(cx: Scope)-> Element {
    let post = use_state(&cx, || {
        PostData {
            id: Uuid::new_v4(),
            score: 10,
            comment_count: 0,
            post_time: std::time::Instant::now(),
            url: String::from("dioxuslabs.com"),
            title: String::from("Hello, world"),
            original_poster: String::from("dioxus")
        }
    });

    cx.render(rsx!{
        Title { title: &post.title }
        Score { score: &post.score }
        // etc
    })
}
```

Sempre que temos um novo post que queremos renderizar, podemos chamar `set_post` e fornecer um novo valor:

```rust
set_post(PostData {
    id: Uuid::new_v4(),
    score: 20,
    comment_count: 0,
    post_time: std::time::Instant::now(),
    url: String::from("google.com"),
    title: String::from("goodbye, world"),
    original_poster: String::from("google")
})
```

Vamos nos aprofundar em como exatamente esses hooks funcionam mais tarde.

### Quando atualizo meu estado?

Existem algumas abordagens diferentes para escolher quando atualizar seu estado. Você pode atualizar seu estado em resposta a eventos acionados pelo usuário ou de forma assíncrona em alguma tarefa em segundo plano.

### Atualizando o estado nos ouvintes

Ao responder a eventos acionados pelo usuário, queremos "ouvir" um evento em algum elemento em nosso componente.

Por exemplo, digamos que fornecemos um botão para gerar uma nova postagem. Sempre que o usuário clica no botão, ele recebe uma nova postagem. Para alcançar esta funcionalidade, vamos querer anexar uma função ao método `onclick` no `button`. Sempre que o botão for clicado, nossa função será executada e obteremos novos dados de postagem para trabalhar.

```rust
fn App(cx: Scope)-> Element {
    let post = use_state(&cx, || PostData::new());

    cx.render(rsx!{
        button {
            onclick: move |_| post.set(PostData::random())
            "Generate a random post"
        }
        Post { props: &post }
    })
}
```

Vamos mergulhar muito mais fundo nos ouvintes de eventos mais tarde.

### Atualizando o estado de forma assíncrona

Também podemos atualizar nosso estado fora dos ouvintes de eventos com `futures` e `coroutines`.

- `Futures` são a versão de Rust das promessas que podem executar trabalho assíncrono por um sistema de pesquisa eficiente. Podemos enviar novos `Futures` para o Dioxus através de `push_future` que retorna um `TaskId` ou com `spawn`.
- `Coroutines` são blocos assíncronos de nosso componente que têm a capacidade de interagir de forma limpa com valores, hooks e outros dados no componente.

Como as corrotinas e os `Futures` permanecem entre as renderizações, os dados nelas devem ser válidos para o tempo de vida `'static`. Devemos declarar explicitamente em quais valores nossa tarefa dependerá para evitar o problema de `props obsoletos` comum no React.

Podemos usar tarefas em nossos componentes para construir um pequeno cronômetro que marca a cada segundo.

> Nota: O hook `use_future` iniciará nossa corrotina imediatamente. O hook `use_coroutine` fornece mais flexibilidade sobre iniciar e parar `Futures` em tempo real.

```rust
fn App(cx: Scope)-> Element {
    let elapsed = use_state(&cx, || 0);

    use_future(&cx, (), |()| {
        to_owned![elapsed]; // explicitly capture this hook for use in async
        async move {
            loop {
                gloo_timers::future::TimeoutFuture::new(1_000).await;
                elapsed.modify(|i| i + 1)
            }
        }
    });

    rsx!(cx, div { "Current stopwatch time: {elapsed}" })
}
```

Usar código assíncrono pode ser difícil! Nós apenas arranhamos a superfície do que é possível. Temos um capítulo inteiro sobre como usar código assíncrono corretamente em seus aplicativos no Dioxus.

### Como eu digo a Dioxus que meu estado mudou?

Sempre que você informar ao Dioxus que o componente precisa ser atualizado, ele irá "renderizar" seu componente novamente, armazenando os `Elements` anteriores e atuais na memória. O Dioxus descobrirá automaticamente as diferenças entre o antigo e o novo e gerará uma lista de edições que o renderizador precisa aplicar para alterar o que está na tela. Este processo é chamado de "diffing":

![Diffing](../images/diffing.png)

No React, as especificidades de quando um componente é renderizado novamente são pouco claras. Com o Dioxus, qualquer componente pode se marcar como "sujo" através de um método em `Context`: `needs_update`. Além disso, qualquer componente pode marcar qualquer outro componente _other_ como sujo, desde que conheça o ID do outro componente com `needs_update_any`.

Com esses blocos de construção, podemos criar novos hooks semelhantes ao `use_state` que nos permitem dizer facilmente ao Dioxus que novas informações estão prontas para serem enviadas para a tela.

### Como atualizo meu estado com eficiência?

Em geral, o Dioxus deve ser bastante rápido para a maioria dos casos de uso. No entanto, existem algumas regras que você deve seguir para garantir que seus aplicativos sejam rápidos.

1. **Não chame set*state \_durante a renderização***. Isso fará com que o Dioxus verifique desnecessariamente o componente para atualizações ou entre em um `loop` infinito.
2. **Divida seu estado em seções menores.** Hooks são explicitamente projetados para "desvincular" seu estado do paradigma típico de MVC, facilitando a reutilização de bits úteis do código com uma única função.
3. **Mova o estado local para baixo**. O Dioxus precisará verificar novamente os componentes filhos do seu aplicativo se o componente raiz estiver sendo atualizado constantemente. Você obterá melhores resultados se o estado de mudança rápida não causar grandes re-renderizações.

<!-- todo: link when the section exists
Don't worry - Dioxus is fast. But, if your app needs *extreme performance*, then take a look at the `Performance Tuning` in the `Advanced Guides` book.
-->

## O objeto `Scope`

Embora muito semelhante ao React, o Dioxus é diferente em alguns aspectos. Mais notavelmente, os componentes React não terão um parâmetro `Scope` na declaração do componente.

Você já se perguntou como a chamada `useState()` funciona no React sem um objeto `this` para realmente armazenar o estado?

```javascript
// in React:
function Component(props) {
  // This state persists between component renders, but where does it live?
  let [state, set_state] = useState(10)
}
```

O React usa variáveis globais para armazenar essas informações. No entanto, variáveis mutáveis globais devem ser cuidadosamente gerenciadas e são amplamente desencorajadas em programas Rust. Como o Dioxus precisa trabalhar com as regras do Rust, ele usa o `Scope` em vez de um objeto de estado global para manter alguma contabilidade interna.

É isso que o objeto `Scope` é: um local para o `Component` armazenar estado, gerenciar ouvintes e alocar elementos. Usuários avançados do Dioxus vão querer aprender como aproveitar corretamente o objeto `Scope` para construir extensões robustas e de alto desempenho para o Dioxus.

```rust
fn Post(cx: Scope<PostProps>) -> Element {
    cx.render(rsx!("hello"))
}
```

## Seguindo em Frente

Esta visão geral foi muita informação - mas não diz tudo!

Nas próximas seções, abordaremos:

- `use_state` em profundidade
- `use_ref` e outros hooks
- Manipulação de entrada do usuário
