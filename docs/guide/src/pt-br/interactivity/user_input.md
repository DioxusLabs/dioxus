# Entradas do Usuário

As interfaces geralmente precisam fornecer uma maneira de inserir dados: por exemplo, texto, números, caixas de seleção, etc. No Dioxus, há duas maneiras de trabalhar com a entrada do usuário.

## Entradas Controladas

Com entradas controladas, você fica diretamente responsável pelo estado da entrada. Isso lhe dá muita flexibilidade e facilita manter as coisas em sincronia. Por exemplo, é assim que você criaria uma entrada de texto controlada:

```rust
{{#include ../../../examples/input_controlled.rs:component}}
```

Observe a flexibilidade - você pode:

- Exibir o mesmo conteúdo em outro elemento, e eles estarão em sincronia
- Transformar a entrada toda vez que for modificada (por exemplo, para garantir que seja maiúscula)
- Validar a entrada toda vez que ela mudar
- Ter uma lógica personalizada acontecendo quando a entrada for alterada (por exemplo, solicitação de rede para preenchimento automático)
- Alterar programaticamente o valor (por exemplo, um botão "randomize" que preenche a entrada com absurdos)

## Entradas não Controladas

Como alternativa às entradas controladas, você pode simplesmente deixar a plataforma acompanhar os valores de entrada. Se não dissermos a uma entrada HTML qual conteúdo ela deve ter, ela poderá ser editada de qualquer maneira (isso está embutido na visualização da web). Essa abordagem pode ser mais eficiente, mas menos flexível. Por exemplo, é mais difícil manter a entrada sincronizada com outro elemento.

Como você não tem necessariamente o valor atual da entrada não controlada no estado, você pode acessá-lo ouvindo os eventos `oninput` (de maneira semelhante aos componentes controlados) ou, se a entrada for parte de um formulário, você pode acessar os dados do formulário nos eventos do formulário (por exemplo, `oninput` ou `onsubmit`):

```rust
{{#include ../../../examples/input_uncontrolled.rs:component}}
```

```
Submitted! UiEvent { data: FormData { value: "", values: {"age": "very old", "date": "1966", "name": "Fred"} } }
```
