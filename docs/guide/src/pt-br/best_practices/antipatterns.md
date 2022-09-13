# Antipadrões

Este exemplo mostra o que não fazer e fornece uma razão pela qual um determinado padrão é considerado um "AntiPattern". A maioria dos antipadrões são considerados errados por motivos de desempenho ou por prejudicar a reutilização do código.

## Fragmentos Aninhados Desnecessariamente

Os fragmentos não montam um elemento físico no DOM imediatamente, então o Dioxus deve recorrer a seus filhos para encontrar um nó DOM físico. Este processo é chamado de "normalização". Isso significa que fragmentos profundamente aninhados fazem o Dioxus realizar um trabalho desnecessário. Prefira um ou dois níveis de fragmentos/componentes aninhados até apresentar um elemento DOM verdadeiro.

Apenas os nós Componente e Fragmento são suscetíveis a esse problema. O Dioxus atenua isso com componentes fornecendo uma API para registrar o estado compartilhado sem o padrão _Context Provider_.

```rust
{{#include ../../../examples/anti_patterns.rs:nested_fragments}}
```

## Chaves do Iterador Incorretas

Conforme descrito no capítulo de renderização condicional, os itens da lista devem ter _keys_ exclusivas associadas aos mesmos itens nas renderizações. Isso ajuda o Dioxus a associar o estado aos componentes contidos e garante um bom desempenho de diferenciação. Não omita as _keys_, a menos que você saiba que a lista é estática e nunca será alterada.

```rust
{{#include ../../../examples/anti_patterns.rs:iter_keys}}
```

## Evite Mutabilidade Interior em `Props`

Embora seja tecnicamente aceitável ter um `Mutex` ou um `RwLock` nos _props_, eles serão difíceis de usar.

Suponha que você tenha um _struct_ `User` contendo o campo `username: String`. Se você passar uma _prop_ `Mutex<User>` para um componente `UserComponent`, esse componente pode querer passar o nome de usuário como uma _prop_ `&str` para um componente filho. No entanto, ele não pode passar esse campo emprestado, pois ele só viveria enquanto o bloqueio do `Mutex`, que pertence à função `UserComponent`. Portanto, o componente será forçado a clonar o campo `username`.

## Evite Atualizar o Estado Durante a Renderização

Toda vez que você atualiza o estado, o Dioxus precisa renderizar novamente o componente – isso é ineficiente! Considere refatorar seu código para evitar isso.

Além disso, se você atualizar incondicionalmente o estado durante a renderização, ele será renderizado novamente em um _loop_ infinito.
