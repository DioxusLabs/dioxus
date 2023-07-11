# Estado de compartilhamento

Muitas vezes, vários componentes precisam acessar o mesmo estado. Dependendo de suas necessidades, existem várias maneiras de implementar isso.

## Elevenado o Estado

Uma abordagem para compartilhar o estado entre os componentes é "elevá-lo" até o ancestral comum mais próximo. Isso significa colocar o _hook_ `use_state` em um componente pai e passar os valores necessários como _props_.

Por exemplo, suponha que queremos construir um editor de memes. Queremos ter uma entrada para editar a legenda do meme, mas também uma visualização do meme com a legenda. Logicamente, o meme e a entrada são 2 componentes separados, mas precisam acessar o mesmo estado (a legenda atual).

> Claro, neste exemplo simples, poderíamos escrever tudo em um componente - mas é melhor dividir tudo em componentes menores para tornar o código mais reutilizável e fácil de manter (isso é ainda mais importante para aplicativos maiores e complexos) .

Começamos com um componente `Meme`, responsável por renderizar um meme com uma determinada legenda:

```rust, no_run
{{#include ../../../examples/meme_editor.rs:meme_component}}
```

> Observe que o componente `Meme` não sabe de onde vem a legenda – ela pode ser armazenada em `use_state`, `use_ref` ou uma constante. Isso garante que seja muito reutilizável - o mesmo componente pode ser usado para uma galeria de memes sem nenhuma alteração!

Também criamos um editor de legendas, totalmente desacoplado do meme. O editor de legendas não deve armazenar a legenda em si – caso contrário, como iremos fornecê-la ao componente `Meme`? Em vez disso, ele deve aceitar a legenda atual como um suporte, bem como um manipulador de eventos para delegar eventos de entrada para:

```rust, no_run
{{#include ../../../examples/meme_editor.rs:caption_editor}}
```

Finalmente, um terceiro componente renderizará os outros dois como filhos. Ele será responsável por manter o estado e passar os _props_ relevantes.

```rust, no_run
{{#include ../../../examples/meme_editor.rs:meme_editor}}
```

![Captura de tela do editor de memes: Um velho esqueleto de plástico sentado em um banco de parque. Legenda: "eu esperando por um recurso de idioma"](./images/meme_editor_screenshot.png)

## Usando o contexto

Às vezes, algum estado precisa ser compartilhado entre vários componentes na árvore, e passá-lo pelos _props_ é muito inconveniente.

Suponha agora que queremos implementar uma alternância de modo escuro para nosso aplicativo. Para conseguir isso, faremos com que cada componente selecione o estilo dependendo se o modo escuro está ativado ou não.

> Nota: estamos escolhendo esta abordagem como exemplo. Existem maneiras melhores de implementar o modo escuro (por exemplo, usando variáveis ​​CSS). Vamos fingir que as variáveis ​​CSS não existem – bem-vindo a 2013!

Agora, poderíamos escrever outro `use_state` no componente superior e passar `is_dark_mode` para cada componente através de _props_. Mas pense no que acontecerá à medida que o aplicativo crescer em complexidade – quase todos os componentes que renderizam qualquer CSS precisarão saber se o modo escuro está ativado ou não – para que todos precisem do mesmo suporte do modo escuro. E cada componente pai precisará passá-lo para eles. Imagine como isso ficaria confuso e verboso, especialmente se tivéssemos componentes com vários níveis de profundidade!

A Dioxus oferece uma solução melhor do que esta "perfuração com hélice" – fornecendo contexto. O _hook_ [`use_context_provider`](https://docs.rs/dioxus-hooks/latest/dioxus_hooks/fn.use_context_provider.html) é semelhante ao `use_ref`, mas o torna disponível através do [`use_context`](https://docs.rs/dioxus-hooks/latest/dioxus_hooks/fn.use_context.html) para todos os componentes filhos.

Primeiro, temos que criar um _struct_ para nossa configuração de modo escuro:

```rust, no_run
{{#include ../../../examples/meme_editor_dark_mode.rs:DarkMode_struct}}
```

Agora, em um componente de nível superior (como `App`), podemos fornecer o contexto `DarkMode` para todos os componentes filhos:

```rust, no_run
{{#include ../../../examples/meme_editor_dark_mode.rs:context_provider}}
```

Como resultado, qualquer componente filho de `App` (direto ou não), pode acessar o contexto `DarkMode`.

```rust, no_run
{{#include ../../../examples/meme_editor_dark_mode.rs:use_context}}
```

> `use_context` retorna `Option<UseSharedState<DarkMode>>` aqui. Se o contexto foi fornecido, o valor é `Some(UseSharedState)`, que você pode chamar `.read` ou `.write`, similarmente a `UseRef`. Caso contrário, o valor é `None`.

Por exemplo, aqui está como implementaríamos a alternância do modo escuro, que lê o contexto (para determinar a cor que deve renderizar) e grava nele (para alternar o modo escuro):

```rust, no_run
{{#include ../../../examples/meme_editor_dark_mode.rs:toggle}}
```
