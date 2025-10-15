pub(crate) type Result<T, E = Error> = std::result::Result<T, E>;
pub use anyhow::Error;
use itertools::Itertools;

pub fn log_stacktrace(err: &anyhow::Error, padding: usize) -> String {
    let mut trace = format!("{err}",);

    for (idx, cause) in err.chain().enumerate().skip(1) {
        trace.push_str(&format!(
            "\n{}{IDX_STYLE}{idx}{IDX_STYLE:#}: {}",
            " ".repeat(padding),
            cause
                .to_string()
                .lines()
                .enumerate()
                .map(|(idx, line)| {
                    if idx == 0 {
                        line.to_string()
                    } else {
                        format!("{}{}", " ".repeat(padding + 3), line)
                    }
                })
                .join("\n"),
            IDX_STYLE = crate::styles::GLOW_STYLE,
        ));
    }

    if crate::verbosity_or_default().trace {
        trace.push_str(&format!("\nBacktrace:\n{}", err.backtrace()));
    }

    trace
}
