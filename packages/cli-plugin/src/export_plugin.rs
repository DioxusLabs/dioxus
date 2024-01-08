#[macro_export]
macro_rules! export_plugin {
    ($name:ident) => {
        ::wit_bindgen::generate!({
            inline: "package plugins:main;

interface definitions {
  use types.{platform, plugin-info, command-event, runtime-event, response-event};
  use toml.{toml, toml-value};

  /// Returns the plugin's default configuration as a TOML value. 
  /// The host application merges this with its `Dioxus.toml`.
  get-default-config: func() -> toml;

  /// Applies the resolved configuration value for this plugin from the main configuration file. 
  /// Plugins should validate the passed config and store relevant values as these could be changed from the default. 
  /// Return an Error if the config is invalid.
  apply-config: func(config: toml) -> result;
  
  /// Performs one-time initialization when the plugin is first loaded. 
  /// Return an Error to fail registration.
  register: func() -> result;

  /// Get the metadata of the plugin
  metadata: func() -> plugin-info;

  /// Called before build commands like build, bundle, etc
  before-command-event: func(event: command-event) -> result;
  /// Called before runtime events like when a served application is being hot-reloaded
  /// or being rebuilt, and the plugin can perform additional steps inbetween
  before-runtime-event: func(event: runtime-event) -> result<response-event>;

  /// Called after build commands like build, bundle, etc
  after-command-event: func(event: command-event) -> result;
  /// Called after runtime events like when a served application is being hot-reloaded
  /// or being rebuilt, and the plugin can perform additional steps inbetween
  after-runtime-event: func(event: runtime-event) -> result<response-event>;
  
  /// Notifies the plugin when watched file(s) change.
  /// Plugins can watch additional paths with `watch_path`
  on-watched-paths-change: func(path: list<string>) -> result<response-event>;

  

  /// Check if there is an update to the plugin 
  /// with a given git repo?
  /// returns error if there was error getting git
  /// Some(url) => git clone url
  /// None => No update needed
  /// check-update: func() -> result<option<string>>
}

interface toml {
  /// The handle for a `TomlValue`
  resource toml {
    /// Creates a value in table and returns the handle
    constructor(value: toml-value);
    /// Clones value from table
    get: func() -> toml-value;
    /// Sets value in table
    set: func(value: toml-value);
    /// Clones the handle, not the value
    clone: func() -> toml;
  }

  variant toml-value {
    %string(string),
    integer(s64),
    float(float64),
    boolean(bool),
    datetime(datetime),
    %array(array),
    %table(table),
  }

  record datetime {
    date: option<date>,
    time: option<time>,
    offset: option<offset>,
  }

  record date {
    year: u16,
    month: u8,
    day: u8,
  }

  record time {
    hour: u8,
    minute: u8, 
    second: u8,
    nanosecond: u32,
  }

  variant offset {
    z,
    custom(tuple<s8,u8>),
  }

  type array = list<toml>;
  type table = list<tuple<string, toml>>;
}

interface types {
  enum platform {
    web,
    desktop,
  }

  /// General information given to the host project about the plugin
  record plugin-info {
    name: string,
    version: string,
    // perms?
  }

  /// General information about the host project
  record project-info {
    // Is true when there is a `/dist` folder available
    has-output-directory: bool,
    // Is true when there is a `/assets` folder available
    has-assets-directory: bool,
    default-platform: platform,
  }

  /// Command events are used to notify the plugin when the project is 
  /// being built, served, translated, or bundled. 
  enum command-event {
    build,
    bundle,
    translate,
    serve,
  }

  /// When the project is being served, the plugin can be called with a
  /// `RuntimeEvent` to affect the project at runtime using a `ResponseEvent`
  enum runtime-event {
    rebuild,
    hot-reload
  }

  /// A `ResponseEvent` object is only ever returned from a plugin call with
  /// a runtime event, when the project calls the plugins during runtime the 
  /// most 'destructive' event is going to be called. E.g. two plugins return 
  /// a reload event and a rebuild event, the rebuild event would take precedence
  variant response-event {
    none,
    reload,
    rebuild,
    refresh(list<string>)
  }
}

interface imports {
  use types.{project-info};

  /// Returns whether the project has 'output' and 'asset' 
  /// directories written in the `Dioxus.toml`, and the 
  /// default platform for the project
  get-project-info: func() -> project-info;

  /// Add path to list of watched paths
  watch-path: func(path: string);

  /// Try to remove a path from list of watched paths
  /// returns an error if path not in list
  remove-watched-path: func(path: string) -> result;

  /// Get list of currently watched paths
  watched-paths: func() -> list<string>;

  /// Set data in a map, is saved between sessions in the lock file
  set-data: func(key: string, data: list<u8>);

  /// Grab data from map, returns none if key not in map
  get-data: func(key: string) -> option<list<u8>>;

  /// Sends the input string to whatever logger is currently being used
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