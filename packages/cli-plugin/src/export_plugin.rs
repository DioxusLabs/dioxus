#[macro_export]
macro_rules! export_plugin {
    ($name:ident) => {
        ::wit_bindgen::generate!({
            inline: "package plugins:main;

interface definitions {
  on-rebuild: func() -> bool;

  on-hot-reload: func();
}

world plugin-world {
  import output-directory: func() -> string;

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