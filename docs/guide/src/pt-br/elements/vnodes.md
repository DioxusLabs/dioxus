# Declarando sua primeira UI com Elements

Cada interface de usuário que você já usou é apenas uma sinfonia de pequenos widgets trabalhando juntos para abstrair funções maiores e complexas. No Dioxus, chamamos esses pequenos widgets de "Elementos". Usando Componentes, você pode facilmente compor `Elements` em grupos maiores para formar estruturas ainda maiores: `Apps`.

Neste capítulo, abordaremos:

- Declarando nosso primeiro elemento
- Compondo elementos juntos
- Propriedades do elemento

## Declarando nosso primeiro elemento

Como o Dioxus é usado principalmente com renderizadores HTML/CSS, a "coleção" de elementos padrão é HTML. Desde que o recurso `html` não esteja desabilitado, podemos declarar `Elements` usando a macro `rsx!`:

```rust
rsx!(
    div {}
)
```

Como você pode esperar, podemos renderizar essa chamada usando Dioxus-SSR para produzir HTML válido:

```rust
dioxus::ssr::render_lazy(rsx!(
    div {}
))
```

Produces:

```html
<div></div>
```

Podemos construir qualquer tag HTML válida com o padrão `tag {}` e esperar que a estrutura HTML resultante se assemelhe à nossa declaração.

## Elementos de composição

Claro, precisamos de estruturas mais complexas para tornar nossos aplicativos realmente úteis! Assim como o HTML, a macro `rsx!` nos permite aninhar elementos um dentro do outro.

```rust
#use dioxus::prelude::*;
rsx!(
    div {
        h1 {}
        h2 {}
        p {}
    }
)
```

Como você pode esperar, o HTML gerado para essa estrutura seria semelhante a:

```html
<div>
  <h1></h1>
  <h2></h2>
  <p></p>
</div>
```

Com a configuração padrão, qualquer `Element` definido dentro do `dioxus-html` pode ser declarado desta forma. Para criar seus próprios novos elementos, consulte o Guia Avançado de `Custom Elements`.

## Elementos de texto

Dioxus também suporta um tipo especial de `Element`: `Text`. Elementos de texto não aceitam filhos, apenas uma string literal denotada por aspas duplas.

```rust
rsx! (
    "hello world"
)
```

Elementos de texto podem ser compostos dentro de outros elementos:

```rust
rsx! (
    div {
        h1 { "hello world" }
        p { "Some body content" }
    }
)
```

O texto também pode ser formatado com qualquer valor que implemente `Display`. Usamos a mesma sintaxe do Rust [format strings](https://www.rustnote.com/blog/format_strings.html) – que já será familiar para usuários de Python e JavaScript:

```rust
let name = "Bob";
rsx! ( "hello {name}" )
```

Infelizmente, você ainda não pode inserir expressões arbitrárias diretamente na string literal com Rust. Nos casos em que precisamos calcular um valor complexo, usaremos `format_args!` diretamente. Devido às especificidades do macro `rsx!` (que abordaremos mais tarde), nossa chamada para `format_args` deve estar entre colchetes.

```rust
rsx!( [format_args!("Hello {}", if enabled { "Jack" } else { "Bob" } )] )
```

Alternativamente, `&str` pode ser incluído diretamente, embora também deva estar entre colchetes:

```rust
rsx!( "Hello ",  [if enabled { "Jack" } else { "Bob" }] )
```

Isso é diferente da maneira do React de gerar marcação arbitrária, mas se encaixa no Rust idiomático.

Normalmente, com o Dioxus, você só vai querer computar suas substrings fora da chamada `rsx!` e aproveitar a formatação f-string:

```rust
let name = if enabled { "Jack" } else { "Bob" };
rsx! ( "hello {name}" )
```

## Atributos

Cada elemento em sua interface de usuário terá algum tipo de propriedade que o renderizador usará ao desenhar na tela. Eles podem informar ao renderizador se o componente deve ser oculto, qual deve ser sua cor de fundo ou fornecer um nome ou ID específico.

Para fazer isso, usamos a sintaxe familiar de estilo `struct` que o Rust fornece:

```rust
rsx!(
    div {
        hidden: "true",
        background_color: "blue",
        class: "card color-{mycolor}"
    }
)
```

Cada campo é definido como um método no elemento na caixa `dioxus-html`. Isso evita que você digite incorretamente um nome de campo e nos permite fornecer documentação em linha. Quando você precisa usar um campo não definido como um método, você tem duas opções:

1. registre um problema se o atributo _should_ estiver ativado
2. adicione um atributo personalizado em tempo real

Para usar atributos personalizados, basta colocar o nome do atributo entre aspas:

```rust
rsx!(
    div {
        "customAttr": "important data here"
    }
)
```

> Nota: o nome do atributo personalizado deve corresponder exatamente ao que você deseja que o renderizador produza. Todos os atributos definidos como métodos em `dioxus-html` seguem a convenção de nomenclatura snake_case. No entanto, eles traduzem internamente sua convenção snake_case para a convenção camelCase do HTML. Ao usar atributos personalizados, verifique se o nome do atributo **exatamente** corresponde ao que o renderizador está esperando.

Todos os atributos de elemento devem ocorrer _antes_ dos elementos filho. A macro `rsx!` lançará um erro se seus elementos filhos vierem antes de qualquer um de seus atributos. Se você não vir o erro, tente editar a configuração do Rust-Analyzer IDE para ignorar erros de macro. Esta é uma solução temporária porque o Rust-Analyzer atualmente lança _dois_ erros em vez de apenas aquele com o qual nos preocupamos.

```rust
// settings.json
{
  "rust-analyzer.diagnostics.disabled": [
    "macro-error"
  ],
}
```

## Ouvintes

Os ouvintes são um tipo especial de atributo que aceita apenas funções. Os ouvintes nos permitem anexar uma funcionalidade aos nossos elementos executando um encerramento sempre que o ouvinte especificado for acionado.

Abordaremos os ouvintes com mais profundidade no capítulo Adicionando Interatividade, mas, por enquanto, saiba que todo ouvinte começa com "on" e aceita encerramentos.

```rust
rsx!(
    div {
        onclick: move |_| log::debug!("div clicked!"),
    }
)
```

## Seguindo em Frente

Este capítulo apenas arranha a superfície de como os Elementos podem ser definidos.

Nós aprendemos:

- Os elementos são os blocos básicos de construção das interfaces de usuário
- Os elementos podem conter outros elementos
- Os elementos podem ser um contêiner nomeado ou texto
- Alguns elementos têm propriedades que o renderizador pode usar para desenhar a interface do usuário na tela

Em seguida, vamos compor Elements juntos usando lógica baseada em Rust.
