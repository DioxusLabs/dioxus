mod check;
mod issues;
mod metadata;

pub use check::check_file;
pub use issues::{Issue, IssueReport};

#[cfg(test)]
mod tests {
    use crate::metadata::{
        AnyLoopInfo, ClosureInfo, ConditionalInfo, ForInfo, HookInfo, IfInfo, LineColumn, LoopInfo,
        MatchInfo, Span, WhileInfo,
    };

    use super::*;

    #[test]
    fn test_no_issues() {
        let contents = r#"
            fn App(cx: Scope) -> Element {
                rsx! {
                    p { "Hello World" }
                }
            }
        "#;

        let report = check_file("app.rs".into(), contents);

        assert_eq!(report.issues, vec![]);
    }

    #[test]
    fn test_conditional_hook_if() {
        let contents = r#"
            fn App(cx: Scope) -> Element {
                if you_are_happy && you_know_it {
                    let something = use_state(cx, || "hands");
                    println!("clap your {something}")
                }
            }
        "#;

        let report = check_file("app.rs".into(), contents);

        assert_eq!(
            report.issues,
            vec![Issue::HookInsideConditional(
                HookInfo::new(
                    Span {
                        start: LineColumn {
                            line: 4,
                            column: 36
                        },
                        end: LineColumn {
                            line: 4,
                            column: 61
                        }
                    },
                    Span {
                        start: LineColumn {
                            line: 4,
                            column: 36
                        },
                        end: LineColumn {
                            line: 4,
                            column: 45
                        }
                    },
                    "use_state".to_string()
                ),
                ConditionalInfo::If(IfInfo::new(
                    Span {
                        start: LineColumn {
                            line: 3,
                            column: 16
                        },
                        end: LineColumn {
                            line: 6,
                            column: 17
                        }
                    },
                    Span {
                        start: LineColumn {
                            line: 3,
                            column: 16
                        },
                        end: LineColumn {
                            line: 3,
                            column: 18
                        }
                    }
                ))
            )],
        );
    }

    #[test]
    fn test_conditional_hook_match() {
        let contents = r#"
            fn App(cx: Scope) -> Element {
                match you_are_happy && you_know_it {
                    true => {
                        let something = use_state(cx, || "hands");
                        println!("clap your {something}")
                    }
                    false => {}
                }
            }
        "#;

        let report = check_file("app.rs".into(), contents);

        assert_eq!(
            report.issues,
            vec![Issue::HookInsideConditional(
                HookInfo::new(
                    Span {
                        start: LineColumn {
                            line: 5,
                            column: 40
                        },
                        end: LineColumn {
                            line: 5,
                            column: 65
                        }
                    },
                    Span {
                        start: LineColumn {
                            line: 5,
                            column: 40
                        },
                        end: LineColumn {
                            line: 5,
                            column: 49
                        }
                    },
                    "use_state".to_string()
                ),
                ConditionalInfo::Match(MatchInfo::new(
                    Span {
                        start: LineColumn {
                            line: 3,
                            column: 16
                        },
                        end: LineColumn {
                            line: 9,
                            column: 17
                        }
                    },
                    Span {
                        start: LineColumn {
                            line: 3,
                            column: 16
                        },
                        end: LineColumn {
                            line: 3,
                            column: 21
                        }
                    }
                ))
            )]
        );
    }

    #[test]
    fn test_for_loop_hook() {
        let contents = r#"
            fn App(cx: Scope) -> Element {
                for _name in &names {
                    let is_selected = use_state(cx, || false);
                    println!("selected: {is_selected}");
                }
            }
        "#;

        let report = check_file("app.rs".into(), contents);

        assert_eq!(
            report.issues,
            vec![Issue::HookInsideLoop(
                HookInfo::new(
                    Span {
                        start: LineColumn {
                            line: 4,
                            column: 38
                        },
                        end: LineColumn {
                            line: 4,
                            column: 61
                        }
                    },
                    Span {
                        start: LineColumn {
                            line: 4,
                            column: 38
                        },
                        end: LineColumn {
                            line: 4,
                            column: 47
                        }
                    },
                    "use_state".to_string()
                ),
                AnyLoopInfo::For(ForInfo::new(
                    Span {
                        start: LineColumn {
                            line: 3,
                            column: 16
                        },
                        end: LineColumn {
                            line: 6,
                            column: 17
                        }
                    },
                    Span {
                        start: LineColumn {
                            line: 3,
                            column: 16
                        },
                        end: LineColumn {
                            line: 3,
                            column: 19
                        }
                    }
                ))
            )]
        );
    }

    #[test]
    fn test_while_loop_hook() {
        let contents = r#"
            fn App(cx: Scope) -> Element {
                while true {
                    let something = use_state(cx, || "hands");
                    println!("clap your {something}")
                }
            }
        "#;

        let report = check_file("app.rs".into(), contents);

        assert_eq!(
            report.issues,
            vec![Issue::HookInsideLoop(
                HookInfo::new(
                    Span {
                        start: LineColumn {
                            line: 4,
                            column: 36
                        },
                        end: LineColumn {
                            line: 4,
                            column: 61
                        }
                    },
                    Span {
                        start: LineColumn {
                            line: 4,
                            column: 36
                        },
                        end: LineColumn {
                            line: 4,
                            column: 45
                        }
                    },
                    "use_state".to_string()
                ),
                AnyLoopInfo::While(WhileInfo::new(
                    Span {
                        start: LineColumn {
                            line: 3,
                            column: 16
                        },
                        end: LineColumn {
                            line: 6,
                            column: 17
                        }
                    },
                    Span {
                        start: LineColumn {
                            line: 3,
                            column: 16
                        },
                        end: LineColumn {
                            line: 3,
                            column: 21
                        }
                    }
                ))
            )],
        );
    }

    #[test]
    fn test_loop_hook() {
        let contents = r#"
            fn App(cx: Scope) -> Element {
                loop {
                    let something = use_state(cx, || "hands");
                    println!("clap your {something}")
                }
            }
        "#;

        let report = check_file("app.rs".into(), contents);

        assert_eq!(
            report.issues,
            vec![Issue::HookInsideLoop(
                HookInfo::new(
                    Span {
                        start: LineColumn {
                            line: 4,
                            column: 36
                        },
                        end: LineColumn {
                            line: 4,
                            column: 61
                        }
                    },
                    Span {
                        start: LineColumn {
                            line: 4,
                            column: 36
                        },
                        end: LineColumn {
                            line: 4,
                            column: 45
                        }
                    },
                    "use_state".to_string()
                ),
                AnyLoopInfo::Loop(LoopInfo::new(
                    Span {
                        start: LineColumn {
                            line: 3,
                            column: 16
                        },
                        end: LineColumn {
                            line: 6,
                            column: 17
                        }
                    },
                    Span {
                        start: LineColumn {
                            line: 3,
                            column: 16
                        },
                        end: LineColumn {
                            line: 3,
                            column: 20
                        }
                    }
                ))
            )],
        );
    }

    #[test]
    fn test_conditional_okay() {
        let contents = r#"
            fn App(cx: Scope) -> Element {
                let something = use_state(cx, || "hands");
                if you_are_happy && you_know_it {
                    println!("clap your {something}")
                }
            }
        "#;

        let report = check_file("app.rs".into(), contents);

        assert_eq!(report.issues, vec![]);
    }

    #[test]
    fn test_closure_hook() {
        let contents = r#"
            fn App(cx: Scope) -> Element {
                let _a = || {
                    let b = use_state(cx, || 0);
                    b.get()
                };
            }
        "#;

        let report = check_file("app.rs".into(), contents);

        assert_eq!(
            report.issues,
            vec![Issue::HookInsideClosure(
                HookInfo::new(
                    Span {
                        start: LineColumn {
                            line: 4,
                            column: 28
                        },
                        end: LineColumn {
                            line: 4,
                            column: 47
                        }
                    },
                    Span {
                        start: LineColumn {
                            line: 4,
                            column: 28
                        },
                        end: LineColumn {
                            line: 4,
                            column: 37
                        }
                    },
                    "use_state".to_string()
                ),
                ClosureInfo::new(Span {
                    start: LineColumn {
                        line: 3,
                        column: 25
                    },
                    end: LineColumn {
                        line: 6,
                        column: 17
                    }
                })
            )]
        );
    }

    #[test]
    fn test_hook_outside_component() {
        let contents = r#"
            fn not_component_or_hook(cx: Scope) {
                let _a = use_state(cx, || 0);
            }
        "#;

        let report = check_file("app.rs".into(), contents);

        assert_eq!(
            report.issues,
            vec![Issue::HookOutsideComponent(HookInfo::new(
                Span {
                    start: LineColumn {
                        line: 3,
                        column: 25
                    },
                    end: LineColumn {
                        line: 3,
                        column: 44
                    }
                },
                Span {
                    start: LineColumn {
                        line: 3,
                        column: 25
                    },
                    end: LineColumn {
                        line: 3,
                        column: 34
                    }
                },
                "use_state".to_string()
            ))]
        );
    }

    #[test]
    fn test_hook_inside_hook() {
        let contents = r#"
            fn use_thing(cx: Scope) {
                let _a = use_state(cx, || 0);
            }
        "#;

        let report = check_file("app.rs".into(), contents);

        assert_eq!(report.issues, vec![]);
    }
}
