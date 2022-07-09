## Publishing with Github Pages
To build our app and publish it to Github:

- Make sure GitHub Pages is set up for your repo
- Build your app with `trunk build --release` (include `--public-url <repo-name>` to update asset prefixes if using a project site)
- Move your generated HTML/CSS/JS/Wasm from `dist` into the folder configured for Github Pages
- Add and commit with git
- Push to GitHub
