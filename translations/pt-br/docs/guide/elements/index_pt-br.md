# Tópicos Principais

Neste capítulo, abordaremos alguns tópicos principais sobre como o Dioxus funciona e como aproveitar melhor os recursos para criar um aplicativo bonito e reativo.

Em um nível muito alto, o Dioxus é simplesmente um framework feito em Rust para _declarar_ interfaces de usuário e _reagir_ a mudanças.

1. Declaramos como queremos que nossa interface de usuário se pareça com um estado usando lógica e fluxo de controle baseados em Rust.
2. Declaramos como queremos que nosso estado mude quando o usuário acionar um evento.

## Interface Declarativa

Dioxus é uma estrutura _declarativa_. Isso significa que, em vez de escrever manualmente as chamadas para "criar elemento" e "definir o plano de fundo do elemento para vermelho", nós simplesmente _declaramos_ como queremos que o elemento se pareça e deixamos o Dioxus lidar com as diferenças.

Vamos fingir que temos um semáforo que precisamos controlar - ele tem um estado de cor com vermelho, amarelo e verde como opções.

Usando uma abordagem imperativa, teríamos que declarar manualmente cada elemento e depois os manipuladores para avançar o semáforo.

```rust
let container = Container::new();

let green_light = Light::new().color("green").enabled(true);
let yellow_light = Light::new().color("yellow").enabled(false);
let red_light = Light::new().color("red").enabled(false);
container.push(green_light);
container.push(yellow_light);
container.push(red_light);

container.set_onclick(move |_| {
    if red_light.enabled() {
        red_light.set_enabled(false);
        green_light.set_enabled(true);
    } else if yellow_light.enabled() {
        yellow_light.set_enabled(false);
        red_light.set_enabled(true);
    } else if green_light.enabled() {
        green_light.set_enabled(false);
        yellow_light.set_enabled(true);
    }
});
```

À medida que a UI cresce em escala, nossa lógica para manter cada elemento no estado adequado cresceria exponencialmente. Isso pode se tornar muito complicado e levar a interfaces de usuário fora de sincronia que prejudicam a experiência do usuário.

Em vez disso, com o Dioxus, _declaramos_ como queremos que nossa interface do usuário se pareça:

```rust
let mut state = use_state(&cx, || "red");

cx.render(rsx!(
    Container {
        Light { color: "red", enabled: state == "red", }
        Light { color: "yellow", enabled: state == "yellow", }
        Light { color: "green", enabled: state == "green", }

        onclick: move |_| {
            state.set(match *state {
                "green" => "yellow",
                "yellow" => "red",
                "red" => "green",
            })
        }
    }
))
```

Lembre-se: este conceito não é novo! Muitos frameworks são declarativos - sendo o React o mais popular. Os frameworks declarativos tendem a ser muito mais agradáveis de se trabalhar do que os frameworks imperativos.

Aqui estão algumas leituras sobre como declarar UI no React:

- [Diferença entre declarativo e imperativo no React.js](https://stackoverflow.com/questions/33655534/difference-between-declarative-and-imperative-in-react-js), um thread do StackOverflow

- [Declarative vs Imperative](https://medium.com/@myung.kim287/declarative-vs-imperative-251ce99c6c44), uma postagem no blog de Myung Kim
