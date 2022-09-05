# Descrevendo a Interface do Usuário

Dioxus é uma estrutura _declarativa_. Isso significa que, em vez de dizer ao Dioxus o que fazer (por exemplo, "criar um elemento" ou "definir a cor para vermelho"), simplesmente _declaramos_ como queremos que a interface do usuário se pareça usando o RSX.

Você já viu um exemplo simples ou sintaxe `RSX` no aplicativo "hello world":

```rust
{{#include ../../../examples/hello_world_desktop.rs:component}}
```

Aqui, usamos a macro `rsx!` para _declarar_ que queremos um elemento `div`, contendo o texto `"Hello, world!"`. Dioxus pega o RSX e constrói uma interface do usuário a partir dele.

## Recursos do RSX

O RSX é muito semelhante ao HTML, pois descreve elementos com atributos e filhos. Aqui está um elemento `div` vazio no RSX, bem como o HTML resultante:

```rust
{{#include ../../../examples/rsx_overview.rs:empty}}
```

```html
<div></div>
```

### Filhos

Para adicionar filhos a um elemento, coloque-os dentro dos colchetes `{}`. Eles podem ser outros elementos ou texto. Por exemplo, você pode ter um elemento `ol` (lista ordenada), contendo 3 elementos `li` (item da lista), cada um dos quais contém algum texto:

```rust
{{#include ../../../examples/rsx_overview.rs:children}}
```

```html
<ol>
  <li>First Item</li>
  <li>Second Item</li>
  <li>Third Item</li>
</ol>
```

### Fragmentos

Você também pode "agrupar" elementos envolvendo-os em `Fragment {}`. Isso não criará nenhum elemento adicional.

> Nota: você também pode renderizar vários elementos no nível superior de `rsx!` e eles serão agrupados automaticamente – não há necessidade de um `Fragment {}` explícito lá.

```rust
{{#include ../../../examples/rsx_overview.rs:fragments}}
```

```html
<p>First Item</p>
<p>Second Item</p>
<span>a group</span>
<span>of three</span>
<span>items</span>
```

### Atributos

Os atributos também são especificados dentro dos colchetes `{}`, usando a sintaxe `name: value`. Você pode fornecer o valor como um literal no RSX:

```rust
{{#include ../../../examples/rsx_overview.rs:attributes}}
```

```html
<a
  href="https://www.youtube.com/watch?v=dQw4w9WgXcQ"
  class="primary_button"
  autofocus="true"
  >Log In</a
>
```

> Nota: Todos os atributos definidos em `dioxus-html` seguem a convenção de nomenclatura snake_case. Eles transformam seus nomes `snake_case` em atributos `camelCase` do HTML.

#### Atributos Personalizados

Dioxus tem um conjunto pré-configurado de atributos que você pode usar. O RSX é validado em tempo de compilação para garantir que você não especificou um atributo inválido. Se você quiser substituir esse comportamento por um nome de atributo personalizado, especifique o atributo entre aspas:

```rust
{{#include ../../../examples/rsx_overview.rs:custom_attributes}}
```

```html
<b customAttribute="value"> Rust is cool </b>
```

### Interpolação

Da mesma forma que você pode [formatar](https://doc.rust-lang.org/rust-by-example/hello/print/fmt.html) Rust _strings_, você também pode interpolar no texto RSX. Use `{variable}` para exibir o valor de uma variável em uma _string_, ou `{variable:?}` para usar a representação `Debug`:

```rust
{{#include ../../../examples/rsx_overview.rs:formatting}}
```

```html
<div class="country-es">
  Coordinates: (42, 0)
  <div>ES</div>
  <div>42</div>
</div>
```

### Expressões

Você pode incluir expressões Rust arbitrárias dentro do RSX, mas deve escapá-las entre colchetes `[]`:

```rust
{{#include ../../../examples/rsx_overview.rs:expression}}
```

```html
<span>DIOXUS</span>
```
