use std::{fs, path::PathBuf};

use clap::Parser;
use miette::Result;
use protox::Compiler;

#[derive(Debug, Parser)]
pub struct Args {
    /// The source file(s) to compile
    #[clap(value_name = "PROTO_FILES", required = true, value_parser)]
    files: Vec<PathBuf>,
    /// The directory in which to search for imports.
    #[clap(
        short = 'I',
        long = "include",
        visible_alias = "proto_path",
        value_name = "PATH",
        default_value = ".",
        value_parser
    )]
    includes: Vec<PathBuf>,
    /// The output path to write a file descriptor set to.
    #[clap(
        short = 'o',
        long = "output",
        visible_alias = "descriptor_set_out",
        value_name = "PATH",
        value_parser
    )]
    output: Option<PathBuf>,
    /// If set, includes source code information in the output file descriptor set.
    #[clap(long, visible_alias = "include_source_info")]
    include_source_info: bool,
    /// If set, all dependencies of the input files are output, so that the file descriptor set is self-contained.
    #[clap(long, visible_alias = "include_imports")]
    include_imports: bool,
}

pub fn main() -> Result<()> {
    miette::set_panic_hook();

    let args = Args::parse();
    let mut compiler = Compiler::new(args.includes)?;
    compiler.include_imports(args.include_imports);
    compiler.include_source_info(args.include_source_info);
    for file in args.files {
        compiler.open_file(file)?;
    }
    if let Some(output) = args.output {
        fs::write(output, compiler.encode_file_descriptor_set())
            .map_err(|err| miette::miette!(err))?;
    }
    Ok(())
}
