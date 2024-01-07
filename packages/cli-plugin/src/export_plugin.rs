#[macro_export]
macro_rules! export_plugin {
    ($name:ident) => {
        ::wit_bindgen::generate!({
            inline: "package plugins:main;

interface definitions {
  use types.{platform, plugin-info, command-event, runtime-event, response-event};
  use toml.{toml, toml-value};

  /// Get the default layout for the plugin to put
  /// into `Dioxus.toml`
  get-default-config: func() -> toml;

  /// Take config from `Dioxus.toml` and apply
  /// to the plugin, returns false if couldn't apply
  apply-config: func(config: toml) -> result;
  
  /// Initialize the plugin. This will be called once after the plugin is added
  register: func() -> result;

  /// Get the metadata of the plugin
  metadata: func() -> plugin-info;

  /// Called right before the event given
  /// This is called when commands like `Build`, `Translate`, etc 
  /// are called from the CLI
  before-command-event: func(event: command-event) -> result;
  /// Called right before the event given
  /// These are the runtime-functions like `HotReload` and `Serve`
  before-runtime-event: func(event: runtime-event) -> result<response-event>;

  /// Called right after the event given
  /// This is called when commands like `Build`, `Translate`, etc 
  /// are called from the CLI
  after-command-event: func(event: command-event) -> result;
  /// Called right after the event given
  after-runtime-event: func(event: runtime-event) -> result<response-event>;
  
  /// Gives a list of paths that have changed,
  /// you can add to the watched list of paths with `watch_path`
  on-watched-paths-change: func(path: list<string>) -> result<response-event>;

  

  /// Check if there is an update to the plugin 
  /// with a given git repo?
  /// returns error if there was error getting git
  /// Some(url) => git clone url
  /// None => No update needed
  /// check-update: func() -> result<option<string>>
}

interface toml {
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

  record plugin-info {
    name: string,
    version: string,
    // perms?
  }

  record project-info {
    // Is true when there is a `/dist` folder available
    has-output-directory: bool,
    // Is true when there is a `/assets` folder available
    has-assets-directory: bool,
    default-platform: platform,
  }

  enum command-event {
    build,
    bundle,
    translate,
    serve,
  }

  enum runtime-event {
    rebuild,
    hot-reload
  }

  variant response-event {
    none,
    reload,
    rebuild,
    refresh(list<string>)
  }
}

interface imports {
  use types.{project-info};

  /// This is used to find out the name of your plugin as well
  /// as the version of your plugin. The name should be 
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