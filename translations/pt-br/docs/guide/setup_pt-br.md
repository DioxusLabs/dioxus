# Visão Geral

Neste capítulo nós vamos configurar uma pequena aplicação para Desktop.

Nós vamos aprenderÇ

- Instalando a linguagem de programação Rust
- Instalando a CLI Dioxus para empacotamento e desenvolvimento
- Extensões Cargo sugeridas

Para guias específicos de cada plataforma veja: [Guia de Plataforma Específica](/reference/platforms/index.md).

# Configurando o Dioxus

Dioxus requer algumas coisas importantes para funcionar:

- O [Compilador Rust](https://www.rust-lang.org) e suas ferramentas associadas
- Um editor de código de sua escolha, idealmente configurado com o [Rust-Analyzer LSP plugin](https://rust-analyzer.github.io)

Dioxus se integra muito bem com o plugin Rust-Analyzer o qual prover destaque de sintaxe, navegação de código, exibir/ocultar código e mais.

## Instalando o Rust

Vá até [https://rust-lang.org/pt-BR](http://rust-lang.org/pt-BR) e instale o compilador Rust.

Uma vez instalado, tenha certeza de ter instalado `wasm32-unknown-unknown` como ponto de compilação se você está planejando implantar seu aplicativo na Web.

```
rustup target add wasm32-unknown-unknown
```

## Dependências de Plataforma Específica

Se você está usando um sistema operacional moderno você não deve precisar de nenhuma configuração de WebView para aplicativos Desktop. No entanto, se você estiver usando uma versão antigo do Windows ou Linux sem suporte padrão para o motor de renderização Web, você vai precisar instalar algumas dependências adicionais.

### Windows

Aplicativos Desktop de Windows dependem do `WebView2` - uma biblioteca que pode ser instalada em todas as distribuições Windows mais modernas. Se você já tem o Edge instalado, então o Dioxus irá funcionar sem problemas. Se você _não_ tiver o WebView2 [então você pode instalar através da Microsoft](https://developer.microsoft.com/en-us/microsoft-edge/webview2/). MS prover 3 opções:

1. Um pequeno instalador _configurado "sempre-pronto"_ que retornará o instalar do CDN da Microsoft
2. Um pequeno _instalador_ que retornará apenas o WebView2 direto do CDN da Microsoft
3. Um binário executável que irá anexar estaticamente o WebView no sistema para usuário offline

Para fins de desenvolvimento, use a opção 1.

### Linux

Aplicativos WebView no Linux requer o `WebkitGtk`. Quando distribuindo, isto pode fazer parte de sua árvore de dependências no seu `.rpm` ou `.deb`. No enttanto, é muito provável que seus usuários já tenham o WebkitGtb instalado.

```
sudo apt install libwebkit2gtk-4.0-dev libgtk-3-dev libappindicator3-dev
```

Se estiver usando Debian/bullseye o `libappindicator3-dev` não está mais disponível, pois foi substituído pelo `libayatana-appindicator3-dev`.

```
# No Debian/bullseye use:
sudo apt install libwebkit2gtk-4.0-dev libgtk-3-dev libayatana-appindicator3-dev
```

Se você estiver com dificuldades, tenha certeza de ter instalado todos os items básicos, como destacado em [Tauri docs](https://tauri.studio/v1/guides/getting-started/prerequisites#setting-up-linux).

### macOS

Atualmente - tudo já está instalado no macOS! No entanto, você pode enfrentar algum problema se você estiver usando uma versão nightly do Rust devido à alguns problemas de permissões em nossa dependência `Tao` (que já foi resolvido, mas não publicado).

## Dioxus-CLI para servidor de desenvolvimento, empacotamento, etc.

Nós também recomendamos instalar o Dioxus CLI. O Dioxus CLI automatiza o empacotamento para várias plataformas e se integra aos simuladores, servidores de desenvolvimento e implantação de aplicativo. Para instalar o CLI você precisará do `Cargo` (que deve instalar automaticamente com o Rust):

```
$ cargo install dioxus-cli
```

Você pode atualizar o dioxus-cli a qualquer momento com:

```
$ cargo install --force dioxus-cli
```

Nós providenciamos essa ferramenta de primeira-mão para evitar que você tenha que executar código potencialmente não confiável toda vez que você adicionar uma nova `crate` no seu projeto - assim é por padrão no ecossistema NPM.

## Extenssões sugeridas

Se você quiser manter o estilo tradicional `npm install xxx` para adicionar pacotes, você pode querer instalar o `cargo-edit` e algumas outras extensões `cargp`:

- [cargo edit](https://github.com/killercup/cargo-edit) para adicionar dependências direto da CLI
- [cargo-expand](https://github.com/dtolnay/cargo-expand) para expandir as `macro` chamadas
- [cargo tree](https://doc.rust-lang.org/cargo/commands/cargo-tree.html) - um comando integrado ao cargo que permite você inspecionar sua árvore de dependências

Pronto, é isso! Nós não precisaremos tocar em `NPM/WebPack/Babel/Parcel`, etc. No entanto, você _pode_ configurar o seu aplicativo para usar o `WebPack` com o [tradicional WASM-pack](https://rustwasm.github.io/wasm-pack/book/tutorials/hybrid-applications-with-webpack/using-your-library.html).

## Conhecimento Rust

Com Rust, coisas como comparativos, testagem e documentação estão inclusos na linguagem. Nós recomendamos fortemente que você leia o livro oficial do Rust por complete. No entanto, esperamos que o Dioxus possa lhe oferecer como seu primeiro grande projeto em Rust. Com o Dioxus você aprenderá sobre:

- Tratamento de Erros
- Structs, Funções, Enums
- Closures
- Macros

Nós tivemos muito zelo em tornar a sintaxe do Dioxus familiar e fácil de entender, então você não precisará de um conhecimento profundo sobre assincronia, "lifetimes" ou ponteiras inteligentes até você precisar construir aplicativos complexos.

Nós encorajamos explorar os guidas para mais informações sobre como trabalhar com as ferramentas integradas:

- [Testagem](Testing.md)
- [Documentação](Documentation.md)
- [Comparativos](Benchmarking.md)
- [Construindo](Building.md)
- [Módulos](Modules.md)
- [Crates](Crates.md)
