# Sinais: Pulando o `diff`

Na maioria dos casos, o padrão da tradicional `VirtualDOM` checa diferenças (`diffing`) de forma bem rápida.

No Dioxus iremos comparar árvores de nós virtuais (`VNodes`), encontrar diferenças, e então atualizar a `DOM` com essas diferenças.

No entanto, isso gera muita sobrecarga para certos tipos de componentes.

Em aplicativos onde a latência visual é prioridade máxima, você pode optar pela API `Signals` para desabilitar o `diffing` da árvore desses componentes.

O Dioxus irá então contruir automaticamente uma máquina de estados para o seu componente, fazendo com que as atualizações sejam quase instantâneas.

Os sinais são construídos na mesma infraestrutura que fornece a renderização assíncrona onde os valores dentro da árvore podem ser atualizados fora da fase de renderização

Em renderização assíncrona, uma `Future` é usada como uma fonte de sinal. Com a API the sinal cru, qualquer valor pode ser usado como um fonte de sinal.

Por padrão, o Dioxus irá checar diferenças apenas nas sub árvores dos componentes com conteúdo dinâmico, automaticamente pulando o `diffing` para conteúdo estático.

## Como isso parece?

Seu componente hoje pode parece algo desse tipo:

```rust
fn Comp(cx: Scope) -> DomTree {
    let (title, set_title) = use_state(&cx, || "Title".to_string());
    cx.render(rsx!{
        input {
            value: title,
            onchange: move |new| set_title(new.value())
        }
    })
}
```

Este componente é bastante direto - a entrada atualiza seu próprio valor em cada alteração.

No entanto, cada nova chamado para `set_title` irá re-renderizar o componente. Se nós adicionarmos uma lista enorme, então cada vez que um novo título mudar causará uma nova atualização, o Dioxus irá precisar fazer um `diff` em toda a lista de novo, e de novo...

Isso é um **enorme** desperdício de processamento!

```rust
fn Comp(cx: Scope) -> DomTree {
    let (title, set_title) = use_state(&cx, || "Title".to_string());
    cx.render(rsx!{
        div {
            input {
                value: title,
                onchange: move |new| set_title(new.value())
            }
            ul {
                {0..10000.map(|f| rsx!{
                    li { "{f}" }
                })}
            }
        }
    })
}
```

Muitos desenvolvedores experientes em React dirão "isto é um planejamento ruim" - mas consideramos isso um poço de falhas ao invés de um poço de sucesso!

É por isso que os sinais existem - para colocar você numa direção de mais desempenho (e ergonômica).

Os sinais nos permitem vincular diretamente os valores ao seu local final no VirtualDOM.

Sempre que o valor do sinal for atualizado, o Dioxus irá checar apenas os nós DOM onde esse sinal for utilizado.

Os sinais são incorporados ao Dioxus, para que possamos vincular diretamente os atributos dos elementos às suas atualizações.

Podemos usar sinais para gerar uma ligação bidirecional entre os dados e a caixa de entradas. Nossa entrada de texto agora é apenas um componente de duas linhas!

```rust
fn Comp(cx: Scope) -> DomTree {
    let mut title = use_signal(&cx, || String::from("Title"));
    cx.render(rsx!(input { value: title }))
}
```

Para um exemplo um pouco mais interessante, este componente calcula a soma entre dois números, mas ignora totalmente o processo de diferenciação (`diffing`).

```rust
fn Calculator(cx: Scope) -> DomTree {
    let mut a = use_signal(&cx, || 0);
    let mut b = use_signal(&cx, || 0);
    let mut c = a + b;
    rsx! {
        input { value: a }
        input { value: b }
        p { "a + b = {c}" }
    }
}
```

Você percebe como podemos usar operações embutidas em sinais?

Nos bastidores, criamos um novo sinal derivado que depende de `a` e `b`.

Sempre que `a` ou `b` forem atualizados, então `c` será atualizado.

Se precisarmos criar um novo sinal derivado que seja mais complexo do que uma operação básica (`std::ops`), podemos encadear os sinais ou combiná-los:

```rust
let mut a = use_signal(&cx, || 0);
let mut b = use_signal(&cx, || 0);

// Chain signals together using the `with` method
let c = a.with(b).map(|(a, b)| *a + *b);
```

## `Deref` e `DerefMut`

Se alguma vez precisarmos obter o valor de um sinal, podemos simplesmente "des-referencia-lo" (`Deref`).

```rust
let mut a = use_signal(&cx, || 0);
let c = *a + *b;
```

Chamar `deref` ou `deref_mut` é realmente mais complexo do que parece.

Quando um valor é desreferenciado, você está essencialmente dizendo ao Dioxus que _este_ elemento _precisa_ ser inscrito no sinal.

Se um sinal for desreferenciado fora de um elemento, todo o componente será inscrito e a vantagem de pular a diferenciação será perdida.

O Dioxus lançará um erro no console quando isso acontecer, informando que você está usando o `Signal` de forma errada, mas seu componente continuará funcionando.

## Sinais Globais

Às vezes, você deseja que um sinal se propague em seu aplicativo, seja por meio de parentes distantes ou por meio de componentes profundamente aninhados.

Nesses casos, usamos o `Dirac`: o kit de ferramentas de gerenciamento de estado de primeira classe do Dioxus. Os "átomos" de `Dirac` implementam automaticamente a API `Signal`.

Este componente vinculará o elemento de entrada ao átomo `TITLE`.

```rust
const TITLE: Atom<String> = || "".to_string();
const Provider: Component = |cx|{
    let title = use_signal(&cx, &TITLE);
    rsx!(cx, input { value: title })
};
```

Se usarmos o átomo `TITLE` em outro componente, podemos fazer com que as atualizações fluam entre os componentes sem chamar o render ou `diff` nas árvores de componentes:

```rust
const Receiver: Component = |cx|{
    let title = use_signal(&cx, &TITLE);
    log::info!("This will only be called once!");
    rsx!(cx,
        div {
            h1 { "{title}" }
            div {}
            footer {}
        }
    )
};
```

Dioxus sabe que o sinal `title` do receptor é usado apenas no nó de texto, e pula completamente a diferenciação do Receptor, sabendo que pode atualizar _apenas_ o nó de texto.

Se você criar um aplicativo complexo em cima do `Dirac`, provavelmente notará que muitos de seus componentes simplesmente não serão diferenciados de nenhuma forma.

Por exemplo, nosso componente `Receiver` nunca será diferenciado depois de montado!

## Sinais e Iteradores

Às vezes você quer usar uma coleção de itens. Com o `Signals`, você pode ignorar a diferenciação de coleções - uma técnica muito poderosa para evitar a re-renderização em coleções grandes.

Por padrão, o Dioxus é limitado quando você usa `iter`/`map`. Com o componente `For`, você pode fornecer um iterador e uma função para o iterador mapear.

Dioxus entende automaticamente como usar seus sinais quando misturados com iteradores através de `Deref`/`DerefMut`.

Isso permite mapear coleções com eficiência, evitando a re-renderização de listas.

Em essência, os sinais funcionam como uma dica para o Dioxus sobre como evitar verificações e renderizações desnecessárias, tornando seu aplicativo mais rápido.

```rust
const DICT: AtomFamily<String, String> = |_| {};
const List: Component = |cx|{
    let dict = use_signal(&cx, &DICT);
    cx.render(rsx!(
        ul {
            For { each: dict, map: |k, v| rsx!( li { "{v}" }) }
        }
    ))
};
```

## Sinais Remotos

Os aplicativos que usam sinais desfrutarão de um híbrido agradável de SSR e Renderização por Cliente.

```rust

```

## Como isso funciona?

Os sinais usam internamente a infraestrutura de renderização assíncrona do Dioxus para realizar atualizações fora da árvore.
