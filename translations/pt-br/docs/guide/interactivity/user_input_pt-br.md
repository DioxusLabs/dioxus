# Entrada do usuário e componentes controlados

Manipular a entrada do usuário é uma das coisas mais comuns que seu aplicativo fará, mas _pode_ ser complicado.

O paradigma reativo e os modelos de fluxo de dados unidirecional foram todos derivados para resolver problemas que podem surgir em torno do manuseio de ações do usuário. Esta seção ensinará a você dois padrões eficazes para lidar com a ação do usuário: entradas controladas e não controladas.

## Entradas Controladas

A abordagem mais comum para lidar com entradas de elementos é por meio de entradas "controladas". Com esse padrão, direcionamos o valor da entrada de nosso estado, enquanto atualizamos simultaneamente nosso estado a partir de novos valores.

Essa é a forma mais comum de manipulação de entrada e é amplamente usada porque evita que a interface do usuário fique fora de sincronia com seu estado local.

```rust
let name = use_state(&cx, || "bob".to_string());

cx.render(rsx!{
    input {
        value: "{name}",
        oninput: move |evt| name.set(evt.value.clone()),
    }
})
```

Por que não apenas "vincular" como em outros frameworks?

Em alguns casos, você não quer que o valor inserido corresponda ao que é renderizado na tela. Digamos que queremos implementar uma entrada que converta a entrada para maiúscula quando a entrada corresponder a um determinado valor. Com a vinculação, somos forçados a compartilhar o mesmo valor de entrada e estado. Ao manipular explicitamente o caso oninput, temos a oportunidade de definir um _novo_ valor.

```rust
let name = use_state(&cx, || "bob".to_string());

cx.render(rsx!{
    input {
        value: "{name}",
        oninput: move |evt| {
            if evt.value == "tim" {
                name.set("TIM");
            }
        },
    }
})
```

## Vinculando (Binding)

> ! Nota - a vinculação não está implementada no Dioxus. Esta seção representa um recurso em andamento.

Como o padrão acima é muito comum, temos um atributo adicional chamado "bind" que é essencialmente uma abreviação para nosso código acima.

O `Bind` apenas conecta um oninput a um `UseState` e é implementado através do sistema de sinal.

```rust
let name = use_state(&cx, || "bob".to_string());

cx.render(rsx!{
    input { bind: name }
})
```

## Entradas não controladas

Ao trabalhar com grandes conjuntos de entradas, você pode se cansar rapidamente de criar um `use_state` para cada valor. Além disso, o padrão de um `use_state` por interação pode se deteriorar quando você precisar ter um número flexível de entradas. Nesses casos, usamos entradas "não controladas". Aqui, não direcionamos o valor da entrada do `use_state`, optando por deixá-lo em um estado "não controlado".

Essa abordagem pode ser mais eficiente e flexível, mas mais propensa a inconsistências da interface do usuário do que sua contraparte controlada.

Para usar o padrão "não controlado", simplesmente omitimos a configuração do valor da entrada. Em vez disso, podemos reagir à mudança diretamente na própria entrada ou em um elemento de formulário mais alto na árvore.

Para este exemplo, não anexamos nenhum identificador `use_state` nos rótulos. Em vez disso, simplesmente anexamos um manipulador `oninput` ao elemento do formulário. Isso será executado toda vez que qualquer uma das entradas filhas for alterada, permitindo que executemos tarefas como validação de formulário.

```rust
form {
    oninput: move |evt| {
        if !validate_input(evt.values) {
            // display what errors validation resulted in
        }
    },
    input { name: "name", }
    input { name: "age", }
    input { name: "date", }
}
```
