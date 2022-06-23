# Progresso & Conjunto de Características

Antes de irmos à fundo no Dioxus, sinta-se à vontade para ver nosso conjunto de características e nosso progresso para saber se o que o Dioxus pode fornecer hoje funciona pra você.

Se as características que você precisa não existem ou você quer contribuir com o progresso do projeto, sinta-se à vontade para se envolver [se juntando ao Discord](https://discord.gg/XgGxMSkvUM).

No geral, aqui tem o estado de cada plataforma:

- **Web**: Dioxus é uma ótima escolha para aplicativos puramente feito para Web - especialmente para aplicativos CRUD/complexos. No entanto, ele ainda falta o ecossistema do React, então você pode sentir falta de alguma biblioteca ou algum Hook útil.

- **SSR**: Dioxus é uma ótima escolha para pre-renderização, hidratação e renderização HTML sobre a Web. Esteja avisado - a VirtualDOM ainda não é (atualmente) `Send + Sync`.

- **Desktop**: Você pode desenvolver aplicativos de única-janela bem competentes agora mesmo. No entanto, aplicativos multi-janelas requerem suporte do núcleo do Dioxus que ainda não estão prontos.

- **Mobile**: Suporte móvel é muito recente. Você terá de solucionar as coisas por si mesmo enquanto desenvolve e ainda não muitos pacotes que suportem periféricos.

- **LiveView**: O suporte ao LiveView é muito recente. Você terá de solucionar as coisas por si mesmo enquanto desenvolve. Entretanto, nenhuma é muito difícil e qualquer solução pode ser encaminhada para dentro do Dioxus.

## Características

---

| Característica                      | Estado | Descrição                                                                                        |
| ----------------------------------- | ------ | ------------------------------------------------------------------------------------------------ |
| Renderização Condicional            | ✅     | `if/then` para esconder/mostrar componente                                                       |
| Map, Iterador                       | ✅     | `map/filter/reduce` para produzir `rsx!`                                                         |
| Componentes Chaveados               | ✅     | comparação em `diff` com chaves                                                                  |
| Web                                 | ✅     | renderizador para navegadores Web                                                                |
| Desktop (WebView)                   | ✅     | renderizador para Desktop                                                                        |
| Estado Compartilhado (Context)      | ✅     | compartilha estados através de árvores                                                           |
| Hooks                               | ✅     | células de memoria nos componentes                                                               |
| SSR                                 | ✅     | renderiza diretamente para `string`                                                              |
| Componente Filho                    | ✅     | `cx.children()` como lista de nós                                                                |
| Componentes Sem Elementos           | ✅     | componentes que não renderizam elementos reais na DOM                                            |
| Fragmentos                          | ✅     | elementos múltiplos sem uma raiz real                                                            |
| Propriedades Manuais                | ✅     | passa manualmente `props` com `spread syntax`                                                    |
| Entradas Controladas                | ✅     | encapsulamento com estado em sobre entradas                                                      |
| Estilos CSS/Inline                  | ✅     | sintaxe para grupos de estilo/atributos em linha                                                 |
| Elementos Personalizados            | ✅     | define novos elementos primitivos                                                                |
| Suspense                            | ✅     | programa futuras renderizações usando `future`/`Promise`                                         |
| Tratamento Integrado de Erros       | ✅     | trata erros graciosamente com a sintaxe `?`                                                      |
| NodeRef                             | ✅     | ganha acesso direto aos nós                                                                      |
| Re-hidratação                       | ✅     | pre-renderiza HTML para acelerar a primeira impressão na tela                                    |
| Renderização Livre de Gargalos      | ✅     | `diffs` grandes são segmentados sobre quadros para uma transição suave como seda                 |
| Efeitos                             | ✅     | executa efeitos após um componente ser enviado para a fila de renderização                       |
| Portais                             | 🛠      | renderiza nós fora da árvore tradicional de elementos (DOM)                                      |
| Agendamento Cooperativo             | 🛠      | prioriza eventos com mais importância sobre eventos menos importantes                            |
| Componentes de Servidor             | 🛠      | componentes híbridos para aplicativos de única página (SPA) e servidores                         |
| Divisão de Pacotes                  | 👀     | carrega o aplicativo assincronamente e eficientemente                                            |
| Componentes Tardios                 | 👀     | dinamicamente carrega os novos componentes assim que estiverem prontos enquanto a página carrega |
| Estado Global de 1ª Classe          | ✅     | `redux/recoil/mobx` sobre o `context`                                                            |
| Execução Nativa                     | ✅     | execução como um binário portátil sem um `runtime` (Node)                                        |
| Sub-Árvore de Memoization           | ✅     | pula o `diffing` em sub-árvores de elementos estáticos                                           |
| Modelos de Alta Eficiência          | 🛠      | chamadas `rsx!` são traduzidas para modelos sobre a `DOM`                                        |
| Garantia de Correção por Compilador | ✅     | avisa sobre erros em esquemas de modelos inválidos antes do final da compilação                  |
| Motor de Heurística                 | ✅     | rastreia componentes na memória para minimizar alocações futures                                 |
| Controle Preciso de Reatividade     | 👀     | pula o `diffing` para ter controle preciso das atualizações de tela                              |

- ✅ = implementado e funcionando
- 🛠 = sendo ativamente trabalho
- 👀 = ainda não implementado ou sendo trabalhado
- ❓ = incerto se pode ou não ser implementado

## Progresso

Estas características estão planejadas para o futuro do Dioxus:

---

### Core

- [x] Entrega do Dioxus Core
- [x] Melhoria da documentação para incluir mais teoria e ser mais compreensivo
- [ ] Suporte de modelos HTML para manipulação da `DOM` incrivelmente rápido
- [ ] Suporte para múltiplos renderizadores para a mesma `VirtualDOM` (sub-árvores)
- [ ] Suporte para ThreadSafe (Send + Sync)
- [ ] Suporte para Portais

### SSR

- [x] Suporte SSR + Hydration
- [ ] Suporte ao Suspense integrado para SSR

### Desktop

- [ ] Gerenciamento de janela declarativo
- [ ] Modelos para contruir/empacotar
- [ ] Renderizador totalmente nativo
- [ ] Acesso ao contexto Canvas/WebGL nativo

### Móvel

- [ ] Biblioteca padrão Móvel
  - [ ] GPS
  - [ ] Câmera
  - [ ] Arquivo de Sistema
  - [ ] Biometria
  - [ ] WiFi
  - [ ] Bluetooth
  - [ ] Notificações
  - [ ] Área de Transferência
- [ ] Animações
- [ ] Renderizador Nativo

### Bundling (CLI)

- [x] Tradução de HTML para RSX
- [x] Servidor de Desenvolvimento
- [x] Live reload
- [x] Tradução de JSX para RSX
- [ ] Substituição de módulos em funcionamento (Hot replace)
- [ ] Segmentação de código
- [ ] Acervo de macros
- [ ] Encadeamento de CSS
- [ ] Encadeamento de Imagens

### Hooks Essenciais

- [x] Router
- [x] Gerenciamento Global de Estado
- [ ] Observador de redimensionamento
