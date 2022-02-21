# Configure Project

This chapter will introduce `Dioxus.toml` and anatomy the config file.

## Structure

We use `toml` to define some info for `dioxus` project.

### Application

1. ***name*** - project name & title
2. ***default_platform*** - which platform target for this project.
   ```
   # current support: web, desktop
   # default: web
   default_platform = "web"
   ```
   change this to `desktop`, the `dioxus build & serve` will default build desktop app.
3. ***out_dir*** - which directory to put the output file; use `dioxus build & service`, the output will put at this directory, and the `assets` will be also copy to here.
    ```
    out_dir = "dist"
    ```
4. ***asset_dir*** - which direcotry to put your `static, assets` file, cli will automatic copy all file to `out_dir`, so you can put some resource file in there, like `CSS, JS, Image` file.
   ```
   asset_dir = "public"
   ```

### Web.App

Web platform application config:

1. ***title*** - this value will display on the web page title. like `<title></title>` tag.
   ```
   # HTML title tag content
   title = "dioxus app | ⛺"
   ```

### Web.Watcher

Web platform `dev-server` watcher config:

1. ***reload_html*** - a boolean value; when watcher trigger, regenerate `index.html` file.
   ```
   # when watcher trigger, regenerate the `index.html`
   reload_html = true
   ```
2. ***watch_path*** - which files & directories will be watcher monitoring.
   ```
   watch_path = ["src", "public"]
   ```

### Web.Resource

Include some `CSS Javascript` resources into html file.

1. ***style*** - include some style(CSS) file into html.
   ```
   style = [
      # include from public_dir.
      "./assets/style.css",
      # or some asset from online cdn.
      "https://cdn.jsdelivr.net/npm/bootstrap/dist/css/bootstrap.css"
   ]
   ```
2. ***script*** - include soe script(JS) file into html.
    ```
    style = [
        # include from public_dir.
        "./assets/index.js",
        # or some asset from online cdn.
        "https://cdn.jsdelivr.net/npm/bootstrap/dist/js/bootstrap.js"
    ]
   ```

### Web.Resource.Dev

Only include resources at `Dev` mode.

1. ***style*** - include some style(CSS) file into html.
   ```
   style = [
      # include from public_dir.
      "./assets/style.css",
      # or some asset from online cdn.
      "https://cdn.jsdelivr.net/npm/bootstrap/dist/css/bootstrap.css"
   ]
   ```
2. ***script*** - include soe script(JS) file into html.
    ```
    style = [
        # include from public_dir.
        "./assets/index.js",
        # or some asset from online cdn.
        "https://cdn.jsdelivr.net/npm/bootstrap/dist/js/bootstrap.js"
    ]
   ```
