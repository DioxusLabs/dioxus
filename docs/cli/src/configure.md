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

1. ***title*** - this value will display on the web page title. like `<title></title>` tag.