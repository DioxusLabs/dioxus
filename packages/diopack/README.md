<div align="center">
  <h1>📦✨  yew-pack</h1>
  <p>
    <strong>Tooling to supercharge yew projects</strong>
  </p>
</div>

# About
---
yewpack (inspired by wasm-pack and webpack) is a tool to help get Yew projects off the ground. It handles all the build, development, bundling, and publishing to make web development just a simple two commands: `cargo init` and `yewpack publish`.

Best thing: 
- No NPM. 
- No Webpack. 
- No `node_modules`. 
- No Babel
- No parcel
- No rollup
- No ESLint

Just install Rust, yewpack, and you're good to go.
`cargo install --git github.com/jkelleyrtp/yewpack`

Need a development server?
`yewpack develop`

Need to run an example?
`yewpack develop --example textbox`

Need to benchmark a component?
`yewpack bench`

Need to test your code?
`yewpack test`

Need to build your code into a bundle?
`yewpack build --outdir public`

Need to publish your code to GitHub pages, Netlify, etc?
`yewpack publish --ghpages myrepo.git`

# Use in your project
---
Sometimes you'll want to include static assets without bundling them into your .wasm content. yewpack provides a few ways of doing this:

- Load in dynamic content using `yewpack::asset("./static/images/blah.svg")`
- Live-reload HTML templates without rebuilding your .wasm with `yewpack::template("./templates/blah.html")`
- Use a CSS library like tailwind in your yewpack configuration with
