# Building the Documentation

Dioxus uses a fork of MdBook with multilanguage support. To build the documentation, you will need to install the forked version of MdBook.

```sh
cargo install mdbook --git https://github.com/Demonthos/mdBook.git --branch master
```

Then, you can build the documentation by running:

Linux and MacOS:

```sh
cd docs && cd guide && mdbook build -d ../nightly/guide && cd .. && cd router && mdbook build -d ../nightly/router && cd .. && cd ..
```

Windows:

```cmd
cd docs; cd guide; mdbook build -d ../nightly/guide; cd ..; cd router; mdbook build -d ../nightly/router; cd ..; cd ..
```
