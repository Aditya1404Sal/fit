use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use sha1::{Sha1, Digest};
use clap::{Args, Parser, Subcommand};
use flate2::write::ZlibEncoder;
use flate2::read::ZlibDecoder;
use flate2::Compression;

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
    url: String,
}

#[derive(Args)]
struct FileArgs {
    hash: String,
}

#[derive(Args)]
struct AddArgs {
    path: String,
}

#[derive(Args)]
struct RmArgs {
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
    println!("Clone functionality not yet implemented {}", args.url);
}

fn log_workflow() {
    // TBD
    println!("Log functionality not yet implemented");
}

fn write_object(content: &[u8], object_type: &str) -> String {
    let mut hasher = Sha1::new();
    let header = format!("{} {}\0", object_type, content.len());
    hasher.update(&header);
    hasher.update(content);
    let hash = hasher.finalize();
    let hash_hex = format!("{:x}", hash);
    let object_path = format!(".fit/objects/{}", hash_hex);
    let file = File::create(object_path).unwrap();
    let mut encoder = ZlibEncoder::new(file, Compression::default());
    encoder.write_all(header.as_bytes()).unwrap();
    encoder.write_all(content).unwrap();
    encoder.finish().unwrap();

    hash_hex
}

fn read_object(hash: &str) -> Option<(String, Vec<u8>)> {
    let object_path = format!(".fit/objects/{}", hash);
    if !Path::new(&object_path).exists() {
        return None;
    }

    let file = File::open(object_path).unwrap();
    let mut decoder = ZlibDecoder::new(file);
    let mut content = Vec::new();
    // decompression takes place here
    decoder.read_to_end(&mut content).unwrap();
    // null position to seperate the header from the content
    let null_pos = content.iter().position(|&b| b == 0).unwrap();
    let header = String::from_utf8_lossy(&content[..null_pos]).to_string();
    // content starts just after the null position 
    let object_content = content[null_pos + 1..].to_vec();
    // object type like blob or tree
    let mut parts = header.splitn(2, ' ');
    let object_type = parts.next().unwrap().to_string();

    Some((object_type, object_content))
}
// Simple management of adding anything
fn add_workflow(args: AddArgs) {
    let path = Path::new(&args.path);
    if path.is_file() {
        add_file(path);
    } else if path.is_dir() {
        add_directory(path);
    } else {
        println!("'{}' is not a valid file or directory", args.path);
    }
}

fn remove_object(hash: &str) {
    let object_path = format!(".fit/objects/{}", hash);
    if Path::new(&object_path).exists() {
        fs::remove_file(&object_path).unwrap();
        println!("Removed old object {} from fit", hash);
    }
}

fn add_file(path: &Path) {
    let mut file = File::open(path).unwrap();
    let mut contents = Vec::new();
    file.read_to_end(&mut contents).unwrap();

    let mut index = read_index();
    let file_path = path.to_str().unwrap().to_string();

    // Remove the old object if it exists
    if let Some(old_hash) = index.get(&file_path) {
        remove_object(old_hash);
    }

    let hash_hex = write_object(&contents, "blob");

    index.insert(file_path, hash_hex);

    write_index(&index);

    println!("Added {} to fit", path.display());
}

fn add_directory(path: &Path) {
    for entry in fs::read_dir(path).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_file() {
            add_file(&path);
        } else if path.is_dir() {
            add_directory(&path);
        }
    }
}

fn read_index() -> HashMap<String, String> {
    let index_path = ".fit/index";
    let index_content = fs::read_to_string(index_path).unwrap_or_default();
    index_content
        .lines()
        .map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            (parts[1].to_string(), parts[0].to_string())
        })
        .collect()
}

fn write_index(index: &HashMap<String, String>) {
    let index_path = ".fit/index";
    let content: String = index
        .iter()
        .map(|(path, hash)| format!("{} {}", hash, path))
        .collect::<Vec<String>>()
        .join("\n");
    fs::write(index_path, content).unwrap();
}

fn rm_workflow(args: RmArgs) {
    let path = Path::new(&args.file);
    if path.exists() {
        let mut index = read_index();
        if let Some(hash_hex) = index.remove(path.to_str().unwrap()) {
            remove_object(&hash_hex);
            write_index(&index);
            println!("Removed {} from fit index", args.file);
        } else {
            println!("File {} not found in fit index", args.file);
        }
    } else {
        println!("File {} not found", args.file);
    }
}

fn commit_workflow(args: CommitArgs) {
    // TBD
    println!("Commit functionality not yet implemented : {}", args.message);
}
// Creates list of entries inside INDEX tracking all the objects as a tree.
fn create_tree_object(index: &HashMap<String, String>) -> String {
    let mut tree_content = String::new();
    for (path, hash) in index {
        tree_content.push_str(&format!("100644 blob {} {}\n", hash, path));
    }
    write_object(tree_content.as_bytes(), "tree")
}
// Returns current commit
fn get_current_commit() -> String {
    let head_content = fs::read_to_string(".fit/HEAD").unwrap();
    let ref_path = head_content.trim().strip_prefix("ref: ").unwrap();
    fs::read_to_string(ref_path).unwrap_or_default().trim().to_string()
}
// Changes HEAD pointer to newest commit
fn update_current_branch(commit_hash: &str) {
    let head_content = fs::read_to_string(".fit/HEAD").unwrap();
    let ref_path = head_content.trim().strip_prefix("ref: ").unwrap();
    fs::write(ref_path, commit_hash).unwrap();
}

fn cat_file_workflow(args: FileArgs) {
    let hash = args.hash;
    println!("Unhashing SHA: {}", hash);
    match read_object(&hash) {
        Some((object_type, content)) => {
            println!("Object type: {}", object_type);
            println!("Content:");
            println!("{}", String::from_utf8_lossy(&content));
        },
        None => println!("Object not found"),
    }
}

