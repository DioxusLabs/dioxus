# Pensando Reativamente

Finalmente chegamos ao ponto em nosso tutorial onde podemos falar sobre a teoria da Reatividade. Falamos sobre definir uma visão declarativa, mas não sobre os aspectos que tornam nosso código _reativo_.

Compreender a teoria da programação reativa é essencial para entender o Dioxus e escrever interfaces de usuário eficazes e de alto desempenho.

Nesta seção, falaremos sobre:

- Fluxo de dados unidirecional
- Modificando dados
- Forçando renderizações
- Como as renderizações se propagam

Esta seção é um pouco longa, mas vale a leitura. Recomendamos café, chá e/ou lanches.

## Programação Reativa

Dioxus é uma das poucas bibliotecas Rust que fornecem um "Modelo de Programação Reativa".

O termo "programação reativa" é uma classificação do paradigma de programação - muito parecido com a programação funcional ou imperativa. Essa é uma distinção muito importante, pois afeta como _pensamos_ sobre nosso código.

A programação reativa é um modelo de programação preocupado em derivar cálculos de fluxo de dados assíncrono. A maioria dos programas reativos são compostos de fontes de dados, cálculos intermediários e um resultado final.

Consideramos a GUI renderizada como o resultado final de nossos aplicativos Dioxus. As fontes de dados para nossos aplicativos incluem estado local e global.

Por exemplo, o modelo apresentado na figura abaixo é composto por duas fontes de dados: tempo e uma constante. Esses valores são passados pelo nosso gráfico para obter um resultado final: `g`.

![Reactive Model](https://upload.wikimedia.org/wikipedia/commons/thumb/e/e9/Reactive_programming_glitches.svg/440px-Reactive_programming_glitches.svg.png)

Sempre que nossa variável `seconds` muda, reavaliaremos para `t`. Como `g` depende de `t`, também reavaliaremos seus computação. Observe que teríamos reavaliado a computação para `g` mesmo que `t` não mudasse porque `seconds` é usado para calcular `g`.

No entanto, se de alguma forma alteramos nossa constante de `1` para `2`, precisamos reavaliar `t`. Se, por qualquer motivo, essa mudança não afetasse o resultado de `t`, então não tentaríamos reavaliar `g`.

Na Programação Reativa, não pensamos se devemos ou não reavaliar `t` ou `g`; em vez disso, simplesmente fornecemos funções e deixamos a estrutura descobrir o resto para nós.

Em Rust, nosso aplicativo reativo seria algo como:

```rust
fn compute_g(t: i32, seconds: i32) -> bool {
    t > seconds
}

fn compute_t(constant: i32, seconds: i32) -> i32 {
    constant + seconds
}

fn compute_graph(constant: i32, seconds: i32) -> bool {
    let t = compute_t(constant, seconds);
    let g = compute_g(t, seconds);
    g
}
```

## Como o Dioxus é Reativo?

O Dioxus tem uma `VirtualDom` que nos fornece um framework para programação reativa. Quando criamos aplicativos com o Dioxus, precisamos fornecer nossas próprias fontes de dados.

Isso pode ser uma `props` inicial ou alguns valores obtidos da rede. Em seguida, passamos esses dados por meio de nosso aplicativo para os componentes por meio de propriedades.

Se representássemos o gráfico reativo apresentado acima no Dioxus, ficaria muito semelhante:

```rust
// Declare a component that holds our datasources and calculates `g`
fn RenderGraph(cx: Scope) -> Element {
    let seconds = use_datasource(SECONDS);
    let constant = use_state(&cx, || 1);

    cx.render(rsx!(
        RenderG { seconds: seconds }
        RenderT { seconds: seconds, constant: constant }
    ))
}

// "calculate" g by rendering `t` and `seconds`
#[inline_props]
fn RenderG(cx: Scope, seconds: i32) -> Element {
    cx.render(rsx!{ "There are {seconds} seconds remaining..." })
}

// calculate and render `t` in its own component
#[inline_props]
fn RenderT(cx: Scope, seconds: i32, constant: i32) -> Element {
    let res = seconds + constant;
    cx.render(rsx!{ "{res}" })
}
```

Com este aplicativo, definimos três componentes. Nosso componente de nível superior fornece nossas fontes de dados (os Hooks), nós de computação (componentes filhos) e um valor final (o que é "renderizado").

Agora, sempre que a `constant` mudar, nosso componente `RenderT` será renderizado novamente. No entanto, se `seconds` não mudar, não precisamos renderizar novamente `RenderG` porque a entrada é a mesma. Se `seconds` _mudar_, então `RenderG` e `RenderT` serão reavaliados.

Dioxus é "Reativo" porque fornece essa estrutura para nós. Tudo o que precisamos fazer é escrever nossas próprias pequenas unidades de computação e o Dioxus descobrirá quais componentes precisam ser reavaliados automaticamente.

Essas verificações e algoritmos extras adicionam alguma sobrecarga, e é por isso que você vê projetos como [Sycamore](http://sycamore-rs.netlify.app) e [SolidJS](http://solidjs.com) eliminando-os completamente. Dioxus é _realmente_ rápido, então estamos dispostos a trocar a sobrecarga adicional por uma experiência aprimorada do desenvolvedor.

## Como Atualizamos Valores em nosso gráfico de Fluxo de Dados?

O Dioxus descobrirá automaticamente como regenerar partes do nosso aplicativo quando as fontes de dados forem alteradas. Mas como exatamente podemos atualizar nossas fontes de dados?

No Dioxus existem duas fontes de dados:

1. Estado local em `use_hook` e todos os outros hooks
2. Estado global através de `provide_context`.

Tecnicamente, os `root props` do `VirtualDom` são uma terceira fonte de dados, mas como não podemos modificá-los, não vale a pena falar sobre eles.

### Estado Local

Para o estado local nos hooks, o Dioxus nos dá o método `use_hook` que retorna um `&mut T` sem nenhum requisito. Isso significa que os valores brutos do hook não são rastreados pelo Dioxus.

Na verdade, poderíamos escrever um componente que modifica um valor de hook diretamente:

```rust
fn app(cx: Scope) -> Element {
    let mut count = cx.use_hook(|_| 0);
    cx.render(rsx!{
        button {
            onclick: move |_| *count += 1,
            "Count: {count}"
        }
    })
}
```

No entanto, quando esse valor é gravado, o componente não sabe ser reavaliado. Devemos dizer explicitamente ao Dioxus que este componente está "sujo" e precisa ser renderizado novamente.

Isso é feito através do método `cx.needs_update`:

```rust
button {
    onclick: move |_| {
        *count += 1;
        cx.needs_update();
    },
    "Count: {count}"
}
```

Agora, sempre que clicarmos no botão, o valor será alterado e o componente será renderizado novamente.

> Re-renderização é quando o Dioxus chama seu componente funcional _novamente_. As funções do componente serão chamadas repetidamente ao longo de sua vida, portanto, devem ser livres de efeitos colaterais.

### Entenda Isso!

Suas funções de componente serão chamadas ("renderizadas" em nosso jargão) enquanto o componente estiver presente na árvore.

Um único componente será chamado várias vezes, modificando seu próprio estado interno ou renderizando novos nós com novos valores de suas propriedades.

### Estado Global do Aplicativo

Com os métodos `provide_context` e `consume_context` em `Scope`, podemos compartilhar valores com descendentes sem ter que passar valores através de `props` dos componentes.

Isso tem o efeito colateral de tornar nossas fontes de dados menos óbvias de uma perspectiva de alto nível, mas torna nossos componentes mais modulares dentro da mesma base de código.

Para tornar o estado global do aplicativo mais fácil de raciocinar, o Dioxus torna todos os valores fornecidos por meio de `provide_context` imutáveis. Isso significa que qualquer biblioteca construída em cima de `provide_context` precisa usar a mutabilidade interior para modificar o estado global compartilhado.

Nesses casos, o Estado Global do aplicativo precisa rastrear manualmente quais componentes precisam ser gerados novamente.

Para regenerar _qualquer_ componente em seu aplicativo, você pode obter um `handle` para o agendador interno do Dioxus através de `schedule_update_any`:

```rust
let force_render = cx.schedule_update_any();

// force a render of the root component
force_render(ScopeId(0));
```

## O que significa um componente "re-renderizar"?

Em nossos guias, frequentemente usamos a frase "re-renderizar" para descrever as atualizações do nosso aplicativo. Muitas vezes você ouvirá isso combinado com "prevenir re-renderizações desnecessárias". Mas o que exatamente isso significa?

Quando chamamos `dioxus::desktop::launch`, o Dioxus criará um novo objeto `Scope` e chamará o componente que demos a ele. Nossas chamadas `rsx!` criarão novos nós que retornaremos ao `VirtualDom`.

O Dioxus então procurará nesses nós por componentes filhos, chamará suas funções e assim por diante até que todos os componentes tenham sido "renderizados". Consideramos esses nós "renderizados" porque foram criados por causa de nossas ações explícitas.

A árvore da interface do usuário que o Dioxus cria se parecerá com a árvore de componentes apresentada anteriormente:

![Árvore da UI](../images/component_tree.png)

Mas o que acontece quando chamamos `needs_update` depois de modificar algum estado importante? Bem, se Dioxus chamasse a função do nosso componente novamente, então produziríamos novos nós diferentes. Na verdade, isso é exatamente o que Dioxus faz!

Neste ponto, temos alguns nós antigos e alguns novos nós. Novamente, chamamos isso de "renderização" porque Dioxus teve que criar novos nós por causa de nossas ações explícitas. Sempre que novos nós são criados, nosso `VirtualDom` está sendo "renderizado".

Esses nós são armazenados em um alocador de memória extremamente eficiente chamado "arena Bump". Por exemplo, um div com um `handle` e `props` seria armazenado na memória em dois locais: a árvore "antiga" e a árvore "nova".

![Arenas Bump](../images/oldnew.png)

A partir daqui, Dioxus calcula a diferença entre essas árvores e atualiza a `DOM Real` para torná-la parecida com a nova versão do que declaramos.

![Diffing](../images/diffing.png)

## Suprimindo Renderizações

Então, nós sabemos como fazer o Dioxus renderizar, mas como podemos _parar_ isso? E se _sabemos_ que nosso estado não mudou e não devemos renderizar e diferenciar novos nós porque eles serão exatamente os mesmos da última vez?

Nesses casos, você deseja uma _memoização_. No Dioxus, a memoização envolve impedir que um componente seja renderizado novamente se suas `props` não mudaram desde a última vez que ele tentou renderizar.

Visualmente, você pode dizer que um componente só será renderizado novamente se o novo valor for suficientemente diferente do antigo.

| props.val | re-render |
| --------- | --------- |
| 10        | true      |
| 20        | true      |
| 20        | false     |
| 20        | false     |
| 10        | true      |
| 30        | false     |

É por isso que quando você usa `derive(Props)`, você também deve implementar a `trait` `PartialEq`. Para substituir a estratégia de memiização de um componente, você pode simplesmente implementar seu próprio `PartialEq`.

```rust
struct CustomProps {
    val: i32,
}

impl PartialEq for CustomProps {
    fn partial_eq(&self, other: &Self) -> bool {
        // we don't render components that have a val less than 5
        if other.val > 5 && self.val > 5{
            self.val == other.val
        }
    }
}
```

No entanto, para componentes que emprestam dados, não faz sentido implementar `PartialEq`, pois as referências reais na memória podem ser diferentes.

Você pode substituir tecnicamente esse comportamento implementando a `trait` `Props` manualmente, embora seja inseguro e fácil de dar errado:

```rust
unsafe impl Properties for CustomProps {
    fn memoize(&self, other &Self) -> bool {
        self != other
    }
}
```

Resumindo:

- Dioxus verifica se as `props` mudaram entre as renderizações
- Se as `props` foram alterados de acordo com `PartialEq`, o Dioxus renderiza novamente o componente
- `Props` que têm uma vida útil (ou seja, `lifetime` `<'a>`) sempre serão renderizados novamente

## Recapitulando

Nossa, foi muito material!

Vamos ver se podemos recapitular o que foi apresentado:

- A programação reativa calcula um valor final de fontes de dados e computação
- Dioxus é "reativo", pois compara quais cálculos devem ser verificados
- `schedule_update` deve ser chamado para marcar um componente como sujo
- componentes sujos serão renderizados novamente (chamados várias vezes) para produzir uma nova interface do usuário
- As renderizações podem ser suprimidas com memoização

Essa teoria é crucial para entender como compor componentes e como controlar renderizações em seu aplicativo.
