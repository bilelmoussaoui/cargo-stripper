#![feature(rustc_private)]

extern crate rustc_ast;
extern crate rustc_driver;
extern crate rustc_lint;
#[macro_use]
extern crate rustc_session;
extern crate rustc_interface;

use std::{
    env,
    path::{Path, PathBuf},
    process::{exit, Command},
};

use rustc_ast::{ast, visit, AstDeref};
use rustc_interface::interface::Config;

use rustc_lint::{EarlyContext, EarlyLintPass};

declare_lint_pass!(RustdocStripperPass => []);

impl EarlyLintPass for RustdocStripperPass {
    fn check_crate(&mut self, _ctx: &EarlyContext<'_>, krate: &ast::Crate) {
        let mut visitor = RustdocStripperVisitor {};
        visit::walk_crate(&mut visitor, krate);
    }
}
/// Helpers
/*
fn span_to_path(span: Span) -> Option<PathBuf>{
    let filename = span.filename(sess);
    if let rustc_span::FileName::Real(file) = filename {
        file.into_local_path()
    } else {
        None
    }
}
*/
struct RustdocStripper {
    is_embed: bool,
}

impl rustc_driver::Callbacks for RustdocStripper {
    fn config(&mut self, config: &mut Config) {
        let previous = config.register_lints.take();

        config.register_lints = Some(Box::new(move |sess, lint_store| {
            // technically we're ~guaranteed that this is none but might as well call anything that
            // is there already. Certainly it can't hurt.
            if let Some(previous) = &previous {
                (previous)(sess, lint_store);
            }

            lint_store.register_early_pass(|| Box::new(RustdocStripperPass));
        }));
    }
}

struct RustdocStripperVisitor {}

impl<'ast> rustc_ast::visit::Visitor<'ast> for RustdocStripperVisitor {
    fn visit_item(&mut self, item: &'ast ast::Item) {
        use ast::ItemKind::*;
        match &item.kind {
            Struct(_, _) => {
                for attr in &item.attrs {
                    self.visit_attribute(attr);
                }
                println!("found a struct {:#?}", item.ident.name.as_str());
            }
            Enum(_, _) => {
                for attr in &item.attrs {
                    self.visit_attribute(attr);
                }
                println!("found an enum {:#?}", item.ident.name.as_str())
            }
            Mod(_, kind) => {
                if let ast::ModKind::Loaded(items, _, _) = kind {
                    for item in items {
                        self.visit_item(item);
                    }
                }
            }
            Trait(t) => {
                for item in &t.items {
                    if let ast::AssocItemKind::Fn(_) = item.kind {
                        println!("{:#?}", item.ident.name.as_str());
                        for attr in &item.attrs {
                            self.visit_attribute(attr);
                        }
                    }
                }
                println!("found a trait {:#?}", item.ident.name.as_str())
            }
            Impl(i) => {
                println!("found a impl {:#?}", item.ident.name.as_str());
                for item in &i.items {
                    if let ast::AssocItemKind::Fn(_) = item.kind {
                        println!("{:#?}", item.ident.name.as_str());
                        for attr in &item.attrs {
                            self.visit_attribute(attr);
                        }
                    }
                }
            }
            Fn(_) => {
                for attr in &item.attrs {
                    self.visit_attribute(attr);
                }
                println!("found a function {:#?}", item.ident.name.as_str());
            }
            _ => (),
        };
    }

    fn visit_attribute(&mut self, attr: &'ast ast::Attribute) {
        if attr.is_doc_comment() {
            println!("{:#?}", attr.doc_str());
        }
    }
}

fn toolchain_path(home: Option<String>, toolchain: Option<String>) -> Option<PathBuf> {
    home.and_then(|home| {
        toolchain.map(|toolchain| {
            let mut path = PathBuf::from(home);
            path.push("toolchains");
            path.push(toolchain);
            path
        })
    })
}

fn main() {
    rustc_driver::init_rustc_env_logger();

    let mut args: Vec<String> = std::env::args().collect();
    let is_embed = env::var("STRIPPER_OPERATION")
        .ok()
        .as_deref()
        .map(|o| o == "embed")
        .unwrap_or_default();

    let wrapper_mode =
        args.get(1).map(Path::new).and_then(Path::file_stem) == Some("rustc".as_ref());
    let sys_root = std::env::var("SYSROOT")
        .ok()
        .map(PathBuf::from)
        .or_else(|| {
            let home = std::env::var("RUSTUP_HOME")
                .or_else(|_| std::env::var("MULTIRUST_HOME"))
                .ok();
            let toolchain = std::env::var("RUSTUP_TOOLCHAIN")
                .or_else(|_| std::env::var("MULTIRUST_TOOLCHAIN"))
                .ok();
            toolchain_path(home, toolchain)
        })
        .or_else(|| {
            Command::new("rustc")
                .arg("--print")
                .arg("sysroot")
                .output()
                .ok()
                .and_then(|out| String::from_utf8(out.stdout).ok())
                .map(|s| PathBuf::from(s.trim()))
        })
        .or_else(|| option_env!("SYSROOT").map(PathBuf::from))
        .or_else(|| {
            let home = option_env!("RUSTUP_HOME")
                .or(option_env!("MULTIRUST_HOME"))
                .map(ToString::to_string);
            let toolchain = option_env!("RUSTUP_TOOLCHAIN")
                .or(option_env!("MULTIRUST_TOOLCHAIN"))
                .map(ToString::to_string);
            toolchain_path(home, toolchain)
        })
        .map(|pb| pb.to_string_lossy().to_string())
        .expect(
            "need to specify SYSROOT env var during clippy compilation, or use rustup or multirust",
        );
    if wrapper_mode {
        // we still want to be able to invoke it normally though
        args.remove(1);
    }
    args.extend(vec!["--sysroot".into(), sys_root]);
    args.extend(vec!["--cfg".into(), r#"feature="cargo-stripper""#.into()]);
    exit(rustc_driver::catch_with_exit_code(move || {
        rustc_driver::RunCompiler::new(&args, &mut RustdocStripper { is_embed }).run()
    }));
}
