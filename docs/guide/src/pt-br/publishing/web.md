## Publicando com páginas do Github

Para construir nosso aplicativo e publicá-lo no Github:

- Verifique se o GitHub Pages está configurado para seu repositório
- Crie seu aplicativo com `trunk build --release` (inclua `--public-url <repo-name>` para atualizar os prefixos de ativos se estiver usando um site de projeto)
- Mova seu HTML/CSS/JS/Wasm gerado de `dist` para a pasta configurada para Github Pages
- Adicione e confirme com `git`
- `git push` para o GitHub
