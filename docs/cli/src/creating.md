# Create a Project

Once you have the Dioxus CLI tool installed, you can use it to create dioxus project.

## Initializing a default project

The `dioxus create` command will create a new directory containing a project template.
```
dioxus create hello-dioxus
```

It will clone a default template from github template: [DioxusLabs/dioxus-template](https://github.com/DioxusLabs/dioxus-template)

> This default template is use for `web` platform application.

then you can change the current directory in to the project:

```
cd hello-dioxus
```

> Make sure `wasm32 target` is installed before running the Web project.

now we can create a `dev server` to display the project:

```
dioxus serve
```

by default, the dioxus dev server will running at: [`http://127.0.0.1:8080/`](http://127.0.0.1:8080/)

## Initalizing from other repository

you can assign which repository you want to create:

```
dioxus init hello-dioxus --template=gh:dioxuslabs/dioxus-template
```