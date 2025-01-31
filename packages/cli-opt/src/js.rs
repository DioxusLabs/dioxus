use std::path::Path;
use std::path::PathBuf;

use anyhow::Context;
use manganis_core::JsAssetOptions;
use swc_common::errors::Emitter;
use swc_common::errors::Handler;
use swc_common::input::SourceFileInput;
use swc_ecma_minifier::option::{ExtraOptions, MinifyOptions};
use swc_ecma_parser::lexer::Lexer;
use swc_ecma_parser::Parser;
use swc_ecma_transforms_base::fixer::fixer;
use swc_ecma_visit::VisitMutWith;

use std::collections::HashMap;

use anyhow::Error;
use swc_bundler::{Bundler, Config, Load, ModuleData, ModuleRecord};
use swc_common::{
    errors::HANDLER, sync::Lrc, FileName, FilePathMapping, Globals, Mark, SourceMap, Span, GLOBALS,
};
use swc_ecma_ast::*;
use swc_ecma_codegen::text_writer::JsWriter;
use swc_ecma_loader::{resolvers::node::NodeModulesResolver, TargetEnv};
use swc_ecma_parser::{parse_file_as_module, Syntax};

struct TracingEmitter;

impl Emitter for TracingEmitter {
    fn emit(&mut self, db: &swc_common::errors::DiagnosticBuilder<'_>) {
        match db.level {
            swc_common::errors::Level::Bug
            | swc_common::errors::Level::Fatal
            | swc_common::errors::Level::PhaseFatal
            | swc_common::errors::Level::Error => tracing::error!("{}", db.message()),
            swc_common::errors::Level::Warning
            | swc_common::errors::Level::FailureNote
            | swc_common::errors::Level::Cancelled => tracing::warn!("{}", db.message()),
            swc_common::errors::Level::Note | swc_common::errors::Level::Help => {
                tracing::trace!("{}", db.message())
            }
        }
    }
}

fn bundle_js_to_writer(
    file: PathBuf,
    bundle: bool,
    minify: bool,
    write_to: &mut impl std::io::Write,
) -> anyhow::Result<()> {
    let globals = Globals::new();
    let handler = Handler::with_emitter_and_flags(Box::new(TracingEmitter), Default::default());
    GLOBALS.set(&globals, || {
        HANDLER.set(&handler, || {
            bundle_js_to_writer_inside_handler(&globals, file, bundle, minify, write_to)
        })
    })
}

fn bundle_js_to_writer_inside_handler(
    globals: &Globals,
    file: PathBuf,
    bundle: bool,
    minify: bool,
    write_to: &mut impl std::io::Write,
) -> anyhow::Result<()> {
    let cm = Lrc::new(SourceMap::new(FilePathMapping::empty()));
    let mut module = if bundle {
        let node_resolver = NodeModulesResolver::new(TargetEnv::Browser, Default::default(), true);
        let mut bundler = Bundler::new(
            globals,
            cm.clone(),
            PathLoader { cm: cm.clone() },
            node_resolver,
            Config {
                require: true,
                ..Default::default()
            },
            Box::new(Hook),
        );
        let mut entries = HashMap::default();
        entries.insert("main".to_string(), FileName::Real(file));

        let mut bundles = bundler
            .bundle(entries)
            .context("failed to bundle javascript with swc")?;
        // Since we only inserted one entry, there should only be one bundle in the output
        let bundle = bundles
            .pop()
            .ok_or_else(|| anyhow::anyhow!("swc did not output any bundles"))?;
        bundle.module
    } else {
        let fm = cm.load_file(Path::new(&file)).expect("Failed to load file");

        let lexer = Lexer::new(
            Default::default(),
            Default::default(),
            SourceFileInput::from(&*fm),
            None,
        );
        let mut parser = Parser::new_from(lexer);

        parser.parse_module().map_err(|err| {
            HANDLER.with(|handler| {
                let message = err.into_diagnostic(handler).message();
                anyhow::anyhow!("{}", message)
            })
        })?
    };

    if minify {
        module = swc_ecma_minifier::optimize(
            std::mem::take(&mut module).into(),
            cm.clone(),
            None,
            None,
            &MinifyOptions {
                rename: true,
                compress: None,
                mangle: None,
                ..Default::default()
            },
            &ExtraOptions {
                unresolved_mark: Mark::new(),
                top_level_mark: Mark::new(),
                mangle_name_cache: None,
            },
        )
        .expect_module();
        module.visit_mut_with(&mut fixer(None));
    }

    let mut emitter = swc_ecma_codegen::Emitter {
        cfg: swc_ecma_codegen::Config::default().with_minify(minify),
        cm: cm.clone(),
        comments: None,
        wr: Box::new(JsWriter::new(cm, "\n", write_to, None)),
    };

    emitter.emit_module(&module)?;

    Ok(())
}

struct PathLoader {
    cm: Lrc<SourceMap>,
}

impl Load for PathLoader {
    fn load(&self, file: &FileName) -> anyhow::Result<ModuleData> {
        let file = match file {
            FileName::Real(v) => v,
            _ => anyhow::bail!("Only real files are supported"),
        };

        let fm = self.cm.load_file(file)?;

        let module = HANDLER.with(|handler| {
            parse_file_as_module(
                &fm,
                Syntax::Es(Default::default()),
                Default::default(),
                None,
                &mut Vec::new(),
            )
            .map_err(|err| {
                let message = err.into_diagnostic(handler).message();
                anyhow::anyhow!("{}", message)
            })
            .context("Failed to parse javascript")
        })?;

        Ok(ModuleData {
            fm,
            module,
            helpers: Default::default(),
        })
    }
}

// Adapted from https://github.com/swc-project/swc/blob/624680b7896cef9d8e30bd5ff910538298016974/bindings/binding_core_node/src/bundle.rs#L266-L302
struct Hook;

impl swc_bundler::Hook for Hook {
    fn get_import_meta_props(
        &self,
        span: Span,
        module_record: &ModuleRecord,
    ) -> Result<Vec<KeyValueProp>, Error> {
        let file_name = module_record.file_name.to_string();

        Ok(vec![
            KeyValueProp {
                key: PropName::Ident(IdentName::new("url".into(), span)),
                value: Box::new(Expr::Lit(Lit::Str(Str {
                    span,
                    raw: None,
                    value: file_name.into(),
                }))),
            },
            KeyValueProp {
                key: PropName::Ident(IdentName::new("main".into(), span)),
                value: Box::new(if module_record.is_entry {
                    Expr::Member(MemberExpr {
                        span,
                        obj: Box::new(Expr::MetaProp(MetaPropExpr {
                            span,
                            kind: MetaPropKind::ImportMeta,
                        })),
                        prop: MemberProp::Ident(IdentName::new("main".into(), span)),
                    })
                } else {
                    Expr::Lit(Lit::Bool(Bool { span, value: false }))
                }),
            },
        ])
    }
}

pub(crate) fn process_js(
    js_options: &JsAssetOptions,
    source: &Path,
    output_path: &Path,
    bundle: bool,
) -> anyhow::Result<()> {
    let mut writer = std::io::BufWriter::new(std::fs::File::create(output_path)?);
    if js_options.minified() {
        if let Err(err) = bundle_js_to_writer(source.to_path_buf(), bundle, true, &mut writer) {
            tracing::error!("Failed to minify js. Falling back to non-minified: {err}");
        }
    } else {
        let mut source_file = std::fs::File::open(source)?;
        std::io::copy(&mut source_file, &mut writer).with_context(|| {
            format!(
                "Failed to write js to output location: {}",
                output_path.display()
            )
        })?;
    }

    Ok(())
}
