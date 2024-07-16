use std::fs;

use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
struct Fit {
    #[clap(subcommand)]
    command: FitCommands,
}

#[derive(Subcommand)]
enum FitCommands {
    #[clap(subcommand = "init")]
    Init,
    #[clap(subcommand = "clone")]
    Clone(CloneArgs),
    #[clap(subcommand = "log")]
    Log,
    #[clap(subcommand = "add")]
    Add(AddArgs),
    #[clap(subcommand = "rm")]
    Rm(RmArgs),
    #[clap(subcommand = "commit")]
    Commit(CommitArgs),
}

#[derive(Args)]
struct CloneArgs {
    #[clap(short, long)]
    url: String,
}

#[derive(Args)]
struct AddArgs {
    #[clap(short, long)]
    file: String,
}

#[derive(Args)]
struct RmArgs {
    #[clap(short, long)]
    file: String,
}

#[derive(Args)]
struct CommitArgs {
    #[clap(short, long)]
    message: String,
}

fn main() {
    let args = Fit::parse();
    match args.command {
        FitCommands::Init => init_workflow(),
        FitCommands::Clone(clone_args) => println!("Cloning from {}... (stub)", clone_args.url),
        FitCommands::Log => println!("Listing fit log... (stub)"),
        FitCommands::Add(add_args) => println!("Adding {} to fit... (stub)", add_args.file),
        FitCommands::Rm(rm_args) => println!("Removing {} from fit... (stub)", rm_args.file),
        FitCommands::Commit(commit_args) => println!("Committing with message: {}... (stub)", commit_args.message),
    }
}

fn init_workflow(){
    fs::create_dir(".fit").unwrap();
    fs::create_dir(".fit/objects").unwrap();
    fs::create_dir(".fit/refs").unwrap();
    fs::write(".fit/HEAD", "ref:refs/heads/master\n").unwrap();
    println!("Initialized fit repository")
}
