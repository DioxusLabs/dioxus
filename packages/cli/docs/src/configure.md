# Configure Project


This chapter will introduce you to how to configure the CLI with your `Dioxus.toml` file

Be aware that if the config file is present in the folder, some fields must be filled out, or the CLI tool will abort. The mandatory [table headers](https://toml.io/en/v1.0.0#table) and keys will have a '✍' sign beside it.

## Structure

The CLI uses a `Dioxus.toml` file in the root of your crate to define some configuration for your `dioxus` project.

### Application ✍

General application confiration:

```
[application]
# configuration
```
1. ***name*** ✍ - project name & title
2. ***default_platform*** ✍ - which platform target for this project.

   ```
   name = "my-project"
   ```
2. ***default_platform*** - The platform this project targets
   ```ß
   # current supported platforms: web, desktop
   # default: web
   default_platform = "web"
   ```
   if you change this to `desktop`, the `dioxus build` will default building a desktop app
3. ***out_dir*** - The directory to place the build artifacts from `dioxus build` or `dioxus service` into. This is also where the `assets` directory will be copied to
    ```
    out_dir = "dist"
    ```
4. ***asset_dir*** - The directory with your static assets. The CLI will automatically copy these assets into the ***out_dir*** after a build/serve.
   ```
   asset_dir = "public"
   ```
5. ***sub_package*** - The sub package in the workspace to build by default
   ```
   sub_package = "my-crate"
   ```

### Web.App ✍

Configeration specific to web applications:

```
[web.app]
# configuration
```

1. ***title*** - The title of the web page
   ```
   # HTML title tag content
   title = "dioxus app | ⛺"
   ```
2. ***base_path*** - The base path to build the appliation for serving at. This can be useful when serving your application in a subdirectory under a domain. For example when building a site to be served on github pages.
   ```
   # The application will be served at domain.com/my_application/, so we need to modify the base_path to the path where the application will be served
   base_path = "my_application"
   ```

### Web.Watcher ✍

Configeration related to the development server:

```
[web.watcher]
# configuration
```

1. ***reload_html*** - If this is true, the cli will rebuild the index.html file every time the application is rebuilt
   ```
   reload_html = true
   ```
2. ***watch_path*** - The files & directories to moniter for changes
   ```
   watch_path = ["src", "public"]
   ```
3. ***index_on_404*** - If enabled, Dioxus CLI will serve the root page when a route is not found. *This is needed when serving an application that uses the router*
   ```
   index_on_404 = true
   ```

### Web.Resource ✍

Configeration related to static resources your application uses:
```
[web.resource]
# configuration
```

1. ***style*** - The styles (`.css` files) to include in your application
   ```
   style = [
      # include from public_dir.
      "./assets/style.css",
      # or some asset from online cdn.
      "https://cdn.jsdelivr.net/npm/bootstrap/dist/css/bootstrap.css"
   ]
   ```
2. ***script*** - The additional scripts (`.js` files) to include in your application
    ```
    style = [
        # include from public_dir.
        "./assets/index.js",
        # or some asset from online cdn.
        "https://cdn.jsdelivr.net/npm/bootstrap/dist/js/bootstrap.js"
    ]
   ```

### Web.Resource.Dev ✍

Configeration related to static resources your application uses in development:
```
[web.resource.dev]
# configuration
```

1. ***style*** - The styles (`.css` files) to include in your application
   ```
   style = [
      # include from public_dir.
      "./assets/style.css",
      # or some asset from online cdn.
      "https://cdn.jsdelivr.net/npm/bootstrap/dist/css/bootstrap.css"
   ]
   ```
2. ***script*** - The additional scripts (`.js` files) to include in your application
    ```
    style = [
        # include from public_dir.
        "./assets/index.js",
        # or some asset from online cdn.
        "https://cdn.jsdelivr.net/npm/bootstrap/dist/js/bootstrap.js"
    ]
   ```

### Web.Proxy

Configeration related to any proxies your application requires durring development. Proxies will forward requests to a new service

```
[web.proxy]
# configuration
```

1. ***backend*** - The URL to the server to proxy. The CLI will forward any requests under the backend relative route to the backend instead of returning 404
   ```
   backend = "http://localhost:8000/api/"
   ```
   This will cause any requests made to the dev server with prefix /api/ to be redirected to the backend server at http://localhost:8000. The path and query parameters will be passed on as-is (path rewriting is not currently supported).

## Config example

```toml
[application]

# App (Project) Name
name = "{{project-name}}"

# The Dioxus platform to default to
default_platform = "web"

# `build` & `serve` output path
out_dir = "dist"

# the static resource path
asset_dir = "public"

[web.app]

# HTML title tag content
title = "dioxus | ⛺"

[web.watcher]

# when watcher is triggered, regenerate the `index.html`
reload_html = true

# which files or dirs will be monitored
watch_path = ["src", "public"]

# include `assets` in web platform
[web.resource]

# CSS style file
style = []

# Javascript code file
script = []

[web.resource.dev]

# serve: [dev-server] only

# CSS style file
style = []

# Javascript code file
script = []

[[web.proxy]]
backend = "http://localhost:8000/api/"
```
