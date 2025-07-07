pub(crate) type Result<T, E = Error> = std::result::Result<T, E>;
pub use anyhow::Error;

pub fn log_stacktrace(err: &anyhow::Error) -> String {
    let mut trace = format!("{err}",);

    for (idx, cause) in err.chain().enumerate().skip(1) {
        trace.push_str(&format!(
            "\n- {IDX_STYLE}{idx}{IDX_STYLE:#}: {ERROR_STYLE}{}{ERROR_STYLE:#}",
            cause,
            IDX_STYLE = crate::styles::GLOW_STYLE,
            ERROR_STYLE = crate::styles::ERROR_STYLE
        ));
    }

    if crate::VERBOSITY.get().unwrap().trace {
        trace.push_str(&format!("\nBacktrace:\n{}", err.backtrace()));
    }

    trace
}
