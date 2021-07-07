<div align="center">
  <h1>📦✨  dioxus-Studio </h1>
  <p>
    <strong>Tooling to supercharge dioxus projects</strong>
  </p>
</div>

# About
---
dioxus-cli (inspired by wasm-pack and webpack) is a tool to help get dioxus projects off the ground. It handles all the build, development, bundling, and publishing to make web development just a simple two commands: `cargo init` and `dioxus-cli publish`.

Best thing: 
- No NPM. 
- No Webpack. 
- No `node_modules`. 
- No Babel
- No parcel
- No rollup
- No ESLint

Just install Rust, dioxus-cli, and you're good to go.
`cargo install --git github.com/jkelleyrtp/dioxus-cli`

Need a development server?
`dioxus develop`

Need to run an example?
`dioxus develop --example textbox`

Need to benchmark a component?
`dioxus bench`

Need to test your code?
`dioxus test`

Need to build your code into a bundle?
`dioxus build --outdir public`

Need to publish your code to GitHub pages, Netlify, etc?
`dioxus publish --ghpages myrepo.git`

# Use in your project
---
Sometimes you'll want to include static assets without bundling them into your .wasm content. dioxus-cli provides a few ways of doing this:

- Load in dynamic content using `dioxus::asset("./static/images/blah.svg")`
- Live-reload HTML templates without rebuilding your .wasm with `dioxus::template("./templates/blah.html")`
- Use a CSS library like tailwind in your dioxus configuration with
