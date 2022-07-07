# `use_state` e `use_ref`

A maioria dos componentes que você escreverá no Dioxus precisará armazenar o estado de alguma forma. Para o estado local, fornecemos dois hooks muito convenientes:

- [use_state](https://docs.rs/dioxus-hooks/latest/dioxus_hooks/fn.use_state.html)
- [use_ref](https://docs.rs/dioxus-hooks/latest/dioxus_hooks/fn.use_ref.html)

Ambos os hooks são extremamente poderosos e flexíveis, por isso dedicamos esta seção para entendê-los corretamente.

> Esses dois hooks não são a única maneira de armazenar o estado. Você sempre poderá criar seus próprios hooks!
