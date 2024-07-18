use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::{self, Read, Write};
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
    }
    Ok(())
}

fn init_workflow() -> io::Result<()> {
    println!("Initializing fit repository...");

    // Create .fit directory
    fs::create_dir(".fit")?;
    println!("Created .fit directory");

    // Create objects directory
    fs::create_dir(".fit/objects")?;
    println!("Created .fit/objects directory");

    // Create refs directory and its subdirectories
    fs::create_dir_all(".fit/refs/heads")?;
    println!("Created .fit/refs/heads directory");

    // Create HEAD file
    fs::write(".fit/HEAD", "ref: refs/heads/master\n")?;
    println!("Created .fit/HEAD file");

    // Create an empty index file
    File::create(".fit/index")?;
    println!("Created empty .fit/index file");

    // Create initial commit
    let empty_tree_hash = create_empty_tree()?;
    let initial_commit_hash = create_initial_commit(empty_tree_hash)?;

    // Update master branch to point to the initial commit
    fs::write(".fit/refs/heads/master", initial_commit_hash)?;
    println!("Created initial commit and updated master branch");

    println!("Initialized fit repository successfully");
    Ok(())
}

fn create_empty_tree() -> io::Result<String> {
    // An empty tree in Git is represented by an empty string
    write_object("".as_bytes(), "tree")
}

fn create_initial_commit(tree_hash: String) -> io::Result<String> {
    let commit_content = format!(
        "tree {}\n\nInitial commit",
        tree_hash
    );
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
    let object_path = format!(".fit/objects/{}", hash_hex);
    let file = File::create(object_path)?;
    let mut encoder = ZlibEncoder::new(file, Compression::default());
    encoder.write_all(header.as_bytes())?;
    encoder.write_all(content)?;
    encoder.finish()?;

    Ok(hash_hex)
}

fn read_object(hash: &str) -> io::Result<Option<(String, Vec<u8>)>> {
    let object_path = format!(".fit/objects/{}", hash);
    if !Path::new(&object_path).exists() {
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
    if path.is_file() {
        add_file(path)?;
    } else if path.is_dir() {
        add_directory(path)?;
    } else {
        println!("'{}' is not a valid file or directory", args.path);
    }
    Ok(())
}

fn remove_object(hash: &str) -> io::Result<()> {
    let object_path = format!(".fit/objects/{}", hash);
    if Path::new(&object_path).exists() {
        fs::remove_file(&object_path)?;
        println!("Removed old object {} from fit", hash);
    }
    Ok(())
}

fn add_file(path: &Path) -> io::Result<()> {
    let mut file = File::open(path)?;
    let mut contents = Vec::new();
    file.read_to_end(&mut contents)?;

    let mut index = read_index()?;
    let file_path = path.to_str().unwrap().to_string();

    if let Some(old_hash) = index.get(&file_path) {
        remove_object(old_hash)?;
    }

    let hash_hex = write_object(&contents, "blob")?;

    index.insert(file_path, hash_hex);

    write_index(&index)?;

    println!("Added {} to fit", path.display());
    Ok(())
}

fn add_directory(path: &Path) -> io::Result<()> {
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            add_file(&path)?;
        } else if path.is_dir() {
            add_directory(&path)?;
        }
    }
    Ok(())
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
        let mut index = read_index()?;
        if let Some(hash_hex) = index.remove(path.to_str().unwrap()) {
            remove_object(&hash_hex)?;
            write_index(&index)?;
            println!("Removed {} from fit index", args.file);
        } else {
            println!("File {} not found in fit index", args.file);
        }
    } else {
        println!("File {} not found", args.file);
    }
    Ok(())
}

fn commit_workflow(args: CommitArgs) -> io::Result<()> {
    println!("Starting commit workflow...");

    let index = read_index()?;
    println!("Index read successfully. {} entries found.", index.len());

    let tree_hash = create_tree_object(&index)?;
    println!("Tree object created with hash: {}", tree_hash);

    let parent_hash = match get_current_commit() {
        Ok(hash) => hash,
        Err(e) => {
            if e.kind() == io::ErrorKind::NotFound {
                println!("No previous commit found. This will be the initial commit.");
                String::new()
            } else {
                return Err(e);
            }
        }
    };
    println!("Current commit (parent) hash: {}", parent_hash);

    let commit_content = if parent_hash.is_empty() {
        format!(
            "tree {}\n\n{}",
            tree_hash,
            args.message
        )
    } else {
        format!(
            "tree {}\nparent {}\n\n{}",
            tree_hash,
            parent_hash,
            args.message
        )
    };
    println!("Commit content created.");

    let commit_hash = write_object(commit_content.as_bytes(), "commit")?;
    println!("Commit object written with hash: {}", commit_hash);

    update_current_branch(&commit_hash)?;
    println!("Current branch updated.");

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
    println!("Entering get_current_commit()");

    let head_path = Path::new(".fit/HEAD");
    if !head_path.exists() {
        return Err(io::Error::new(io::ErrorKind::NotFound, "HEAD file not found. Have you initialized the repository?"));
    }

    let head_content = fs::read_to_string(head_path)?;
    println!("HEAD content: {}", head_content);

    let ref_path = head_content.trim().strip_prefix("ref: ").ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidData, "Invalid HEAD content")
    })?;
    println!("Reference path: {}", ref_path);

    let full_ref_path = Path::new(".fit").join(ref_path);
    if !full_ref_path.exists() {
        return Err(io::Error::new(io::ErrorKind::NotFound, format!("Reference file not found: {}", full_ref_path.display())));
    }

    let commit_hash = fs::read_to_string(full_ref_path)?.trim().to_string();
    println!("Current commit hash: {}", commit_hash);

    Ok(commit_hash)
}

fn update_current_branch(commit_hash: &str) -> io::Result<()> {
    let head_content = fs::read_to_string(".fit/HEAD")?;
    let ref_path = head_content.trim().strip_prefix("ref: ").ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidData, "Invalid HEAD content")
    })?;
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
        },
        None => println!("Object not found"),
    }
    Ok(())
}