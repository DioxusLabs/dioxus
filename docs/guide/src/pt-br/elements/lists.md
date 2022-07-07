# Listas Condicionais e Chaves

Muitas vezes, você precisará exibir vários componentes semelhantes de uma coleção de dados.

Neste capítulo, você aprenderá:

- Como usar iteradores em `rsx!`
- Como filtrar e transformar dados em uma lista de elementos
- Como criar listas eficientes com chaves

## Renderizando Dados de Listas

Se quisermos construir o aplicativo Reddit, precisamos implementar uma lista de dados que precisam ser renderizados: a lista de postagens. Esta lista de postagens está sempre mudando, então não podemos simplesmente codificar as listas diretamente em nosso aplicativo, assim:

```rust
// we shouldn't ship our app with posts that don't update!
rsx!(
    div {
        Post {
            title: "Post A",
            votes: 120,
        }
        Post {
            title: "Post B",
            votes: 14,
        }
        Post {
            title: "Post C",
            votes: 999,
        }
    }
)
```

Em vez disso, precisamos transformar a lista de dados em uma lista de elementos.

Por conveniência, o `rsx!` suporta qualquer tipo de chave que implemente a `trait` `IntoVnodeList`. Convenientemente, todo iterador que retorna algo que pode ser renderizado como um `Element` também implementa `IntoVnodeList`.

Como um exemplo simples, vamos renderizar uma lista de nomes. Primeiro, comece com nossos dados de entrada:

```rust
let names = ["jim", "bob", "jane", "doe"];
```

Then, we create a new iterator by calling `iter` and then [`map`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.map). In our `map` function, we'll render our template.

```rust
let name_list = names.iter().map(|name| rsx!(
    li { "{name}" }
));
```

We can include this list in the final Element:

```rust
rsx!(
    ul {
        name_list
    }
)
```

Em vez de armazenar `name_list` em uma variável temporária, também podemos incluir o iterador inline:

```rust
rsx!(
    ul {
        names.iter().map(|name| rsx!(
            li { "{name}" }
        ))
    }
)
```

A lista HTML renderizada é o que você esperaria:

```html
<ul>
  <li>jim</li>
  <li>bob</li>
  <li>jane</li>
  <li>doe</li>
</ul>
```

## Filtrando Iteradores

Os iteradores do Rust são extremamente poderosos, especialmente quando usados para filtrar tarefas. Ao construir interfaces de usuário, você pode querer exibir uma lista de itens filtrados por alguma verificação arbitrária.

Como um exemplo bem simples, vamos configurar um filtro onde listamos apenas os nomes que começam com a letra "j".

Usando a lista acima, vamos criar um novo iterador. Antes de renderizar a lista com `map` como no exemplo anterior, vamos user [`filter`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.filter) nos nomes para permitir apenas aqueles que começam com "j".

```rust
let name_list = names
    .iter()
    .filter(|name| name.starts_with('j'))
    .map(|name| rsx!( li { "{name}" }));
```

Os Iteradores do Rust são muito versáteis – confira [sua documentação](https://doc.rust-lang.org/std/iter/trait.Iterator.html) para mais coisas que você pode fazer com eles!

Para rustáceos perspicazes: observe como nós não chamamos `collect` na lista de nomes. Se nós `coletarmos` nossa lista filtrada em um novo Vec, precisaríamos fazer uma alocação para armazenar esses novos elementos, o que torna a renderização mais lenta.

Em vez disso, criamos um iterador _lazy_ inteiramente novo que o Dioxus consumirá na chamada `render`. O método `render` é extraordinariamente eficiente, então é uma boa prática deixá-lo fazer a maioria das alocações para nós.

## Mantendo os itens da lista em ordem com `key`

Os exemplos acima demonstram o poder dos iteradores no `rsx!`, mas todos compartilham o mesmo problema: se os itens do seu array se movem (por exemplo, devido à classificação), são inseridos ou excluídos, o Dioxus não tem como saber o que aconteceu. Isso pode fazer com que os elementos sejam removidos, alterados e reconstruídos desnecessariamente quando tudo o que era necessário era mudar sua posição – isso é ineficiente.

Para resolver esse problema, cada item da lista deve ser **identificável de forma única**. Você pode conseguir isso dando a ele uma "chave" única e fixa. No Dioxus, uma chave é uma string que identifica um item entre outros na lista.

```rust
rsx!( li { key: "a" } )
```

Agora, se um item já foi renderizado uma vez, o Dioxus pode usar a chave para combiná-lo mais tarde para fazer as atualizações corretas – e evitar trabalho desnecessário.

Nota: a linguagem desta seção é fortemente emprestada do [react's guide on keys](https://reactjs.org/docs/lists-and-keys.html).

### Onde obter sua chave

Diferentes fontes de dados fornecem diferentes fontes de chaves:

- _Dados de um banco de dados_: Se seus dados são provenientes de um banco de dados, você pode usar as chaves/IDs do banco de dados, que são únicas por natureza.
- _Dados gerados localmente_: Se seus dados são gerados e persistidos localmente (por exemplo, notas em um aplicativo de anotações), acompanhe as chaves junto com seus dados. Você pode usar um contador de incremento ou um pacote como `uuid` para gerar chaves para novos itens – mas certifique-se de que eles permaneçam os mesmos durante a vida útil do item.

Lembre-se: as chaves permitem que Dioxus identifique um item de forma única entre seus irmãos. Uma chave bem escolhida fornece mais informações do que a posição dentro da matriz. Mesmo que a posição mude devido ao reordenamento, a chave permite que a Dioxus identifique o item ao longo de sua vida útil.

### Regras das Chaves

- As chaves devem ser únicas entre irmãos. No entanto, não há problema em usar as mesmas chaves para `Elements` em `arrays` diferentes.
- A chave de um item não deve ser alterada – **não as gere em tempo real** durante a renderização. Caso contrário, Dioxus não conseguirá acompanhar qual item é qual, e voltamos à estaca zero.

Você pode ficar tentado a usar o índice de um item na matriz como sua chave. Na verdade, é isso que o Dioxus usará se você não especificar uma chave. Isso só é aceitável se você puder garantir que a lista seja constante – ou seja, sem reordenação, adições ou exclusões. Em todos os outros casos, não use o índice para a chave – isso levará aos problemas de desempenho descritos acima.

Observe que se você passar a chave para um [componente personalizado](../components/index.md) que você criou, ele não receberá a chave como um `prop`. É usado apenas como uma dica pelo próprio Dioxus. Se o seu componente precisar de um ID, você deve passá-lo como um `prop` separado:

```rust
Post { key: "{key}", id: "{key}" }
```

## Seguindo em Frente

Nesta seção, aprendemos:

- Como renderizar listas de dados
- Como usar ferramentas iteradoras para filtrar e transformar dados
- Como usar chaves para renderizar listas de forma eficiente

Daqui para frente, aprenderemos mais sobre atributos.
