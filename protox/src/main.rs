use std::{fs, path::PathBuf};

use clap::Parser;
use miette::Result;
use prost::Message;
use protox::compile;

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(value_name = "PROTO_FILES", required = true, value_parser)]
    files: Vec<PathBuf>,
    #[clap(
        short = 'I',
        long = "include",
        visible_alias = "proto_path",
        value_name = "PATH",
        default_value = ".",
        value_parser
    )]
    includes: Vec<PathBuf>,
    #[clap(
        short = 'o',
        long = "output",
        visible_alias = "descriptor_set_out",
        value_name = "PATH",
        value_parser
    )]
    output: Option<PathBuf>,
}

pub fn main() -> Result<()> {
    miette::set_panic_hook();

    let args = Args::parse();
    let files = compile(args.files, args.includes)?;
    // TODO include_source_info flag, include_imports
    if let Some(output) = args.output {
        fs::write(output, files.encode_to_vec()).map_err(|err| miette::miette!(err))?;
    }
    Ok(())
}
