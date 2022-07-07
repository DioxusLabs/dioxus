# Passando Filhos e Atributos

Muitas vezes, você precisará envolver algumas funcionalidades importantes _ao redor_ de seu estado, não diretamente aninhadas _dentro_ de outro componente.

Nesses casos, você pode passar elementos e atributos para um componente e deixar que o componente os posicione adequadamente.

Neste capítulo, você aprenderá sobre:

- Passando elementos para componentes
- Passando atributos para componentes

## Casos de Uso

Digamos que você esteja criando uma interface de usuário e queira transformar parte dela em um link clicável para outro site. Você normalmente começaria com a tag HTML `<a>`, assim:

```rust
rsx!(
    a {
        href: "https://google.com"
        "Link to google"
    }
)
```

Mas, e se quisermos estilizar nossa tag `<a>`? Ou envolvê-lo com algum ícone auxiliar? Poderíamos abstrair nosso `RSX` em seu próprio componente:

```rust
#[derive(Props)]
struct ClickableProps<'a> {
    href: &'a str,
    title: &'a str
}

fn Clickable<'a>(cx: Scope<'a, ClickableProps<'a>>) -> Element {
    cx.render(rsx!(
        a {
            href: "{cx.props.href}"
            "{cx.props.title}"
        }
    ))
}
```

And then use it in our code like so:

```rust
rsx!(
    Clickable {
        href: "https://google.com"
        title: "Link to Google"
    }
)
```

Digamos que não queremos apenas que o texto seja clicável, mas que outro elemento, como uma imagem, seja clicável. Como implementamos isso?

## Passando Filhos

Se quisermos passar uma imagem para nosso componente, podemos apenas ajustar nossos `props` e componentes para permitir qualquer `Element`.

```rust
#[derive(Props)]
struct ClickableProps<'a> {
    href: &'a str,
    body: Element<'a>
}

fn Clickable<'a>(cx: Scope<'a, ClickableProps<'a>>) -> Element {
    cx.render(rsx!(
        a {
            href: "{cx.props.href}",
            &cx.props.body
        }
    ))
}
```

Então, no lugar da chamada, podemos renderizar alguns nós e passá-los:

```rust
rsx!(
    Clickable {
        href: "https://google.com"
        body: cx.render(rsx!(
            img { src: "https://www.google.com/logos/doodles/..." }
        ))
    }
)
```

## Conversão Automática de `Children`

Esse padrão pode se tornar tedioso em alguns casos, então o Dioxus realmente realiza uma conversão implícita de qualquer chamada `rsx` dentro de componentes em `Elements` no campo `children`.

Isso significa que você deve declarar explicitamente se um componente pode receber filhos.

```rust
#[derive(Props)]
struct ClickableProps<'a> {
    href: &'a str,
    children: Element<'a>
}

fn Clickable<'a>(cx: Scope<'a, ClickableProps<'a>>) -> Element {
    cx.render(rsx!(
        a {
            href: "{cx.props.href}",
            &cx.props.children
        }
    ))
}
```

Agora, sempre que usarmos `Clickable` em outro componente, não precisamos chamar `render` em nós filhos - isso acontecerá automaticamente!

```rust
rsx!(
    Clickable {
        href: "https://google.com"
        img { src: "https://www.google.com/logos/doodles/...." }
    }
)
```

> Nota: Passar filhos para componentes interromperá qualquer memoização devido ao tempo de vida (`lifetime`) associado.

Embora tecnicamente permitido, é um antipadrão passar filhos mais de uma vez em um componente e provavelmente fará com que seu aplicativo falhe.

No entanto, como o `Element` é transparentemente ao `VNode`, podemos combinar com ele para extrair os próprios nós, caso estejamos esperando um formato específico:

```rust
fn clickable<'a>(cx: Scope<'a, ClickableProps<'a>>) -> Element {
    match cx.props.children {
        Some(VNode::Text(text)) => {
            // ...
        }
        _ => {
            // ...
        }
    }
}
```

Passar nós por `props` significa que eles são imutáveis. Se você precisar alterar nós passados por `props`, considere criar um novo nó em seu lugar que assuma seus atributos, filhos e ouvintes.

<!-- ## Passing attributes

In the cases where you need to pass arbitrary element properties into a component - say to add more functionality to the `<a>` tag, Dioxus will accept any quoted fields. This is similar to adding arbitrary fields to regular elements using quotes.

```rust

rsx!(
    Clickable {
        "class": "blue-button",
        "style": "background: red;"
    }
)

```

For a component to accept these attributes, you must add an `attributes` field to your component's properties. We can use the spread syntax to add these attributes to whatever nodes are in our component.

```rust
#[derive(Props)]
struct ClickableProps<'a> {
    attributes: Attributes<'a>
}

fn clickable(cx: Scope<ClickableProps<'a>>) -> Element {
    cx.render(rsx!(
        a {
            ..cx.props.attributes,
            "Any link, anywhere"
        }
    ))
}
```

The quoted escapes are a great way to make your components more flexible.
 -->

## Passando handlers

O Dioxus também fornece algumas conversões implícitas de atributos de ouvinte em um `EventHandler` para qualquer campo em componentes que comecem com `on`. Por exemplo, `onclick`, `onhover`, etc.

Para propriedades, devemos definir nossos campos `on` como um manipulador de eventos (`EventHandler`):

```rust
#[derive(Props)]
struct ClickableProps<'a> {
    onclick: EventHandler<'a, MouseEvent>
}

fn clickable<'a>(cx: Scope<'a, ClickableProps<'a>>) -> Element {
    cx.render(rsx!(
        a {
            onclick: move |evt| cx.props.onclick.call(evt)
        }
    ))
}
```

Então, podemos anexar um ouvinte no lugar da chamada:

```rust
rsx!(
    Clickable {
        onclick: move |_| log::info!("Clicked"),
    }
)
```

Atualmente, o Dioxus não suporta uma quantidade arbitrária de ouvintes - eles devem ser fortemente tipados em `Properties`. Se você precisar desse caso de uso, você pode passar um elemento com esses ouvintes ou mergulhar na API `NodeFactory`.

## Recapitulando

Neste capítulo, aprendemos:

- Como passar nós arbitrários pela árvore
- Como o campo `children` funciona nas propriedades do componente
- Como o campo `attributes` funciona nas propriedades do componente
- Como converter `listeners` em `EventHandlers` para componentes
- Como estender qualquer nó com atributos personalizados e filhos
