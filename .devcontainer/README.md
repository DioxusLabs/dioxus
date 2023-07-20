# Dev Container

A dev container in the most simple context allows one to create a consistent development environment within a docker container that can easily be opened locally or remotely via codespaces such that contributors don't need to install anything to contribute.

## Useful Links

- <https://code.visualstudio.com/docs/devcontainers/containers>
- <https://containers.dev/>
- <https://github.com/devcontainers>
- <https://marketplace.visualstudio.com/items?itemName=ms-vscode-remote.remote-containers>

## Using A Dev Container

### Locally

To use this dev container locally, make sure Docker is installed and in VSCode install the `ms-vscode-remote.remote-containers` extension. Then from the root of Dioxus you can type `Ctrl + Shift + P`, then choose `Dev Containers: Rebuild and Reopen in Devcontainer`.

### Codespaces

[Codespaces Setup](https://docs.github.com/en/codespaces/developing-in-codespaces/creating-a-codespace-for-a-repository#creating-a-codespace-for-a-repository)

## Troubleshooting

If having difficulty commiting with github, and you use ssh or gpg keys, you may need to ensure that the keys are being shared properly between your host and VSCode.

Though VSCode does a pretty good job sharing credentials between host and devcontainer, to save some time you can always just reopen the container locally to commit with `Ctrl + Shift + P`, then choose `Dev Containers: Reopen Folder Locally`
