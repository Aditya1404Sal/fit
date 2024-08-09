use clap::{Args, Parser, Subcommand};
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use sha1::{Digest, Sha1};
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::fs::{self};
use std::io::{self, Error, Read, Write};
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
    Branch(BranchArgs),
    Diff(DiffArgs),
    Merge(MergeArgs),
    Stash(StashArgs),
}

#[derive(Args)]
struct StashArgs {
    #[clap(subcommand)]
    command: Option<StashSubCommand>,
}

#[derive(Subcommand)]
enum StashSubCommand {
    Pop,
}

#[derive(Args)]
struct MergeArgs {
    branch: String,
}
#[derive(Args)]
struct DiffArgs {
    #[clap(subcommand)]
    command: Option<DiffSubcommand>,
}

#[derive(Subcommand)]
enum DiffSubcommand {
    Commit { commit1: String, commit2: String },
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
#[derive(Args)]
struct BranchArgs {
    #[clap(subcommand)]
    command: BranchSubcommand,
}
#[derive(Subcommand)]
enum BranchSubcommand {
    List,
    Create { name: String },
    Delete { name: String },
    Checkout { name: String },
    CheckoutNew { name: String },
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
        FitCommands::Branch(branch_args) => branch_workflow(branch_args)?,
        FitCommands::Diff(diff_args) => diff_workflow(diff_args)?,
        FitCommands::Merge(merge_args) => merge_workflow(merge_args)?,
        FitCommands::Stash(stash_args) => stash_workflow(stash_args)?,
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
        .unwrap_or(&head_content);
    let full_ref_path = Path::new(".fit").join(ref_path);
    Ok(fs::read_to_string(full_ref_path)?.trim().to_string())
}

fn update_current_branch(commit_hash: &str) -> io::Result<()> {
    let current_branch = get_current_branch()?;
    let branch_path = Path::new(".fit/refs/heads").join(current_branch);
    fs::write(branch_path, commit_hash)
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
    let current_branch = get_current_branch()?;
    println!("On branch: {}", current_branch);
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

fn get_current_branch() -> io::Result<String> {
    let head_content = fs::read_to_string(".fit/HEAD")?;
    Ok(head_content
        .trim()
        .strip_prefix("ref: refs/heads/")
        .unwrap_or("master")
        .to_string())
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

fn branch_workflow(args: BranchArgs) -> io::Result<()> {
    match args.command {
        BranchSubcommand::List => list_branches()?,
        BranchSubcommand::Create { name } => create_branch(&name)?,
        BranchSubcommand::Delete { name } => delete_branch(&name)?,
        BranchSubcommand::Checkout { name } => checkout_branch(&name)?,
        BranchSubcommand::CheckoutNew { name } => checkout_new_branch(&name)?,
    }
    Ok(())
}

fn list_branches() -> io::Result<()> {
    let branches_dir = Path::new(".fit/refs/heads");
    for entry in fs::read_dir(branches_dir)? {
        let entry = entry?;
        println!("{}", entry.file_name().to_string_lossy());
    }
    Ok(())
}

fn create_branch(name: &str) -> io::Result<()> {
    if name == "master" {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "Cannot create a duplicate master branch",
        ));
    }
    let current_commit = get_current_commit()?;
    let branch_path = Path::new(".fit/refs/heads").join(name);
    if branch_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!(
                "Branch called '{}' already exists, choose a different name",
                name
            ),
        ));
    }
    fs::write(branch_path, current_commit)?;
    println!("Created branch '{}'", name);
    Ok(())
}

fn delete_branch(name: &str) -> io::Result<()> {
    if name == "master" {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "Cannot delete the master branch",
        ));
    }
    let current_branch = get_current_branch()?;
    if current_branch == *name {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "Cannot delete branch currently in use, please switch to master or different branch",
        ));
    }
    let branch_path = Path::new(".fit/refs/heads").join(name);
    if !branch_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Branch not found, Cannot delete non-existent branch",
        ));
    }
    fs::remove_file(branch_path)?;
    println!("Deleted branch '{}'", name);
    Ok(())
}

fn checkout_branch(name: &str) -> io::Result<()> {
    let branch_path = Path::new(".fit/refs/heads").join(name);
    if !branch_path.exists() {
        return Err(io::Error::new(io::ErrorKind::NotFound, "Branch not found"));
    }
    let commit_hash = fs::read_to_string(branch_path)?;
    fs::write(".fit/HEAD", format!("ref: refs/heads/{}\n", name))?;
    reset_workflow(&commit_hash)?;
    println!("Switched to branch '{}'", name);
    Ok(())
}

fn checkout_new_branch(name: &str) -> io::Result<()> {
    create_branch(name)?;
    checkout_branch(name)?;
    Ok(())
}
fn diff_workflow(args: DiffArgs) -> io::Result<()> {
    match args.command {
        Some(DiffSubcommand::Commit { commit1, commit2 }) => {
            diff_commits(&commit1, &commit2)?;
        }
        None => {
            diff_staged_vs_latest()?;
        }
    }
    Ok(())
}

fn diff_commits(commit1: &str, commit2: &str) -> io::Result<()> {
    println!("Diffing commit {} and {}", commit1, commit2);

    // Get tree hashes for both commits
    let tree1 = get_commit_tree(commit1)?;
    let tree2 = get_commit_tree(commit2)?;

    // Get file lists for both trees
    let files1 = get_tree_files(&tree1)?;
    let files2 = get_tree_files(&tree2)?;

    // Compare files in both trees
    let all_files: HashSet<_> = files1.keys().chain(files2.keys()).collect();

    for file in all_files {
        match (files1.get(file), files2.get(file)) {
            (Some(hash1), Some(hash2)) if hash1 != hash2 => {
                // File exists in both commits but has changed
                let (_, content1) = read_object(hash1)?.unwrap();
                let (_, content2) = read_object(hash2)?.unwrap();
                print_diff(
                    file,
                    &String::from_utf8_lossy(&content1),
                    &String::from_utf8_lossy(&content2),
                );
            }
            (Some(hash), None) => {
                // File exists in commit1 but not in commit2 (deleted)
                let (_, content) = read_object(hash)?.unwrap();
                print_diff(file, &String::from_utf8_lossy(&content), "");
            }
            (None, Some(hash)) => {
                // File exists in commit2 but not in commit1 (new file)
                let (_, content) = read_object(hash)?.unwrap();
                print_diff(file, "", &String::from_utf8_lossy(&content));
            }
            _ => {} // File exists in both commits and hasn't changed, or doesn't exist in either
        }
    }

    Ok(())
}

fn get_commit_tree(commit_hash: &str) -> io::Result<String> {
    let (_, commit_content) = read_object(commit_hash)?.unwrap();
    let commit_content = String::from_utf8_lossy(&commit_content);
    Ok(commit_content
        .lines()
        .next()
        .unwrap()
        .split_whitespace()
        .nth(1)
        .unwrap()
        .to_string())
}

fn get_tree_files(tree_hash: &str) -> io::Result<HashMap<String, String>> {
    let (_, tree_content) = read_object(tree_hash)?.unwrap();
    let tree_content = String::from_utf8_lossy(&tree_content);

    let mut files = HashMap::new();
    for line in tree_content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        let file_hash = parts[2];
        let file_path = parts[3];
        files.insert(file_path.to_string(), file_hash.to_string());
    }

    Ok(files)
}

fn diff_staged_vs_latest() -> io::Result<()> {
    let index = read_index()?;
    let current_commit = get_current_commit()?;

    // Get the tree hash from the current commit
    let (_, commit_content) = read_object(&current_commit)?.unwrap();
    let commit_content = String::from_utf8_lossy(&commit_content);
    let tree_hash = commit_content
        .lines()
        .next()
        .unwrap()
        .split_whitespace()
        .nth(1)
        .unwrap();

    // Read the tree object
    let (_, tree_content) = read_object(tree_hash)?.unwrap();
    let tree_content = String::from_utf8_lossy(&tree_content);

    // Parse the tree content to get file hashes
    let mut commit_files = HashMap::new();
    for line in tree_content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        let file_hash = parts[2];
        let file_path = parts[3];
        commit_files.insert(file_path.to_string(), file_hash.to_string());
    }

    // Compare staged files with commit files
    for (file_path, staged_hash) in &index {
        if let Some(commit_hash) = commit_files.get(file_path) {
            if staged_hash != commit_hash {
                let (_, staged_content) = read_object(staged_hash)?.unwrap();
                let (_, commit_content) = read_object(commit_hash)?.unwrap();
                print_diff(
                    file_path,
                    &String::from_utf8_lossy(&commit_content),
                    &String::from_utf8_lossy(&staged_content),
                );
            }
        } else {
            // New file in staging
            let (_, staged_content) = read_object(staged_hash)?.unwrap();
            print_diff(file_path, "", &String::from_utf8_lossy(&staged_content));
        }
    }

    // Check for deleted files
    for (file_path, commit_hash) in &commit_files {
        if !index.contains_key(file_path) {
            let (_, commit_content) = read_object(commit_hash)?.unwrap();
            print_diff(file_path, &String::from_utf8_lossy(&commit_content), "");
        }
    }

    Ok(())
}

fn print_diff(file_path: &str, old_content: &str, new_content: &str) {
    println!("Diff for file: {}", file_path);

    let diff = diff::lines(old_content, new_content);

    for change in diff {
        match change {
            diff::Result::Left(l) => println!("-{}", l),
            diff::Result::Both(l, _) => println!(" {}", l),
            diff::Result::Right(r) => println!("+{}", r),
        }
    }

    println!();
}

fn merge_workflow(args: MergeArgs) -> io::Result<()> {
    let current_branch = get_current_branch()?;
    if current_branch == args.branch {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "cannot merge a branch into itself",
        ));
    }
    if args.branch == "master" || current_branch != "master" {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "cannot merge master into Non-Head branch",
        ));
    }
    println!("Merging {} into master...", args.branch);
    let current_commit = get_current_commit()?;
    let branch_commit = get_branch_commit(&args.branch)?;

    if current_commit == branch_commit {
        println!("Already up to date. Nothing to merge.");
        return Ok(());
    }

    let merge_base = find_merge_base(&current_commit, &branch_commit)?;

    if merge_base == branch_commit {
        println!("Fast-forward merge possible.");
        fast_forward_merge(&branch_commit)?;
    } else {
        println!("Performing three-way merge.");
        // three_way_merge(&current_commit, &branch_commit, &merge_base)?;
    }

    Ok(())
}

fn get_branch_commit(branch_name: &str) -> io::Result<String> {
    let branch_path = Path::new(".fit/refs/heads").join(branch_name);
    if !branch_path.exists() {
        return Err(io::Error::new(io::ErrorKind::NotFound, "Branch not found"));
    }
    Ok(fs::read_to_string(branch_path)?.trim().to_string())
}

fn find_merge_base(current_commit: &str, branch_commit: &str) -> io::Result<String> {
    let commit_history_1 = get_commit_history(current_commit)?;
    let commit_history_2 = get_commit_history(branch_commit)?;

    for commit in commit_history_2 {
        if commit_history_1.contains(&commit) {
            return Ok(commit);
        }
    }
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "Merge Base not found",
    ))
}

fn get_commit_history(commit: &str) -> io::Result<Vec<String>> {
    let mut history = Vec::new();
    let mut current = commit.to_string();

    while !current.is_empty() {
        history.push(current.clone());
        current = get_parent_commit(&read_object(&current)?.unwrap().0);
    }

    Ok(history)
}

fn fast_forward_merge(branch_commit: &str) -> io::Result<()> {
    update_current_branch(branch_commit)?;
    reset_workflow(branch_commit)?;
    println!("Fast-forward merge completed.");
    Ok(())
}

fn stash_workflow(args: StashArgs) -> io::Result<()> {
    match args.command {
        Some(StashSubCommand::Pop) => {
            pop_stashed_content()?;
        }
        None => {
            stash_content()?;
        }
    }
    Ok(())
}
// Create a STASH file that stores content present in staged area, or unsaved content after which a commit hash is created
// Which represents the contents of the pwd at that given instance, then a reset is made to the previous commit leaving the STASH hash saved
// then when stash pop is called, this STASH hash is reset, if consecutive Stashes are made then it creates a stack
// following LIFO principle, most recent stash will be restored
fn stash_content() -> io::Result<()> {
    let index = read_index()?;
    let tree_hash = create_tree_object(&index)?;
    let parent_hash = get_current_commit()?;
    let commit_content = format!("tree {}\nparent {}\n\n{}", tree_hash, parent_hash, "stash");

    let stash_hash = write_object(commit_content.as_bytes(), "commit")?;
    write_stashing_area(&stash_hash)?;
    reset_workflow(&parent_hash)?;
    Ok(())
}

fn read_stashing_area() -> io::Result<Option<String>> {
    let st_path = ".fit/STASH";
    if !Path::new(st_path).exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(st_path)?;

    let mut lines: Vec<&str> = content.lines().collect();

    if lines.is_empty() {
        return Ok(None);
    }

    let topmost_stash = lines.remove(0).to_string();

    let updated_content = lines.join("\n");
    fs::write(st_path, updated_content)?;

    Ok(Some(topmost_stash))
}

fn write_stashing_area(stash_hash: &str) -> io::Result<()> {
    let st_path = ".fit/STASH";

    let existing_content = fs::read_to_string(st_path).unwrap_or_default();

    let updated_content = format!("{}\n{}", stash_hash, existing_content.trim());

    fs::write(st_path, updated_content)
}

fn pop_stashed_content() -> io::Result<()> {
    match read_stashing_area()? {
        Some(latest_hash) => {
            reset_workflow(&latest_hash)?;
            Ok(())
        }
        None => Err(Error::new(
            io::ErrorKind::NotFound,
            "cannot pop, stash something first",
        )),
    }
}
