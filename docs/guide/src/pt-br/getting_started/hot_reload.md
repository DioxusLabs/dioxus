# Configurando o Hot Reload

1. O recarregamento em tempo-real (_hot reload_) permite tempos de iteração muito mais rápidos dentro de chamadas 'rsx', interpretando-as e transmitindo as edições.
2. É útil para alterar o estilo/layout de um programa, mas não ajudará na alteração da lógica de um programa.
3. Atualmente, o cli implementa apenas o _hot-reload_ para o renderizador da web.

# Configurar

Instale o [dioxus-cli](https://github.com/DioxusLabs/cli).
Habilite o recurso de _hot-reload_ no dioxus:

```toml
dioxus = { version = "*", features = ["hot-reload"] }
```

# Usage

1. Execute:

```
dioxus serve --hot-reload
```

2. alterar algum código dentro de uma macro `rsx`
3. abra seu `localhost` em um navegador
4. salve e observe a mudança de estilo sem recompilar

# Limitações

1. O interpretador só pode usar expressões que existiam na última recompilação completa. Se você introduzir uma nova variável ou expressão na chamada `rsx`, ela acionará uma recompilação completa para capturar a expressão.
2. Componentes e Iteradores podem conter código de Rust arbitrário e acionarão uma recompilação completa quando alterados.
