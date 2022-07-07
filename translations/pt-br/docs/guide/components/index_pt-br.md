# Introdução aos Componentes

No capítulo anterior, aprendemos sobre `Elements` e como eles podem ser compostos para criar uma interface de usuário básica. Agora, vamos aprender como agrupar `Elements` para formar `Components`.

Cobriremos:

- O que faz um componente
- Como modelar um componente e suas propriedades no Dioxus
- Como "pensar declarativamente"

## O que é um componente?

Resumindo, um componente é uma função especial que recebe propriedades de entrada e gera um elemento como saída. Assim como uma função encapsula alguma tarefa de computação específica, um `Component` encapsula alguma tarefa de renderização específica – normalmente, renderizando uma parte isolada da interface do usuário.

### Exemplo do mundo real

Vamos usar um post do Reddit como exemplo:

![Reddit Post](../images/reddit_post.png)

Se observarmos o layout do componente, notamos alguns botões e peças de funcionalidade:

- Voto positivo / negativo
- Ver comentários
- Compartilhar
- Salvar
- Esconder
- Dar prêmio
- Relatório
- Publicação cruzada
- Filtrar por site
- Ver artigo
- Visitar usuário

Se incluíssemos todas essas funcionalidades em uma chamada `rsx!`, seria enorme! Em vez disso, vamos dividir o post em `Components`:

![Postar como componente](../images/reddit_post_components.png)

- **VoteButton**: VotoPositivo/VotoNegativo
- **TitleCard**: Título, Filtro por URL
- **MetaCard**: Pôster Original, Prazo de Envio
- **ActionCard**: veja comentários, compartilhe, salve, oculte, dê prêmio, denuncie, publicação cruzada

Neste capítulo, aprenderemos como definir esses componentes.
