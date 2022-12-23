# Aplicativo de área de trabalho

Crie um aplicativo de desktop nativo autônomo que tenha a mesma aparência em todos os sistemas operacionais.

Os aplicativos criados com o Dioxus geralmente têm menos de 5 MB de tamanho e usam os recursos existentes do sistema, para que não consumam quantidades extremas de RAM ou memória.

Exemplos:

- [Explorador de arquivos](https://github.com/DioxusLabs/example-projects/blob/master/file-explorer)
- [Scanner WiFi](https://github.com/DioxusLabs/example-projects/blob/master/wifi-scanner)

[![Exemplo do Explorador de Arquivos](https://raw.githubusercontent.com/DioxusLabs/example-projects/master/file-explorer/image.png)](https://github.com/DioxusLabs/example-projects/tree /master/file-explorer)

## Suporte

O desktop é uma plataforma poderosa para o Dioxus, mas atualmente é limitado em capacidade quando comparado à plataforma Web. Atualmente, os aplicativos de desktop são renderizados com a biblioteca WebView da própria plataforma, mas seu código Rust está sendo executado nativamente em um _thread_ nativo. Isso significa que as APIs do navegador _não_ estão disponíveis, portanto, renderizar WebGL, Canvas, etc. não é tão fácil quanto a Web. No entanto, as APIs do sistema nativo _são_ acessíveis, portanto, streaming, WebSockets, sistema de arquivos, etc., são todas APIs viáveis. No futuro, planejamos migrar para um renderizador DOM personalizado baseado em webrenderer com integrações WGPU.

O Dioxus Desktop é construído a partir do [Tauri](https://tauri.app/). No momento, não há abstrações do Dioxus sobre atalhos de teclado, barra de menus, manuseio, etc., então você deve aproveitar o Tauri - principalmente [Wry](http://github.com/tauri-apps/wry/) e [ Tao](http://github.com/tauri-apps/tao)) diretamente.

## Criando um projeto

Crie uma nova caixa:

```shell
cargo new --bin demo
cd demo
```

Adicione o Dioxus com o recurso `desktop` (isso irá editar o `Cargo.toml`):

```shell
cargo add dioxus
cargo add dioxus-desktop
```

Edite seu `main.rs`:

```rust
{{#include ../../../examples/hello_world_desktop.rs:all}}
```
