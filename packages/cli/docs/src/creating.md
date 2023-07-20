# Create a Project

Once you have the Dioxus CLI tool installed, you can use it to create dioxus project.

## Initializing a default project

First, run the `dx create` command to create a new project ready to be used with Dioxus and the Dioxus CLI:

```
dx create hello-dioxus
```

> It will clone a default template from github template: [DioxusLabs/dioxus-template](https://github.com/DioxusLabs/dioxus-template)
> This default template is use for `web` platform application.
>
> You can choose to create your project from a different template by passing the `template` argument:
> ```
> dx init hello-dioxus --template=gh:dioxuslabs/dioxus-template
> ```

Next, move the current directory into your new project:

```
cd hello-dioxus
```

> Make sure `wasm32 target` is installed before running the Web project.
> You can install the wasm target for rust using rustup:
> ```
> rustup target add wasm32-unknown-unknown
> ```

Finally, create serve your project with the Dioxus CLI:

```
dx serve
```

By default, the CLI serve your site at: [`http://127.0.0.1:8080/`](http://127.0.0.1:8080/)
