use crate::error::CliError;

type CliResult<T> = Result<T, CliError>;

/// Prints runtime/install diagnostics to help validate local setup.
pub fn run_doctor() -> CliResult<()> {
    let exe = std::env::current_exe().ok();
    let home = tsn_core::paths::tsn_home_dir();
    let cache = tsn_core::paths::tsn_cache_dir();
    let stdlib = tsn_checker::module_resolver::stdlib_dir();

    println!("TSN Doctor");
    println!("  version: {}", env!("CARGO_PKG_VERSION"));
    if let Some(exe_path) = exe {
        println!("  exe: {}", exe_path.display());
    } else {
        println!("  exe: <unavailable>");
    }

    println!("  TSN_HOME: {}", home.display());
    println!("  cache dir: {}", cache.display());

    if let Ok(raw) = std::env::var("TSN_STDLIB") {
        println!("  TSN_STDLIB env: {}", raw);
    } else {
        println!("  TSN_STDLIB env: <not set>");
    }

    match stdlib {
        Some(path) => {
            println!("  stdlib: {} (ok)", path.display());
            Ok(())
        }
        None => Err(CliError::fatal(
            "stdlib not found. Run installer script or set TSN_STDLIB to a valid directory.",
        )),
    }
}
