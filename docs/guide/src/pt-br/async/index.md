# Trabalhando em Assincronia

Muitas vezes, os aplicativos precisam interagir com sistemas de arquivos, interfaces de rede, hardware ou temporizadores. Este capítulo fornece uma visão geral do uso de código assíncrono no Dioxus.

## O Tempo de Execução (runtime)

Por padrão, o Dioxus-Desktop vem com o runtime `Tokio` e configura tudo automaticamente para você. No momento, isso não é configurável, embora seja fácil escrever uma integração para o desktop Dioxus que use um tempo de execução assíncrono diferente.

Dioxus atualmente não é `thread-safe`, então qualquer código assíncrono que você escreve _não_ precisa ser `Send/Sync`. Isso significa que você pode usar estruturas não `thread-safe` como `Cell`, `Rc` e `RefCell`.
