//! Common utilities for integration examples

pub mod logger {

    use fern::colors::{Color, ColoredLevelConfig};
    use log::debug;

    pub fn set_up_logging(bin_name: &'static str) {
        // configure colors for the whole line
        let colors_line = ColoredLevelConfig::new()
            .error(Color::Red)
            .warn(Color::Yellow)
            // we actually don't need to specify the color for debug and info, they are white by default
            .info(Color::White)
            .debug(Color::White)
            // depending on the terminals color scheme, this is the same as the background color
            .trace(Color::BrightBlack);

        // configure colors for the name of the level.
        // since almost all of them are the same as the color for the whole line, we
        // just clone `colors_line` and overwrite our changes
        let colors_level = colors_line.clone().info(Color::Green);
        // here we set up our fern Dispatch
        fern::Dispatch::new()
            .format(move |out, message, record| {
                out.finish(format_args!(
                    "{color_line}[{level}{color_line}] {message}\x1B[0m",
                    color_line = format_args!(
                        "\x1B[{}m",
                        colors_line.get_color(&record.level()).to_fg_str()
                    ),
                    level = colors_level.color(record.level()),
                    message = message,
                ));
            })
            // set the default log level. to filter out verbose log messages from dependencies, set
            // this to Warn and overwrite the log level for your crate.
            .level(log::LevelFilter::Warn)
            // change log levels for individual modules. Note: This looks for the record's target
            // field which defaults to the module path but can be overwritten with the `target`
            // parameter:
            // `info!(target="special_target", "This log message is about special_target");`
            .level_for(bin_name, log::LevelFilter::Info)
            // .level_for("pretty_colored", log::LevelFilter::Trace)
            // output to stdout
            .chain(std::io::stdout())
            .apply()
            .unwrap();
    }
}
