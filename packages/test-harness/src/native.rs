use super::TestCase;
use std::{cell::RefCell, panic, time::Instant};
use tokio::runtime::Builder;

thread_local! {
    static PANIC_MESSAGE: RefCell<Option<String>> = const { RefCell::new(None) };
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Outcome {
    Passed,
    Failed,
    Ignored,
}

pub fn run() {
    let cases = inventory::iter::<TestCase>.into_iter().collect::<Vec<_>>();
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let format = value_after(&args, "--format").unwrap_or("pretty");
    let exact = values_after(&args, "--exact");
    let filters = positional_filters(&args);
    let include_ignored = args.iter().any(|arg| arg == "--include-ignored");
    let ignored_only = args.iter().any(|arg| arg == "--ignored");

    let selected = cases
        .iter()
        .copied()
        .filter(|case| {
            let name = display_name(case.name);
            let exact_match = exact.is_empty() || exact.contains(&name);
            let filter_match =
                filters.is_empty() || filters.iter().any(|filter| name.contains(filter));
            exact_match && filter_match && (!ignored_only || case.ignore)
        })
        .collect::<Vec<_>>();

    if args.iter().any(|arg| arg == "--list") {
        match format {
            "json" => {
                for case in selected {
                    println!(
                        "{}",
                        serde_json::json!({
                            "name": display_name(case.name), "file": case.file,
                            "line": case.line, "ignore": case.ignore,
                            "should_panic": case.should_panic, "tags": case.tags,
                            "platforms": case.platforms, "runnable": case.run.is_some()
                        })
                    );
                }
            }
            _ => {
                for case in selected {
                    println!("{}: test", display_name(case.name));
                }
            }
        }
        return;
    }

    let started = Instant::now();
    let mut results = Vec::new();
    let mut failures = Vec::new();
    if format == "json" {
        println!(
            "{}",
            serde_json::json!({"type":"suite", "event":"started", "test_count":selected.len()})
        );
    } else {
        println!("running {} tests", selected.len());
    }

    for case in selected.iter().copied() {
        let name = display_name(case.name).to_string();
        if case.run.is_none() {
            if format == "json" {
                println!(
                    "{}",
                    serde_json::json!({"type":"test", "event":"ignored", "name":name, "reason":"not_runnable"})
                );
            } else {
                println!("test {name} ... ignored, not runnable on this target");
            }
            results.push(Outcome::Ignored);
            continue;
        }
        if case.ignore && !include_ignored {
            emit_json_or_pretty(format, &name, Outcome::Ignored, None, 0.0, false);
            results.push(Outcome::Ignored);
            continue;
        }

        let test_started = Instant::now();
        if format == "json" {
            println!(
                "{}",
                serde_json::json!({"type":"test", "event":"started", "name":name})
            );
        }
        let outcome = run_case(case);
        let elapsed = test_started.elapsed().as_secs_f64();
        if outcome.0 == Outcome::Failed {
            failures.push((
                name.clone(),
                outcome.1.clone().unwrap_or_else(|| "test failed".into()),
            ));
        }
        emit_json_or_pretty(
            format,
            &name,
            outcome.0,
            outcome.1.as_deref(),
            elapsed,
            true,
        );
        results.push(outcome.0);
    }

    let passed = results.iter().filter(|o| **o == Outcome::Passed).count();
    let failed = results.iter().filter(|o| **o == Outcome::Failed).count();
    let ignored = results.iter().filter(|o| **o == Outcome::Ignored).count();
    let filtered = cases.len().saturating_sub(results.len());
    let elapsed = started.elapsed().as_secs_f64();
    if format == "json" {
        println!(
            "{}",
            serde_json::json!({"type":"suite", "event":if failed == 0 {"ok"} else {"failed"},
                "passed":passed, "failed":failed, "ignored":ignored, "measured":0,
                "filtered_out":filtered, "exec_time":elapsed})
        );
    } else {
        if !failures.is_empty() {
            println!("\nfailures:");
            for (name, message) in &failures {
                println!("---- {name} ----\n{message}");
            }
        }
        println!(
            "\ntest result: {}. {passed} passed; {failed} failed; {ignored} ignored; 0 measured; {filtered} filtered out; finished in {elapsed:.2}s",
            if failed == 0 { "ok" } else { "FAILED" }
        );
    }
    if failed > 0 {
        std::process::exit(101);
    }
}

fn run_case(case: &TestCase) -> (Outcome, Option<String>) {
    let previous = panic::take_hook();
    PANIC_MESSAGE.with(|message| *message.borrow_mut() = None);
    panic::set_hook(Box::new(|info| {
        let message = if let Some(s) = info.payload().downcast_ref::<&str>() {
            (*s).to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            info.to_string()
        };
        PANIC_MESSAGE.with(|stored| *stored.borrow_mut() = Some(message));
    }));

    let runtime = Builder::new_current_thread().enable_time().build();
    let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        let future = (case.run.expect("not runnable on this target"))();
        runtime.as_ref().expect("runtime").block_on(async {
            if let Some(timeout) = case.timeout_ms {
                tokio::time::timeout(std::time::Duration::from_millis(timeout), future)
                    .await
                    .map_err(|_| "timed out".to_string())?
                    .pipe(Ok)
            } else {
                future.await;
                Ok(())
            }
        })
    }));
    let panic_message = PANIC_MESSAGE.with(|message| message.borrow_mut().take());
    panic::set_hook(previous);

    let message = match result {
        Ok(Ok(())) if case.should_panic => Some("test did not panic".into()),
        Ok(Err(message)) => Some(message),
        Ok(Ok(())) => None,
        Err(_) => panic_message.or_else(|| Some("test panicked".into())),
    };
    let passed = message.is_none();
    let passed = if case.should_panic { !passed } else { passed };
    if passed {
        (Outcome::Passed, None)
    } else {
        (Outcome::Failed, message)
    }
}

fn emit_json_or_pretty(
    format: &str,
    name: &str,
    outcome: Outcome,
    message: Option<&str>,
    elapsed: f64,
    started: bool,
) {
    if format == "json" {
        if started {
            println!(
                "{}",
                serde_json::json!({"type":"test", "name":name, "event":match outcome { Outcome::Passed=>"ok", Outcome::Failed=>"failed", Outcome::Ignored=>"ignored"}, "exec_time":elapsed, "stdout":message.unwrap_or("")})
            );
        }
    } else {
        println!(
            "test {name} ... {}",
            match outcome {
                Outcome::Passed => "ok",
                Outcome::Failed => "FAILED",
                Outcome::Ignored => "ignored",
            }
        );
    }
}

fn display_name(name: &str) -> &str {
    name.split_once("::").map(|(_, rest)| rest).unwrap_or(name)
}

fn value_after<'a>(args: &'a [String], key: &str) -> Option<&'a str> {
    args.windows(2)
        .find(|pair| pair[0] == key)
        .map(|pair| pair[1].as_str())
}

fn values_after<'a>(args: &'a [String], key: &str) -> Vec<&'a str> {
    args.windows(2)
        .filter(|pair| pair[0] == key)
        .map(|pair| pair[1].as_str())
        .collect()
}

fn positional_filters(args: &[String]) -> Vec<&str> {
    let mut skip_next = false;
    args.iter()
        .filter_map(|arg| {
            if skip_next {
                skip_next = false;
                return None;
            }
            if arg.starts_with('-') {
                skip_next = matches!(arg.as_str(), "--format" | "--exact" | "--test-threads");
                return None;
            }
            Some(arg.as_str())
        })
        .collect()
}

trait Pipe: Sized {
    fn pipe<T>(self, f: impl FnOnce(Self) -> T) -> T {
        f(self)
    }
}
impl<T> Pipe for T {}
