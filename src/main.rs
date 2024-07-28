use clap::{Args, Parser, Subcommand};
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use sha1::{Digest, Sha1};
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::Path;

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
    Status,
    Reset(ResetArgs),
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
struct ResetArgs {
    commit_hash: String,
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

#[derive(Default)]
struct StagingArea {
    added: HashMap<String, String>,
    modified: HashMap<String, String>,
    deleted: Vec<String>,
}

impl StagingArea {
    fn new() -> Self {
        StagingArea::default()
    }

    fn add(&mut self, path: String, hash: String) {
        self.added.insert(path, hash);
    }

    fn modify(&mut self, path: String, hash: String) {
        self.modified.insert(path, hash);
    }

    fn delete(&mut self, path: String) {
        self.deleted.push(path);
    }

    fn is_staged(&self, path: &String) -> bool {
        self.added.contains_key(path)
            || self.modified.contains_key(path)
            || self.deleted.contains(path)
    }
}

fn main() -> io::Result<()> {
    let args = Fit::parse();
    match args.command {
        FitCommands::Init => init_workflow()?,
        FitCommands::Clone(clone_args) => clone_workflow(clone_args)?,
        FitCommands::Log => log_workflow()?,
        FitCommands::Add(add_args) => add_workflow(add_args)?,
        FitCommands::Rm(rm_args) => rm_workflow(rm_args)?,
        FitCommands::Commit(commit_args) => commit_workflow(commit_args)?,
        FitCommands::Catfile(file_args) => cat_file_workflow(file_args)?,
        FitCommands::Status => status_workflow()?,
        FitCommands::Reset(reset_args) => reset_workflow(&reset_args.commit_hash)?,
    }
    Ok(())
}

fn init_workflow() -> io::Result<()> {
    println!("Initializing fit repository...");

    fs::create_dir(".fit")?;
    fs::create_dir(".fit/objects")?;
    fs::create_dir_all(".fit/refs/heads")?;
    fs::write(".fit/HEAD", "ref: refs/heads/master\n")?;
    File::create(".fit/index")?;

    let empty_tree_hash = create_empty_tree()?;
    let initial_commit_hash = create_initial_commit(empty_tree_hash)?;

    fs::write(".fit/refs/heads/master", initial_commit_hash)?;

    println!("Initialized fit repository successfully");
    Ok(())
}

fn create_empty_tree() -> io::Result<String> {
    write_object("".as_bytes(), "tree")
}

fn create_initial_commit(tree_hash: String) -> io::Result<String> {
    let commit_content = format!("tree {}\n\nInitial commit", tree_hash);
    write_object(commit_content.as_bytes(), "commit")
}

fn clone_workflow(_args: CloneArgs) -> io::Result<()> {
    println!("Clone functionality not yet implemented");
    Ok(())
}

fn log_workflow() -> io::Result<()> {
    let mut current_commit = get_current_commit()?;
    while !current_commit.is_empty() {
        if let Some((_, content)) = read_object(&current_commit)? {
            let commit_content = String::from_utf8_lossy(&content);
            let (commit_info, message) = commit_content.split_once("\n\n").unwrap();
            println!("commit {}", current_commit);
            println!("{}", commit_info);
            println!("\n    {}\n", message.trim());
            current_commit = get_parent_commit(&commit_info);
        } else {
            break;
        }
    }
    Ok(())
}

fn write_object(content: &[u8], object_type: &str) -> io::Result<String> {
    let mut hasher = Sha1::new();
    let header = format!("{} {}\0", object_type, content.len());
    hasher.update(&header);
    hasher.update(content);
    let hash = hasher.finalize();
    let hash_hex = format!("{:x}", hash);

    let dir_name = &hash_hex[0..2];
    let file_name = &hash_hex[2..];
    let object_dir = Path::new(".fit").join("objects").join(dir_name);
    fs::create_dir_all(&object_dir)?;

    let object_path = object_dir.join(file_name);
    let file = File::create(object_path)?;
    let mut encoder = ZlibEncoder::new(file, Compression::default());
    encoder.write_all(header.as_bytes())?;
    encoder.write_all(content)?;
    encoder.finish()?;

    Ok(hash_hex)
}

fn read_object(hash: &str) -> io::Result<Option<(String, Vec<u8>)>> {
    let dir_name = &hash[0..2];
    let file_name = &hash[2..];
    let object_path = Path::new(".fit")
        .join("objects")
        .join(dir_name)
        .join(file_name);

    if !object_path.exists() {
        return Ok(None);
    }

    let file = File::open(object_path)?;
    let mut decoder = ZlibDecoder::new(file);
    let mut content = Vec::new();
    decoder.read_to_end(&mut content)?;
    let null_pos = content.iter().position(|&b| b == 0).unwrap();
    let header = String::from_utf8_lossy(&content[..null_pos]).to_string();
    let object_content = content[null_pos + 1..].to_vec();
    let mut parts = header.splitn(2, ' ');
    let object_type = parts.next().unwrap().to_string();

    Ok(Some((object_type, object_content)))
}

fn add_workflow(args: AddArgs) -> io::Result<()> {
    let path = Path::new(&args.path);
    let mut staging_area = read_staging_area()?;
    let mut index = read_index()?;

    if path.is_file() {
        add_file(path, &mut staging_area, &mut index)?;
    } else if path.is_dir() {
        add_directory(path, &mut staging_area, &mut index)?;
    } else {
        println!("'{}' is not a valid file or directory", args.path);
    }

    write_staging_area(&staging_area)?;
    write_index(&index)?;
    Ok(())
}

fn add_file(
    path: &Path,
    staging_area: &mut StagingArea,
    index: &mut HashMap<String, String>,
) -> io::Result<()> {
    let mut file = File::open(path)?;
    let mut contents = Vec::new();
    file.read_to_end(&mut contents)?;

    let file_path = path.to_str().unwrap().to_string();
    let hash_hex = write_object(&contents, "blob")?;

    if let Some(old_hash) = index.get(&file_path) {
        if old_hash != &hash_hex {
            staging_area.modify(file_path.clone(), hash_hex.clone());
        }
    } else {
        staging_area.add(file_path.clone(), hash_hex.clone());
    }

    index.insert(file_path, hash_hex);

    println!("Added {} to staging area", path.display());
    Ok(())
}

fn add_directory(
    path: &Path,
    staging_area: &mut StagingArea,
    index: &mut HashMap<String, String>,
) -> io::Result<()> {
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            add_file(&path, staging_area, index)?;
        } else if path.is_dir() {
            add_directory(&path, staging_area, index)?;
        }
    }
    Ok(())
}

fn read_staging_area() -> io::Result<StagingArea> {
    let staging_path = ".fit/STAGING";
    if !Path::new(staging_path).exists() {
        return Ok(StagingArea::new());
    }

    let staging_content = fs::read_to_string(staging_path)?;
    let mut staging_area = StagingArea::new();

    for line in staging_content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() == 3 {
            match parts[0] {
                "A" => staging_area.add(parts[2].to_string(), parts[1].to_string()),
                "M" => staging_area.modify(parts[2].to_string(), parts[1].to_string()),
                "D" => staging_area.delete(parts[2].to_string()),
                _ => {}
            }
        }
    }

    Ok(staging_area)
}

fn write_staging_area(staging_area: &StagingArea) -> io::Result<()> {
    let staging_path = ".fit/STAGING";
    let mut content = String::new();

    for (path, hash) in &staging_area.added {
        content.push_str(&format!("A {} {}\n", hash, path));
    }
    for (path, hash) in &staging_area.modified {
        content.push_str(&format!("M {} {}\n", hash, path));
    }
    for path in &staging_area.deleted {
        content.push_str(&format!("D {}\n", path));
    }

    fs::write(staging_path, content)
}

fn read_index() -> io::Result<HashMap<String, String>> {
    let index_path = ".fit/index";
    let index_content = fs::read_to_string(index_path)?;
    Ok(index_content
        .lines()
        .map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            (parts[1].to_string(), parts[0].to_string())
        })
        .collect())
}

fn write_index(index: &HashMap<String, String>) -> io::Result<()> {
    let index_path = ".fit/index";
    let content: String = index
        .iter()
        .map(|(path, hash)| format!("{} {}", hash, path))
        .collect::<Vec<String>>()
        .join("\n");
    fs::write(index_path, content)
}

fn rm_workflow(args: RmArgs) -> io::Result<()> {
    let path = Path::new(&args.file);
    if path.exists() {
        let mut staging_area = read_staging_area()?;
        let mut index = read_index()?;

        if index.remove(path.to_str().unwrap()).is_some() {
            staging_area.delete(path.to_str().unwrap().to_string());
            write_staging_area(&staging_area)?;
            write_index(&index)?;
            println!("Removed {} from fit index and staging area", args.file);
        } else {
            println!("File {} not found in fit index", args.file);
        }
    } else {
        println!("File {} not found", args.file);
    }
    Ok(())
}

fn commit_workflow(args: CommitArgs) -> io::Result<()> {
    println!("Commiting...");

    let staging_area = read_staging_area()?;
    if staging_area.added.is_empty()
        && staging_area.modified.is_empty()
        && staging_area.deleted.is_empty()
    {
        println!("Nothing to commit. Working tree clean.");
        return Ok(());
    }

    let mut index = read_index()?;

    // Apply changes from staging area to index
    for (path, hash) in staging_area
        .added
        .iter()
        .chain(staging_area.modified.iter())
    {
        index.insert(path.clone(), hash.clone());
    }
    for path in &staging_area.deleted {
        index.remove(path);
    }

    let tree_hash = create_tree_object(&index)?;
    println!("Tree object created with hash: {}", tree_hash);

    let parent_hash = get_current_commit()?;
    println!("Current commit (parent) hash: {}", parent_hash);

    let commit_content = format!(
        "tree {}\nparent {}\n\n{}",
        tree_hash, parent_hash, args.message
    );
    println!("Commit content created.");

    let commit_hash = write_object(commit_content.as_bytes(), "commit")?;
    println!("Commit object written with hash: {}", commit_hash);

    update_current_branch(&commit_hash)?;
    println!("Current branch updated.");

    // Clear staging area
    fs::remove_file(".fit/STAGING")?;

    write_index(&index)?;

    println!("Created commit {}", commit_hash);
    Ok(())
}

fn create_tree_object(index: &HashMap<String, String>) -> io::Result<String> {
    let mut tree_content = String::new();
    for (path, hash) in index {
        tree_content.push_str(&format!("100644 blob {} {}\n", hash, path));
    }
    write_object(tree_content.as_bytes(), "tree")
}

fn get_current_commit() -> io::Result<String> {
    let head_content = fs::read_to_string(".fit/HEAD")?;
    let ref_path = head_content
        .trim()
        .strip_prefix("ref: ")
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid HEAD content"))?;
    let full_ref_path = Path::new(".fit").join(ref_path);
    let commit_hash = fs::read_to_string(full_ref_path)?.trim().to_string();
    Ok(commit_hash)
}

fn update_current_branch(commit_hash: &str) -> io::Result<()> {
    let head_content = fs::read_to_string(".fit/HEAD")?;
    let ref_path = head_content
        .trim()
        .strip_prefix("ref: ")
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid HEAD content"))?;
    let full_ref_path = format!(".fit/{}", ref_path);
    fs::write(full_ref_path, commit_hash)
}

fn get_parent_commit(commit_info: &str) -> String {
    commit_info
        .lines()
        .find(|line| line.starts_with("parent "))
        .map(|line| line.strip_prefix("parent ").unwrap().to_string())
        .unwrap_or_default()
}

fn cat_file_workflow(args: FileArgs) -> io::Result<()> {
    let hash = args.hash;
    println!("Unhashing SHA: {}", hash);
    match read_object(&hash)? {
        Some((object_type, content)) => {
            println!("Object type: {}", object_type);
            println!("Content:");
            println!("{}", String::from_utf8_lossy(&content));
        }
        None => println!("Object not found"),
    }
    Ok(())
}

fn status_workflow() -> io::Result<()> {
    let staging_area = read_staging_area()?;
    let index = read_index()?;

    println!("Changes to be committed:");
    for (path, _) in &staging_area.added {
        println!("  new file: {}", path);
    }
    for (path, _) in &staging_area.modified {
        println!("  modified: {}", path);
    }
    for path in &staging_area.deleted {
        println!("  deleted: {}", path);
    }

    println!("\nChanges not staged for commit:");
    for (path, hash) in &index {
        if !staging_area.is_staged(path) {
            if let Ok(file_content) = fs::read(path) {
                let file_hash = write_object(&file_content, "blob")?;
                if &file_hash != hash {
                    println!("  modified: {}", path);
                }
            } else {
                println!("  deleted: {}", path);
            }
        }
    }

    println!("\nUntracked files:");
    for entry in fs::read_dir(".")? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file()
            && !path.starts_with(".fit")
            && !index.contains_key(path.to_str().unwrap())
        {
            println!("  {}", path.display());
        }
    }

    Ok(())
}

fn reset_workflow(commit_hash: &str) -> io::Result<()> {
    if read_object(&commit_hash)?.is_none() {
        return Err(io::Error::new(io::ErrorKind::NotFound, "Commit not found"));
    }
    update_current_branch(&commit_hash)?;

    let (_, commit_content) = read_object(&commit_hash)?.unwrap();
    let commit_content = String::from_utf8_lossy(&commit_content);
    let tree_hash = commit_content
        .lines()
        .next()
        .unwrap()
        .split_whitespace()
        .nth(1)
        .unwrap();

    let (_, tree_content) = read_object(tree_hash)?.unwrap();
    let tree_content: Cow<str> = String::from_utf8_lossy(&tree_content);

    let mut new_index = HashMap::new();
    if Path::new(".fit/STAGING").exists() {
        fs::remove_file(".fit/STAGING")?;
    }

    let current_index = read_index()?;
    let current_files: HashSet<_> = current_index.keys().cloned().collect();

    let mut target_files = HashSet::new();

    for line in tree_content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        let file_hash = parts[2];
        let file_path = parts[3];

        target_files.insert(file_path.to_string());

        let (_, blob_content) = read_object(file_hash)?.unwrap();

        if let Some(parent) = Path::new(file_path).parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(file_path, blob_content)?;

        new_index.insert(file_path.to_string(), file_hash.to_string());
    }

    for file in current_files.difference(&target_files) {
        if Path::new(file).exists() {
            fs::remove_file(file)?;
            println!("Removed file: {}", file);
        }
    }

    write_index(&new_index)?;

    println!("Reset to commit {}", commit_hash);
    Ok(())
}
