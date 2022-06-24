# Liveview do Dioxus

`Liveview` é uma configuração onde um servidor e um cliente trabalham juntos para renderizar um aplicativo Dioxus. O `Liveview` [monomorfiza](https://rustc-dev-guide.rust-lang.org/backend/monomorph.html) um aplicativo da Web, eliminando a necessidade de APIs específicas de front-end.

Esta é uma alternativa amigável ao desenvolvedor para a JAM-stack (Javascript + API + Markdown), combinando a compatibilidade Wasm e o desempenho assíncrono do Rust.

## Porquê LiveView?

### Sem necessidade de APIs!

Como o Liveview combina o servidor e o cliente, você achará as APIs dedicadas desnecessárias.

Você ainda desejará implementar um serviço de busca de dados para aplicativos `LiveView`, mas isso pode ser implementado como uma `crate` e compartilhado entre aplicativos.

Essa abordagem foi projetada para permitir que você modele seus requisitos de dados sem precisar manter uma API com versão pública.

Você pode encontrar mais informações sobre modelagem e busca de dados para `LiveApps` no "Livro de Padrões Dioxus".

### Facilidade de Implantação

Não há problemas para implantar aplicativos `LiveView` - basta fazer o upload do binário para o provedor de hospedagem de sua escolha.

Simplesmente não há necessidade de lidar com APIs intermediárias. O front-end e o back-end de um aplicativo são fortemente acoplados um ao outro, o que significa que o back-end e o front-end estarão sempre atualizados.

Isso é especialmente poderoso para equipes pequenas, onde iteração rápida, protótipos exclusivos com versão e implantações simples são importantes. À medida que seu aplicativo cresce, o Dioxus crescerá com você, servindo até 100.000 RPS (graças ao desempenho assíncrono do Rust).

### Poder de Renderização do Servidor combinado com a natureza reativa do JavaScript

Os aplicativos `LiveView` são apoiados por um servidor e aprimorados com WebAssembly. Isso elimina completamente a necessidade de escrever Javascript para adicionar conteúdo dinâmico às páginas da web.

Os aplicativos são escritos em apenas **uma** linguagem e exigem essencialmente 0 conhecimento de sistemas de compilação, transpilação ou versões ECMAScript. O suporte mínimo do navegador é garantido para cobrir 95% dos usuários, e o Dioxus-CLI pode ser configurado para gerar polyfills para ter ainda mais suporte.

### Suporte com LiveHost por DioxusLabs

O Dioxus-CLI vem pronto para uso com serviços premium Dioxus-Labs, como LiveHost. Basta vincular seu repositório git ao Dioxus LiveHost e começar a implantar seu aplicativo Dioxus fullstack hoje. O LiveHost suporta:

- Fronted/backend com versões com links exclusivos
- CI/CD integrado
- Backends de Banco de Dados gerenciado e conectável
- Suporte sem servidor para latência mínima
- Análise do site
- Otimizado para Lighthouse
- Suporte no local (consulte os termos de licença)

Dioxus LiveHost é um serviço de gerenciamento Dioxus LiveApp com todos os recursos suportados em um único binário. Para OSS, permitimos o uso e a implantação gratuitos do LiveHost em seu provedor de hospedagem favorito.
