use std::env;
use std::process::{self, Command};

const HELP: &str = r#"Strip & embed docs from your rust codebase
Usage:
    cargo stripper [options]
Options:
    --embed                    Embed the docs from a docs.md file
    --strip                    Strip the docs from a package
    -h, --help                 Print this message
"#;

fn main() {
    pretty_env_logger::init();
    let path = env::current_exe()
        .expect("current executable path invalid")
        .with_file_name("stripper-driver");
    let mut args = std::env::args().skip(2).collect::<Vec<_>>();

    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("{}", HELP);
        return;
    }

    let mut operation = "strip";

    if let Some(pos) = args.iter().position(|a| a == "--embed" || a == "-e") {
        operation = "embed";
        args.remove(pos);
    }

    log::debug!("{}", operation);

    let mut cmd = Command::new("cargo");
    let code = cmd
        .env("RUSTC_WORKSPACE_WRAPPER", path)
        .env("STRIPPER_OPERATION", operation)
        .arg("fix")
        .args(&args)
        .spawn()
        .unwrap()
        .wait()
        .unwrap()
        .code()
        .unwrap_or(-1);
    process::exit(code);
}
