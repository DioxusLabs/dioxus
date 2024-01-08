Example: Basic Tailwind usage

This example shows how an app might be styled with TailwindCSS.

# Setup

1. Install the Dioxus CLI:

```bash
cargo install dioxus-cli
```

2. Install npm: https://docs.npmjs.com/downloading-and-installing-node-js-and-npm
3. Install the tailwind css cli: https://tailwindcss.com/docs/installation
4. Initialize the tailwind css project:

```bash
npx tailwindcss init
```

This should create a `tailwind.config.js` file in the root of the project.

5. Edit the `tailwind.config.js` file to include rust files:

```json
module.exports = {
    mode: "all",
    content: [
        // include all rust, html and css files in the src directory
        "./src/**/*.{rs,html,css}",
        // include all html files in the output (dist) directory
        "./dist/**/*.html",
    ],
    theme: {
        extend: {},
    },
    plugins: [],
}
```

6. Create a `input.css` file with the following content:

```css
@tailwind base;
@tailwind components;
@tailwind utilities;
```

7. Create a `Dioxus.toml` file with the following content that links to the `tailwind.css` file:

```toml
[application]

# App (Project) Name
name = "Tailwind CSS + Dioxus"

# Dioxus App Default Platform
# desktop, web, mobile, ssr
default_platform = "web"

# `build` & `serve` dist path
out_dir = "dist"

# resource (public) file folder
asset_dir = "public"

[web.app]

# HTML title tag content
title = "dioxus | ⛺"

[web.watcher]

# when watcher trigger, regenerate the `index.html`
reload_html = true

# which files or dirs will be watcher monitoring
watch_path = ["src", "public"]

# include `assets` in web platform
[web.resource]

# CSS style file
style = ["/tailwind.css"]

# Javascript code file
script = []

[web.resource.dev]

# serve: [dev-server] only

# CSS style file
style = []

# Javascript code file
script = []
```

## Bonus Steps

8. Install the tailwind css vs code extension
9. Go to the settings for the extension and find the experimental regex support section. Edit the setting.json file to look like this:

```json
"tailwindCSS.experimental.classRegex": ["class: \"(.*)\""],
"tailwindCSS.includeLanguages": {
    "rust": "html"
},
```

# Development

1. Run the following command in the root of the project to start the tailwind css compiler:

```bash
npx tailwindcss -i ./input.css -o ./public/tailwind.css --watch
```

## Web

- Run the following command in the root of the project to start the dioxus dev server:

```bash
dx serve --hot-reload
```

- Open the browser to http://localhost:8080

## Desktop

- Launch the dioxus desktop app

```bash
cargo run
```
