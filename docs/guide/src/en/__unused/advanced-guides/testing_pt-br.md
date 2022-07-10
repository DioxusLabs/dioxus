## Testes

Para testar seu código Rust, você pode anotar qualquer função com o bloco `#[test]`. No VSCode com RA (`rust-analyzer`), isso fornecerá uma opção para clicar e executar o teste.

```rust
#[test]
fn component_runs() {
    assert!(true)
}
```

Isso testará seu código Rust _sem_ passar pelo navegador. É ideal para eliminar bugs lógicos e garantir que os componentes sejam renderizados adequadamente quando o DOM dos navegadores não for necessário. Se você precisar executar testes no navegador, você pode anotar seus blocos com o bloco `#[dioxus::test]`.

```rust
#[dioxus::test]
fn runs_in_browser() {
    // ...
}
```

Então, quando você executar:

```console
$ dioxus test --chrome
```

Dioxus irá montar e testar seu código usando o navegador Chrome como um depurador.

Há muito mais para testar se você mergulhar de cabeça nos testes, então confira o guia [Testagem]() para mais informações
