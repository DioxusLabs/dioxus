# Renderização Dinâmica

Às vezes você quer renderizar coisas diferentes dependendo do estado/_props_. Com o Dioxus, apenas descreva o que você quer ver usando o fluxo de controle do Rust – o _framework_ se encarregará de fazer as mudanças necessárias em tempo real se o estado ou _props_ mudarem!

## Renderização Condicional

Para renderizar diferentes elementos com base em uma condição, você pode usar uma instrução `if-else`:

```rust, no_run
{{#include ../../../examples/conditional_rendering.rs:if_else}}
```

> Você também pode usar instruções `match`, ou qualquer função Rust para renderizar condicionalmente coisas diferentes.

### Inspecionando _props_ `Element`

Como `Element` é uma `Option<VNode>`, os componentes que aceitam `Element` como _prop_ podem realmente inspecionar seu conteúdo e renderizar coisas diferentes com base nisso. Exemplo:

```rust, no_run
{{#include ../../../examples/component_children_inspect.rs:Clickable}}
```

Você não pode modificar o `Element`, mas se precisar de uma versão modificada dele, você pode construir um novo baseado em seus atributos/filhos/etc.

## Renderizando Nada

Para renderizar nada, você pode retornar `None` de um componente. Isso é útil se você deseja ocultar algo condicionalmente:

```rust, no_run
{{#include ../../../examples/conditional_rendering.rs:conditional_none}}
```

Isso funciona porque o tipo `Element` é apenas um alias para `Option<VNode>`

> Novamente, você pode usar um método diferente para retornar condicionalmente `None`. Por exemplo, a função _booleana_ [`then()`](https://doc.rust-lang.org/std/primitive.bool.html#method.then) pode ser usada.

## Listas de renderização

Frequentemente, você desejará renderizar uma coleção de componentes. Por exemplo, você pode querer renderizar uma lista de todos os comentários em uma postagem.

Para isso, o Dioxus aceita iteradores que produzem `Element`s. Então precisamos:

- Obter um iterador sobre todos os nossos itens (por exemplo, se você tiver um `Vec` de comentários, itere sobre ele com `iter()`)
- `.map` o iterador para converter cada item em um `Element` renderizado usando `cx.render(rsx!(...))`
  - Adicione um atributo `key` exclusivo a cada item do iterador
- Incluir este iterador no RSX final

Exemplo: suponha que você tenha uma lista de comentários que deseja renderizar. Então, você pode renderizá-los assim:

```rust, no_run
{{#include ../../../examples/rendering_lists.rs:render_list}}
```

### O Atributo `key`

Toda vez que você renderiza novamente sua lista, o Dioxus precisa acompanhar qual item foi para onde, porque a ordem dos itens em uma lista pode mudar – itens podem ser adicionados, removidos ou trocados. Apesar disso, Dioxus precisa:

- Acompanhar o estado do componente
- Descobrir com eficiência quais atualizações precisam ser feitas na interface do usuário

Por exemplo, suponha que o `CommentComponent` tenha algum estado – ex. um campo onde o usuário digitou uma resposta. Se a ordem dos comentários mudar repentinamente, o Dioxus precisa associar corretamente esse estado ao mesmo comentário – caso contrário, o usuário acabará respondendo a um comentário diferente!

Para ajudar o Dioxus a acompanhar os itens da lista, precisamos associar cada item a uma chave exclusiva. No exemplo acima, geramos dinamicamente a chave exclusiva. Em aplicações reais, é mais provável que a chave venha de, por exemplo, um ID de banco de dados. Realmente não importa de onde você obtém a chave, desde que atenda aos requisitos

- As chaves devem ser únicas em uma lista
- O mesmo item deve sempre ser associado à mesma chave
- As chaves devem ser relativamente pequenas (ou seja, converter toda a estrutura Comment em uma String seria uma chave muito ruim) para que possam ser comparadas com eficiência

Você pode ficar tentado a usar o índice de um item na lista como sua chave. Na verdade, é isso que o Dioxus usará se você não especificar uma chave. Isso só é aceitável se você puder garantir que a lista seja constante – ou seja, sem reordenação, adições ou exclusões.

> Observe que se você passar a chave para um componente que você criou, ele não receberá a chave como _prop_. É usado apenas como uma dica pelo próprio Dioxus. Se o seu componente precisar de um ID, você deve passá-lo como um _prop_ separado.
