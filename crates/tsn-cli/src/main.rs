mod args;
mod bench;
mod bench_output;
mod disasm;
mod doctor;
mod error;
mod import_collector;
mod module_precompile;
mod pipeline;
use args::Command;
use std::process;

fn main() {
    let command = Command::parse().unwrap_or_else(|e| {
        eprintln!("{}", e);
        process::exit(e.exit_code);
    });

    let result = match command {
        Command::Version => {
            println!("{}", Command::version_string());
            Ok(())
        }
        Command::Run(opts) => pipeline::run(&opts),
        Command::Bench(opts) => bench::run_bench(
            &opts.file_path,
            opts.runs,
            &opts.debug,
            opts.no_run,
            opts.with_output,
        ),
        Command::Doctor => doctor::run_doctor(),
        Command::Disasm(opts) => {
            pipeline::compile_file(&opts.file_path, false, &opts.debug).map(|proto| {
                if !opts.debug.bytecode {
                    disasm::print(&proto);
                }
            })
        }
    };

    if let Err(e) = result {
        eprintln!("{}", e);

        process::exit(0);
    }
}
