# Renderizador Personalizado

Dioxus é uma estrutura incrivelmente portátil para desenvolvimento de interface do usuário.

As lições, conhecimentos, Hooks e componentes que você adquire ao longo do tempo sempre podem ser usados para projetos futuros.

No entanto, às vezes, esses projetos não podem aproveitar um renderizador compatível ou você precisa implementar seu próprio renderizador.

Ótimas notícias: o design do renderizador depende inteiramente de você!

Nós fornecemos sugestões e inspiração com os renderizadores originais, mas só realmente exigimos a implementação da `trait` `RealDom` para que as coisas funcionem corretamente.

## As Especificidades:

A implementação do renderizador é bastante simples. O renderizador precisa:

1. Lidar com o fluxo de edições gerado por atualizações no `VirtualDOM`
2. Registrar ouvintes e passar eventos para o sistema de eventos do `VirtualDOM`
3. Prossiga para o `VirtualDOM` com um executor assíncrono (ou desative a API de suspensão e use `progress_sync`)

Essencialmente, seu renderizador precisa implementar a `trait` `RealDom` e gerar objetos `EventTrigger` para atualizar o `VirtualDOM`.

A partir daí, você terá tudo o que precisa para renderizar o `VirtualDOM` na tela.

Internamente, o Dioxus lida com o relacionamento da árvore, diferenciação, gerenciamento de memória e o sistema de eventos, deixando o mínimo necessário para que os renderizadores se implementem.

Para referência, confira o renderizador `WebSys` como ponto de partida para seu renderizador personalizado.

## Implementação de Traits e DomEdit

A característica atual do `RealDom` vive em `dioxus-core/diff`. Uma versão dele é fornecida aqui (mas pode não estar atualizada):

```rust
pub trait RealDom<'a> {
    fn handle_edit(&mut self, edit: DomEdit);
    fn request_available_node(&mut self) -> ElementId;
    fn raw_node_as_any(&self) -> &mut dyn Any;
}
```

Para referência, o tipo `DomEdit` é uma enumeração serializada que representa uma operação atômica que ocorre na `RealDom`. As variantes seguem aproximadamente este conjunto:

```rust
enum DomEdit {
    PushRoot,
    AppendChildren,
    ReplaceWith,
    CreateTextNode,
    CreateElement,
    CreateElementNs,
    CreatePlaceholder,
    NewEventListener,
    RemoveEventListener,
    SetText,
    SetAttribute,
    RemoveAttribute,
}
```

O mecanismo de diferenciação do Dioxus opera como uma [máquina de pilha](https://en.wikipedia.org/wiki/Stack_machine) onde o método "push_root" empilha um novo nó "real" na DOM e "append_child" e "replace_with" " ambos removem nós da pilha.

### Um Exemplo

Por uma questão de compreensão, vamos considerar este exemplo - uma declaração de interface do usuário muito simples:

```rust
rsx!( h1 {"hello world"} )
```

Para começar, o Dioxus deve primeiro navegar até o contêiner dessa tag `h1`. Para "navegar", o algoritmo de diferenciação interna gera o DomEdit `PushRoot` onde o ID da raiz é o contêiner.

Quando o renderizador recebe essa instrução, ele empurra o Nó real para sua própria pilha. A pilha do renderizador real ficará assim:

```rust
instructions: [
    PushRoot(Container)
]
stack: [
    ContainerNode,
]
```

Em seguida, o Dioxus encontrará o nó `h1`. O algoritmo `diff` decide que este nó precisa ser criado, então o Dioxus irá gerar o DomEdit `CreateElement`.

Quando o renderizador receber esta instrução, ele criará um nó desmontado e o enviará para sua própria pilha:

```rust
instructions: [
    PushRoot(Container),
    CreateElement(h1),
]
stack: [
    ContainerNode,
    h1,
]
```

Em seguida, Dioxus vê o nó de texto e gera o DomEdit `CreateTextNode`:

```rust
instructions: [
    PushRoot(Container),
    CreateElement(h1),
    CreateTextNode("hello world")
]
stack: [
    ContainerNode,
    h1,
    "hello world"
]
```

Lembre-se, o nó de texto não está anexado a nada (ele está desmontado), então o Dioxus precisa gerar um Edit que conecte o nó de texto ao elemento `h1`.

Depende da situação, mas neste caso usamos `AppendChildren`. Isso remove o nó de texto da pilha, deixando o elemento `h1` como o próximo elemento na linha.

```rust
instructions: [
    PushRoot(Container),
    CreateElement(h1),
    CreateTextNode("hello world"),
    AppendChildren(1)
]
stack: [
    ContainerNode,
    h1
]
```

Chamamos `AppendChildren` novamente, retirando o nó `h1` e anexando-o ao pai:

```rust
instructions: [
    PushRoot(Container),
    CreateElement(h1),
    CreateTextNode("hello world"),
    AppendChildren(1),
    AppendChildren(1)
]
stack: [
    ContainerNode,
]
```

Finalmente, o contêiner é aberto, pois não precisamos mais dele.

```rust
instructions: [
    PushRoot(Container),
    CreateElement(h1),
    CreateTextNode("hello world"),
    AppendChildren(1),
    AppendChildren(1),
    Pop(1)
]
stack: []
```

Com o tempo, nossa pilha ficou assim:

```rust
[]
[Container]
[Container, h1]
[Container, h1, "hello world"]
[Container, h1]
[Container]
[]
```

Observe como nossa pilha fica vazia depois que a interface do usuário é montada. Convenientemente, essa abordagem separa completamente o `VirtualDOM` e o `RealDOM`. Além disso, essas edições são serializáveis, o que significa que podemos até gerenciar UIs em uma conexão de rede.

Esta pequena máquina de pilha e edições serializadas tornam o Dioxus independente das especificidades da plataforma.

Dioxus também é muito rápido. Como o Dioxus divide a fase de diff e patch, ele é capaz de fazer todas as edições no `RealDOM` em um período de tempo muito curto (menos de um único quadro), tornando a renderização muito rápida.

Ele também permite que o Dioxus cancele grandes operações de diferenciação se ocorrer um trabalho de prioridade mais alta durante a diferenciação.

É importante notar que _há_ uma camada de conexão entre o Dioxus e o renderizador.

Dioxus salva e carrega elementos (a edição `PushRoot`) com um ID. Dentro do `VirtualDOM`, isso é apenas rastreado como um tipo `u64`.

Sempre que uma edição `CreateElement` é gerada durante a comparação, o Dioxus incrementa seu contador de nós e atribui a esse novo elemento seu `NodeCount` atual.

O `RealDom` é responsável por lembrar este ID e enviar o nó correto quando `PushRoot(ID)` é gerado. O Dioxus não recupera IDs de elementos removidos, mas seu renderizador provavelmente desejará.

Para fazer isso, sugerimos usar um `SecondarySlotMap` se implementar o renderizador em Rust, ou apenas adiar para uma abordagem do tipo HashMap.

Esta pequena demonstração serve para mostrar exatamente como um renderizador precisaria processar um stream de edição para construir UIs.

Um conjunto de `EditStreams` serializados para várias demos está disponível para você testar seu renderizador personalizado.

## Ciclo de Eventos

Como a maioria das GUIs, o Dioxus conta com um loop de eventos para progredir no `VirtualDOM`. O próprio `VirtualDOM` também pode produzir eventos, por isso é importante que seu renderizador personalizado também possa lidar com eles.

O código para a implementação do `WebSys` é bem direto, então vamos adicioná-lo aqui para demonstrar como um ciclo de eventos é simples:

```rust
pub async fn run(&mut self) -> dioxus_core::error::Result<()> {
    // Push the body element onto the WebsysDom's stack machine
    let mut websys_dom = crate::new::WebsysDom::new(prepare_websys_dom());
    websys_dom.stack.push(root_node);

    // Rebuild or hydrate the virtualdom
    self.internal_dom.rebuild(&mut websys_dom)?;

    // Wait for updates from the real dom and progress the virtual dom
    loop {
        let user_input_future = websys_dom.wait_for_event();
        let internal_event_future = self.internal_dom.wait_for_event();

        match select(user_input_future, internal_event_future).await {
            Either::Left((trigger, _)) => trigger,
            Either::Right((trigger, _)) => trigger,
        }
    }

    while let Some(trigger) =  {
        websys_dom.stack.push(body_element.first_child().unwrap());
        self.internal_dom
            .progress_with_event(&mut websys_dom, trigger)?;
    }
}
```

É importante que você decodifique os eventos reais do seu sistema de eventos no sistema de eventos sintético do Dioxus (entenda sintético como abstraído).

Isso significa simplesmente combinar seu tipo de evento e criar um tipo Dioxus `VirtualEvent`. Seu evento personalizado deve implementar a `trait` de eventos correspondente. No momento, o sistema `VirtualEvent` é modelado quase inteiramente em torno da especificação `HTML`, mas estamos interessados em reduzi-lo.

```rust
fn virtual_event_from_websys_event(event: &web_sys::Event) -> VirtualEvent {
    match event.type_().as_str() {
        "keydown" | "keypress" | "keyup" => {
            struct CustomKeyboardEvent(web_sys::KeyboardEvent);
            impl dioxus::events::KeyboardEvent for CustomKeyboardEvent {
                fn char_code(&self) -> usize { self.0.char_code() }
                fn ctrl_key(&self) -> bool { self.0.ctrl_key() }
                fn key(&self) -> String { self.0.key() }
                fn key_code(&self) -> usize { self.0.key_code() }
                fn locale(&self) -> String { self.0.locale() }
                fn location(&self) -> usize { self.0.location() }
                fn meta_key(&self) -> bool { self.0.meta_key() }
                fn repeat(&self) -> bool { self.0.repeat() }
                fn shift_key(&self) -> bool { self.0.shift_key() }
                fn which(&self) -> usize { self.0.which() }
                fn get_modifier_state(&self, key_code: usize) -> bool { self.0.get_modifier_state() }
            }
            VirtualEvent::KeyboardEvent(Rc::new(event.clone().dyn_into().unwrap()))
        }
        _ => todo!()
```

## Elementos Brutos Personalizados

Se você precisa ir tão longe a ponto de confiar em elementos personalizados para o seu renderizador - você pode.

Isso ainda permite que você use a natureza reativa do Dioxus, sistema de componentes, estado compartilhado e outros recursos, mas acabará gerando nós diferentes.

Todos os atributos e ouvintes para o namespace `HTML` e `SVG` são transportados por meio de estruturas auxiliares que essencialmente podem ser compiladas (não representam sobrecarga na execução).

Você pode colocar seus próprios elementos a qualquer hora, sem problemas. No entanto, você deve ter certeza absoluta de que seu renderizador pode lidar com o novo tipo, ou ele irá bater e incendiar.

Esses elementos personalizados são definidos como estruturas de unidade com implementações de características.

Por exemplo, o elemento `div` é (aproximadamente!) definido assim:

```rust
struct div;
impl div {
    /// Some glorious documentation about the class property.
    #[inline]
    fn class<'a>(&self, cx: NodeFactory<'a>, val: Arguments) -> Attribute<'a> {
        cx.attr("class", val, None, false)
    }
    // more attributes
}
```

Você provavelmente notou que muitos elementos nas macros `rsx!` e `html!` suportam Documentação apenas passando o mouse por cima.

A abordagem que adotamos para elementos personalizados significa que a estrutura da unidade é criada imediatamente onde o elemento é usado na macro.

Quando o macro é expandido, os `doc comments` ainda se aplicam à estrutura da unidade, dando toneladas de feedback no editor, mesmo dentro de um macro procedural.

## Compatibilidade

Aviso prévio: nem todos os Hooks e serviços funcionarão na sua plataforma. Dioxus envolve o que precisa para ser multiplataforma em tipos "sintéticos". No entanto, a conversão para um tipo nativo pode falhar se os tipos não corresponderem.

Existem três tipos de incompatibilidades de plataforma para quebrar seu programa:

1. Ao fazer `downcast` de elementos via `Ref.downcast_ref<T>()`
2. Ao fazer `downcast` de eventos via `Event.downcast_ref<T>()`
3. Chamando APIs específicas da plataforma que não existem

Os melhores Hooks detectarão corretamente a plataforma de destino e ainda fornecerão funcionalidade, falhando de forma elegante caso uma plataforma não for suportada.

Incentivamos - e fornecemos - uma indicação ao usuário em quais plataformas um Hook suporta.

Para os problemas 1 e 2, eles retornam um `Result` para não causar pânico em plataformas não suportadas. Ao projetar seus Hooks, recomendamos propagar esse erro no código voltado para o usuário, tornando óbvio que esse serviço específico não é suportado.

Este código em particular _entrará em pânico_ devido ao desempacotamento em `downcast_ref`. Tente evitar esses tipos de padrões.

```rust
let div_ref = use_node_ref(cx);

cx.render(rsx!{
    div { ref: div_ref, class: "custom class",
        button { "click me to see my parent's class"
            onclick: move |_| if let Some(div_ref) = div_ref {
                log::info!("Div class is {}", div_ref.downcast_ref::<web_sys::Element>().unwrap().class())
            }
        }
    }
})

```

## Conclusão

Pronto! Você deve ter quase todo o conhecimento necessário sobre como implementar seu próprio renderizador.

Estamos super interessados em ver os aplicativos Dioxus trazidos para renderizadores de desktop personalizados, renderizador para dispositivos móveis, interface do usuário de videogame e até realidade aumentada!

Se você estiver interessado em contribuir para qualquer um desses projetos, não tenha medo de entrar em contato ou se juntar à comunidade.
