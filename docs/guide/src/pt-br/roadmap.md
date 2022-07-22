# Roteiro e Conjunto de Recursos

Este conjunto de recursos e roteiro podem ajudá-lo a decidir se o que a Dioxus pode fazer hoje funciona para você.

Se um recurso que você precisa não existe ou você deseja contribuir com projetos no roteiro, sinta-se à vontade para se envolver [juntando-se ao discord](https://discord.gg/XgGxMSkvUM).

Em geral, aqui está o status de cada plataforma:

- **Web**: Dioxus é uma ótima opção para aplicativos da web puros - especialmente para aplicativos CRUD/complexos. No entanto, ele não possui o ecossistema do React, então você pode estar perdendo uma biblioteca de componentes ou algum _hook_ útil.

- **SSR**: Dioxus é uma ótima opção para pré-renderização, hidratação e renderização de HTML em um endpoint da web. Esteja ciente: o `VirtualDom` não é (atualmente) `Send + Sync`.

- **Desktop**: você pode criar aplicativos de desktop de janela única muito competentes agora mesmo. No entanto, os aplicativos de várias janelas exigem suporte do núcleo do Dioxus que não estão prontos.

- **Celular**: o suporte móvel é muito recente. Você descobrirá as coisas à medida que avança e não há muitas `crates` de suporte para periféricos.

- **LiveView**: o suporte ao LiveView é muito recente. Você descobrirá as coisas à medida que avança. Felizmente, nada disso é muito difícil e qualquer trabalho poderá ser enviado `upstream` no Dioxus.

## Recursos

---

| Recurso                             | Situação | Descrição                                                                                        |
| ----------------------------------- | -------- | ------------------------------------------------------------------------------------------------ |
| Renderização Condicional            | ✅       | `if/then` para esconder/mostrar componente                                                       |
| Map, Iterador                       | ✅       | `map/filter/reduce` para produzir `rsx!`                                                         |
| Componentes Chaveados               | ✅       | comparação em `diff` com chaves                                                                  |
| Web                                 | ✅       | renderizador para navegadores Web                                                                |
| Desktop (WebView)                   | ✅       | renderizador para Desktop                                                                        |
| Estado Compartilhado (Context)      | ✅       | compartilha estados através de árvores                                                           |
| Hooks                               | ✅       | células de memoria nos componentes                                                               |
| SSR                                 | ✅       | renderiza diretamente para `string`                                                              |
| Componente Filho                    | ✅       | `cx.children()` como lista de nós                                                                |
| Componentes Sem Elementos           | ✅       | componentes que não renderizam elementos reais na DOM                                            |
| Fragmentos                          | ✅       | elementos múltiplos sem uma raiz real                                                            |
| Propriedades Manuais                | ✅       | passa manualmente `props` com a sintaxe de propagação (`spread syntax`)                          |
| Entradas Controladas                | ✅       | encapsulamento com estado em sobre entradas                                                      |
| Estilos CSS/Inline                  | ✅       | sintaxe para grupos de estilo/atributos em linha                                                 |
| Elementos Personalizados            | ✅       | define novos elementos primitivos                                                                |
| Suspensão                           | ✅       | programa futuras renderizações usando `future`/`Promise`                                         |
| Tratamento Integrado de Erros       | ✅       | trata erros graciosamente com a sintaxe `?`                                                      |
| NodeRef                             | ✅       | ganha acesso direto aos nós                                                                      |
| Re-hidratação                       | ✅       | pre-renderiza HTML para acelerar a primeira impressão na tela                                    |
| Renderização Livre de Gargalos      | ✅       | `diffs` grandes são segmentados sobre quadros para uma transição suave como seda                 |
| Efeitos                             | ✅       | executa efeitos após um componente ser enviado para a fila de renderização                       |
| Portais                             | 🛠        | renderiza nós fora da árvore tradicional de elementos (DOM)                                      |
| Agendamento Cooperativo             | 🛠        | prioriza eventos com mais importância sobre eventos menos importantes                            |
| Componentes de Servidor             | 🛠        | componentes híbridos para aplicativos de única página (SPA) e servidores                         |
| Divisão de Pacotes                  | 👀       | carrega o aplicativo assincronamente e eficientemente                                            |
| Componentes Tardios                 | 👀       | dinamicamente carrega os novos componentes assim que estiverem prontos enquanto a página carrega |
| Estado Global de 1ª Classe          | ✅       | `redux/recoil/mobx` sobre o `context`                                                            |
| Execução Nativa                     | ✅       | execução como um binário portátil sem um `runtime` (Node)                                        |
| Sub-Árvore de Memoization           | ✅       | pula o `diffing` em sub-árvores de elementos estáticos                                           |
| Modelos de Alta Eficiência          | 🛠        | chamadas `rsx!` são traduzidas para modelos sobre a `DOM`                                        |
| Garantia de Correção por Compilador | ✅       | avisa sobre erros em esquemas de modelos inválidos antes do final da compilação                  |
| Motor de Heurística                 | ✅       | rastreia componentes na memória para minimizar alocações futures                                 |
| Controle Preciso de Reatividade     | 👀       | pula o `diffing` para ter controle preciso das atualizações de tela                              |

- ✅ = implementado e funcionando
- 🛠 = sendo trabalhado ativamente
- 👀 = ainda não implementado ou sendo trabalhado

## Roteiro

Esses recursos estão planejados para o futuro do Dioxus:

### Essencial

- [x] Liberação do Núcleo Dioxus
- [x] Atualizar a documentação para incluir mais teoria e ser mais abrangente
- [ ] Suporte para modelos HTML para manipulação de DOM ultrarrápida
- [ ] Suporte para vários renderizadores para o mesmo `virtualdom` (subárvores)
- [ ] Suporte para `ThreadSafe` (`Send` + `Sync`)
- [ ] Suporte para `Portals`

### SSR

- [x] Suporte SSR + Hidratação
- [ ] Suporte integrado de suspensão para SSR

### Desktop

- [ ] Gerenciamento de janela declarativa
- [ ] Modelos para construção/agregação
- [ ] Renderizador totalmente nativo
- [ ] Acesso ao contexto Canvas/WebGL nativamente

### Móvel

- [ ] Biblioteca padrão móvel
  - [ ] GPS
  - [ ] Câmera
  - [ ] Sistema de Arquivo
  - [ ] Biometria
  - [ ] Wi-fi
  - [ ] Bluetooth
  - [ ] Notificações
  - [ ] Prancheta (_Clipboard_)
- [ ] Animações
- [ ] Renderizador nativo

### Empacotamento (CLI)

- [x] Tradução de HTML para RSX
- [x] Servidor de desenvolvimento
- [x] Recarregamento em tempo-real (_hot-reload_)
- [x] Tradução de JSX para RSX
- [ ] Substituição de módulos em tempo-real (_hot-modules_)
- [ ] Divisão de código
- [ ] Acervo de macros
- [ ] _Pipeline_ CSS
- [ ] _Pipeline_ de imagens

### Hooks Essenciais

- [x] Roteador
- [x] Gerenciamento de estado global
- [ ] Redimensionar o observador

## Trabalho em Progresso

### Ferramenta de Construção

Atualmente, estamos trabalhando em nossa própria ferramenta de compilação chamada [Dioxus CLI](https://github.com/DioxusLabs/cli) que suportará:

- uma TUI interativa
- reconfiguração em tempo real
- recarga de CSS em tempo-real
- ligação de dados bidirecional entre o navegador e o código-fonte
- um interpretador para `rsx!`
- capacidade de publicar no github/netlify/vercel
- pacote para iOS/Desktop/etc

### Suporte ao LiveView / Componente do Servidor

A arquitetura interna do Dioxus foi projetada desde o primeiro dia para suportar o caso de uso `LiveView`, onde um servidor Web hospeda um aplicativo em execução para cada usuário conectado. A partir de hoje, não há suporte LiveView de primeira classe – você precisará conectar isso sozinho.

Embora não esteja totalmente implementado, a expectativa é que os aplicativos LiveView possam ser um híbrido entre Wasm e renderizado pelo servidor, onde apenas partes de uma página são "ao vivo" e o restante da página é renderizado pelo servidor, gerado estaticamente ou manipulado pelo SPA anfitrião.
