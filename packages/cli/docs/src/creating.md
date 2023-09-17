# Create a Project

Once you have the Dioxus CLI installed, you can use it to create your own project!

## Initializing a default project

First, run the `dx create` command to create a new project:
```
dx create hello-dioxus
```

> It will clone this [template](https://github.com/DioxusLabs/dioxus-template).
> This default template is used for `web` platform application.
>
> You can choose to create your project from a different template by passing the `template` argument:
> ```
> dx init hello-dioxus --template=gh:dioxuslabs/dioxus-template
> ```

Next, navigate into your new project:

```
cd hello-dioxus
```

> Make sure the WASM target is installed before running the projects.
> You can install the WASM target for rust using rustup:
> ```
> rustup target add wasm32-unknown-unknown
> ```

Finally, serve your project:
```
dx serve
```

By default, the CLI serves your website at [`http://127.0.0.1:8080/`](http://127.0.0.1:8080/).
