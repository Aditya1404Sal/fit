// use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
// use std::path::PathBuf;
use sha1::{Sha1, Digest};
use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
struct Fit {
    #[clap(subcommand)]
    command: FitCommands,
}

#[derive(Subcommand)]
enum FitCommands {
    Init,
    Clone(CloneArgs),
    Log,
    Add(AddArgs),
    Rm(RmArgs),
    Commit(CommitArgs),
    Catfile(FileArgs),
}

#[derive(Args)]
struct CloneArgs {
    #[clap(short, long)]
    url: String,
}

#[derive(Args)]
struct FileArgs {
    #[clap(short, long)]
    hash: String,
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
        FitCommands::Clone(clone_args) => clone_workflow(clone_args),
        FitCommands::Log => log_workflow(),
        FitCommands::Add(add_args) => add_workflow(add_args),
        FitCommands::Rm(rm_args) => rm_workflow(rm_args),
        FitCommands::Commit(commit_args) => commit_workflow(commit_args),
        FitCommands::Catfile(file_args) => cat_file_workflow(file_args),
    }
}

fn init_workflow() {
    fs::create_dir(".fit").unwrap();
    fs::create_dir(".fit/objects").unwrap();
    fs::create_dir(".fit/refs").unwrap();
    fs::write(".fit/HEAD", "ref: refs/heads/master\n").unwrap();
    File::create(".fit/index").unwrap();
    println!("Initialized fit repository");
}

fn clone_workflow(args: CloneArgs) {
    // TBD
}

fn log_workflow() {
    // TBD
}

fn add_workflow(args: AddArgs) {
    let path = Path::new(&args.file);
    if path.exists() {
        let mut file = File::open(path).unwrap();
        let mut contents = Vec::new();
        file.read_to_end(&mut contents).unwrap();

        let mut hasher = Sha1::new();
        hasher.update(&contents);
        let hash = hasher.finalize();
        let hash_hex = format!("{:x}", hash);

        let object_path = format!(".fit/objects/{}", hash_hex);
        fs::write(&object_path, &contents).unwrap();

        let mut index = fs::OpenOptions::new().append(true).open(".fit/index").unwrap();
        writeln!(index, "{} {}", hash_hex, args.file).unwrap();

        println!("Added {} to fit", args.file);
    } else {
        println!("File {} not found", args.file);
    }
}

fn rm_workflow(args: RmArgs) {
    let path = Path::new(&args.file);
    if path.exists() {
        // Update the index to reflect the removal
        let index_path = ".fit/index";
        let index_content = fs::read_to_string(index_path).unwrap();
        let updated_content: Vec<String> = index_content
            .lines()
            .filter(|line| !line.contains(&args.file))
            .map(|line| line.to_string())
            .collect();
        fs::write(index_path, updated_content.join("\n")).unwrap();

        // Remove the object file corresponding to the hash
        let mut file = File::open(path).unwrap();
        let mut contents = Vec::new();
        file.read_to_end(&mut contents).unwrap();

        let mut hasher = Sha1::new();
        hasher.update(&contents);
        let hash = hasher.finalize();
        let hash_hex = format!("{:x}", hash);

        let object_path = format!(".fit/objects/{}", hash_hex);
        if Path::new(&object_path).exists() {
            fs::remove_file(&object_path).unwrap();
            println!("Removed object {} from fit", hash_hex);
        }

        println!("Removed {} from fit index", args.file);
    } else {
        println!("File {} not found", args.file);
    }
}

fn commit_workflow(args: CommitArgs) {
    // TBD
}

fn cat_file_workflow(args: FileArgs) {
    let hash = args.hash;
    println!("Unhashing SHA: {}", hash);
    let object_path = format!(".fit/objects/{}", hash);
    if Path::new(&object_path).exists() {
        let content = fs::read_to_string(object_path).unwrap();
        println!("{}", content);
    } else {
        println!("Object not found");
    }
}

