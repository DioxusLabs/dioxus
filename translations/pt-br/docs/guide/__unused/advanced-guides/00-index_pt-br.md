# Tópicos de Base

Nesse capítulo nós iremos cobrir alguns dos tópicos sobre como o Dioxus funciona e como melhor aproveitar suas funções para construir lindos aplicativos reativos.

Em um nível mais superficial, o Dioxus é simplesmente um framework feito em Rust para _declarar_ interfaces de usuário e _reagir_ de acordo com as mudanças.

1. Nós declaramos o que queremos que nossa interface pareça perante um estado usando lógica e controle de fluxo em Rust.
2. Nós declaramos como queremos que o estado mude quando o usuário disparar um evento.

## Interfaces Declarativas

Dioxus é um framework _declarativo_. Isso significa que ao contrário de escrever chamadas manuais para "criar um elemento" e "ajustar um elemento de fundo", nós simplesmente _declaramos_ o que queremos que o elemento pareça e o Dioxus se encarrega de calcular as diferenças com o estado anterior.

Vamos fingir que nós queremos controlar uma semáforo - ele tem as cores vermelho, amarelo e verde como opções.

Usando um método imperativo, nós teríamos que declarar manualmente cada um dos elementos e então manipular para o semáforo avançar de estado.

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

A partir do momento que a interface cresce, a lógica para manter cada elemento no seu devido estado também cresce exponencialmente. Isso pode virar muito trabalho e acarretar em interfaces fora de sincronia prejudicando a experiência do usuário.

Ao contrário, com o Dioxus, nós _declaramos_ o que queremos que a interface faça:

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

Lembre-se: este conceito não é novidade! Muitos frameworks são declarativos - como o React sendo o mais popular deles. Frameworks declarativos tendem a ser mais apreciados de trabalhar que os imperativos.

Aqui temos algumas leituras sobre interfaces declarativos em React:

- [https://stackoverflow.com/questions/33655534/difference-between-declarative-and-imperative-in-react-js](https://stackoverflow.com/questions/33655534/difference-between-declarative-and-imperative-in-react-js)

- [https://medium.com/@myung.kim287/declarative-vs-imperative-251ce99c6c44](https://medium.com/@myung.kim287/declarative-vs-imperative-251ce99c6c44)
