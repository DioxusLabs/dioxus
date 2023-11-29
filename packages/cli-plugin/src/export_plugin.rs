#[macro_export]
macro_rules! export_plugin {
    ($name:ident) => {
        ::wit_bindgen::generate!({
            inline: "package plugins:main;

interface definitions {
  on-rebuild: func() -> bool;

  on-hot-reload: func();

  on-watched-paths-change: func(path: list<string>);
}

interface imports {
  output-directory: func() -> string;

  reload-browser: func();
  refresh-asset: func(old-url: string, new-url: string);

  watch-path: func(path: string);
}

world plugin-world {
  import imports;

  export definitions;
}
",
            world: "plugin-world",
            exports: {
                world: $name,
                "plugins:main/definitions": $name
            },
        });
    };
}