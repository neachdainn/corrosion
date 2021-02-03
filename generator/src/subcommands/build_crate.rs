use clap::{App, Arg, ArgMatches, SubCommand};
use std::{env, process};

// build-crate Subcommand
pub const BUILD_CRATE: &str = "build-crate";

// build-crate options
const RELEASE: &str = "release";
const PACKAGE: &str = "package";
const TARGET: &str = "target";

pub fn subcommand() -> App<'static, 'static> {
    SubCommand::with_name(BUILD_CRATE)
        .arg(Arg::with_name(RELEASE).long("release"))
        .arg(
            Arg::with_name(TARGET)
                .long("target")
                .value_name("TRIPLE")
                .required(true)
                .help("The target triple to build for"),
        )
        .arg(
            Arg::with_name(PACKAGE)
                .long("package")
                .value_name("PACKAGE")
                .required(true)
                .help("The name of the package being built with cargo"),
        )
}

pub fn invoke(
    args: &crate::GeneratorSharedArgs,
    matches: &ArgMatches,
) -> Result<(), Box<dyn std::error::Error>> {
    let target = matches.value_of(TARGET).unwrap();

    let mut cargo = process::Command::new(&args.cargo_executable);

    cargo.args(&[
        "build",
        "--target",
        target,
        "--package",
        matches.value_of(PACKAGE).unwrap(),
        "--manifest-path",
        args.manifest_path.to_str().unwrap(),
    ]);

    if args.verbose {
        cargo.arg("--verbose");
    }

    if matches.is_present(RELEASE) {
        cargo.arg("--release");
    }

    let languages: Vec<String> = env::var("CMAKECARGO_LINKER_LANGUAGES")
        .unwrap_or("".to_string())
        .split(";")
        .map(Into::into)
        .collect();

    if !languages.is_empty() {
        let mut rustflags = env::var("RUSTFLAGS").unwrap_or_default();
        rustflags += " -C default-linker-libraries=yes";

        // This loop gets the highest preference link language to use for the linker
        let mut highest_preference: Option<(Option<i32>, &str)> = None;
        for language in &languages {
            highest_preference = Some(
                if let Ok(preference) =
                    env::var(&format!("CMAKECARGO_{}_LINKER_PREFERENCE", language))
                {
                    let preference = preference
                        .parse()
                        .expect("cmake-cargo internal error: PREFERENCE wrong format");
                    match highest_preference {
                        Some((Some(current), language)) if current > preference => {
                            (Some(current), language)
                        }
                        _ => (Some(preference), &language),
                    }
                } else if let Some(p) = highest_preference {
                    p
                } else {
                    (None, &language)
                },
            );
        }

        // If a preferred compiler is selected, use it as the linker so that the correct standard, implicit libraries
        // are linked in.
        if let Some((_, language)) = highest_preference {
            if let Ok(compiler) = env::var(&format!("CMAKECARGO_{}_COMPILER", language)) {
                let linker_arg = format!(
                    "CARGO_TARGET_{}_LINKER",
                    target.replace("-", "_").to_uppercase()
                );

                cargo.env(linker_arg, compiler);
            }

            if let Ok(target) = env::var(format!("CMAKECARGO_{}_COMPILER_TARGET", language)) {
                rustflags += " -C link-args=--target=";
                rustflags += &target;
            }
        }

        cargo.env("RUSTFLAGS", rustflags);
    }

    if args.verbose {
        println!("Corrosion: {:?}", cargo);
    }

    process::exit(if cargo.status()?.success() { 0 } else { 1 });
}
