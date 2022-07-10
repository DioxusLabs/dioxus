# Estado Gerenciado

Cada aplicativo que você construir com o Dioxus terá algum tipo de estado que precisa ser mantido e atualizado à medida que seus usuários interagem com ele. No entanto, gerenciar o estado pode ser particularmente desafiador às vezes e é frequentemente a fonte de bugs em muitas estruturas de GUI.

Neste capítulo, abordaremos as várias maneiras de gerenciar o estado, a terminologia apropriada, vários padrões e alguns problemas que você pode encontrar.

## O Problema

Por que as pessoas dizem que a gestão do estado é tão difícil? O que isto significa?

Geralmente, o gerenciamento de estado é o código que você precisa escrever para garantir que seu aplicativo renderize o conteúdo _correto_. Se o usuário inserir um nome, você precisará exibir a resposta apropriada - como alertas, validação e desabilitar/habilitar vários elementos na página. As coisas podem se tornar complicadas rapidamente se você precisar de telas de carregamento e tarefas canceláveis.

Para os aplicativos mais simples, todo o seu estado pode entrar no aplicativo a partir das `props` raiz. Isso é comum na renderização do lado do servidor - podemos coletar todo o estado necessário _antes_ de renderizar o conteúdo.

```rust
let all_content = get_all_content().await;

let output = dioxus::ssr::render_lazy(rsx!{
    div {
        RenderContent { content: all_content }
    }
});
```

Com esta configuração incrivelmente simples, é altamente improvável que você tenha bugs de renderização. Simplesmente há quase nenhum estado para gerenciar.

No entanto, a maioria de seus aplicativos armazenará o estado dentro do Dioxus `VirtualDom` - por meio do estado local ou do estado global.

## Suas opções

Para lidar com a complexidade, você tem algumas opções:

- Refatorar o estado fora do estado compartilhado e em componentes e hooks reutilizáveis.
- Elevar o estado para ser distribuído por vários componentes (fan-out).
- Use a API de contexto para compartilhar o estado globalmente.
- Use uma solução de gerenciamento de estado dedicada como o `Fermi`.
