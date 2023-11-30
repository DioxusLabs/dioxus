#[macro_export]
macro_rules! export_plugin {
    ($name:ident) => {
        ::wit_bindgen::generate!({
            inline: "package plugins:main;

interface definitions {
  use imports.{platform};
  use toml.{toml, toml-value};

  get-default-config: func() -> toml;

  apply-config: func(config: toml) -> bool;
  
  // Initialize the plugin
  register: func() -> bool;

  // Before the app is built
  before-build: func() -> bool;

  // After the application is built, before serve
  before-serve: func() -> bool;

  // Reload on serve with no hot-reloading(?)
  on-rebuild: func() -> bool;

  // Reload on serve with hot-reloading
  on-hot-reload: func();

  /// Check if there is an update to the plugin 
  /// with a given git repo?
  /// returns error if there was error getting git
  /// Some(url) => git clone url
  /// None => No update needed
  /// check-update: func() -> result<option<string>>

  on-watched-paths-change: func(path: list<string>);
}

interface toml {
  resource toml {
    constructor(value: toml-value);
    get: func() -> toml-value;
    set: func(value: toml-value);
  }

  variant toml-value {
    %string(string),
    integer(s64),
    float(float64),
    %array(array),
    %table(table),
  }

  type array = list<toml>;
  type table = list<tuple<string, toml>>;
}

interface imports {
  enum platform {
    web,
    desktop,
  }

  get-platform: func() -> platform;

  output-directory: func() -> string;

  refresh-browser-page: func();

  /// Searches through links to only refresh the 
  /// necessary components when changing assets
  refresh-asset: func(old-url: string, new-url: string);

  /// Add path to list of watched paths
  watch-path: func(path: string);

  /// Get list of watched paths
  watched-paths: func() -> list<string>;

  /// Try to remove a path from list of watched paths
  /// returns false if path not in list
  remove-path: func(path: string) -> bool;

  log: func(info: string);

}

world plugin-world {
  import imports;
  import toml;
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