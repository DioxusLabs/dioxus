# Introdução

Esta seção irá ajudá-lo a configurar seu projeto Dioxus!

## Pré-requisitos

### Editor

O Dioxus se integra muito bem com o [plugin Rust-Analyzer LSP](https://rust-analyzer.github.io) que fornecerá realce de sintaxe apropriado, navegação de código, _folding_ e muito mais.

### Rust

Vá para [https://rust-lang.org](http://rust-lang.org) e instale o compilador Rust.

É altamente recomendável ler o [livro oficial do Rust](https://doc.rust-lang.org/book/ch01-00-getting-started.html) _completamente_. No entanto, nossa esperança é que um aplicativo Dioxus possa servir como um ótimo primeiro projeto Rust. Com Dioxus, você aprenderá sobre:

- Manipulação de erros
- Estruturas, Funções, Enums
- Closures
- Macros

Nós empenhamos muito cuidado para tornar a sintaxe do Dioxus familiar e fácil de entender, para que você não precise de conhecimento profundo sobre _async_, _lifetime_ ou _smart pointers_ até que você realmente comece a criar aplicativos Dioxus complexos.

### Dependências Específicas da Plataforma

#### Windows

Os aplicativos da área de trabalho do Windows dependem do WebView2 – uma biblioteca que deve ser instalada em todas as distribuições modernas do Windows. Se você tiver o Edge instalado, o Dioxus funcionará bem. Se você _não_ tiver o Webview2, [você poderá instalá-lo pela Microsoft](https://developer.microsoft.com/en-us/microsoft-edge/webview2/). MS oferece 3 opções:

1. Um pequeno _bootstrapper_ "evergreen" que buscará um instalador do CDN da Microsoft
2. Um pequeno _instalador_ que buscará o Webview2 do CDN da Microsoft
3. Uma versão vinculada estaticamente do Webview2 em seu binário final para usuários offline

Para fins de desenvolvimento, use a Opção 1.

#### Linux

Os aplicativos Webview Linux requerem WebkitGtk. Ao distribuir, isso pode ser parte de sua árvore de dependência em seu `.rpm` ou `.deb`. No entanto, é muito provável que seus usuários já tenham o WebkitGtk.

```bash
sudo apt install libwebkit2gtk-4.0-dev libgtk-3-dev libappindicator3-dev
```

Ao usar o Debian/bullseye, o `libappindicator3-dev` não está mais disponível, pois foi substituído por `libayatana-appindicator3-dev`.

```bash
# on Debian/bullseye use:
sudo apt install libwebkit2gtk-4.0-dev libgtk-3-dev libayatana-appindicator3-dev
```

Se você tiver problemas, certifique-se de ter todo o básico instalado, conforme descrito nos [documentos do Tauri](https://tauri.studio/v1/guides/getting-started/prerequisites#setting-up-linux).

#### Mac OS

Atualmente – tudo para macOS está integrado! No entanto, você pode encontrar um problema se estiver usando o Rust **nightly** devido a alguns problemas de permissão em nossa dependência do `Tao` (que foram resolvidos, mas não publicados).

### Extensões do Cargo Sugeridas

Se você quiser manter seu fluxo de trabalho tradicional como `npm install XXX` para adicionar pacotes, você pode querer instalar o `cargo-edit` e algumas outras extensões `cargo` interessantes:

- [cargo-expand](https://github.com/dtolnay/cargo-expand) para expandir chamadas de macro
- [árvore de carga](https://doc.rust-lang.org/cargo/commands/cargo-tree.html) – um comando de carga integrado que permite inspecionar sua árvore de dependência

## Guias de configuração

Dioxus suporta múltiplas plataformas. Dependendo do que você quer, a configuração é um pouco diferente.

- [Web](web.md): rodando no navegador usando WASM
- [Server Side Rendering](ssr.md): renderiza Dioxus HTML como texto
- [Desktop](desktop.md): um aplicativo autônomo usando o webview
- [Celular](mobile.md)
- [Terminal UI](tui.md): interface gráfica baseada em texto do terminal
