# fit: A Version Control System

fit is a version control system inspired by Git. It offers similar functionality with commands to initialize, clone, add, remove, commit, log, reset, and check the status of your repositories. fit is written in Rust and uses SHA-1 for hashing, zlib for compression, and offers a simple CLI for interaction.

## Features

- Initialize a new repository
- Clone an existing repository (under development)
- Log commit history
- Add and remove files to/from the staging area
- Commit changes
- View the contents of repository objects
- Check the status of the working directory
- Reset to a specific commit
- Viewing Diff between two commits
- Creating and switching to branches
- Cloning branches

## Installation

To install fit, you need to have Rust installed on your system. If you don't have Rust installed, you can get it from [here](https://www.rust-lang.org/tools/install).

1. Clone the repository:
    ```sh
    git clone https://github.com/yourusername/fit.git
    cd fit
    ```

2. Build the project:
    ```sh
    cargo build --release
    ```

3. Add the `fit` executable to your PATH:
    ```sh
    export PATH=$PATH:/path/to/fit/target/release
    ```

## Usage

Here are the commands you can use with fit:

### Initialize a Repository

```sh
fit init
```

### Clone a Repository (Under Development)
```sh
fit clone <url>
```
### Log Commit History
```sh
fit log
```
### Add a File to the Staging Area
```sh
fit add <file-path>
```
### Remove a File from the Staging Area
```sh
fit rm <file-path>
```
### Commit Changes
```sh
fit commit -m "Commit message"
```
### View the Contents of an Object
```sh
fit cat-file <hash>
```
### Check the Status of the Working Directory
```sh
fit status
```
### Reset to a Specific Commit
```sh
fit reset <commit-hash>
```
### Viewing Diff of currently staged items and latest commit
```sh
fit diff
```
### Viewing Diff of any 2 Commits
```sh
fit diff commit <commit_1> <commit_2>
```
### Stashing un-commited changes for a clean work-tree
```sh
fit stash
```
### Popping last stashed content to present working directory
```sh
fit stash pop
```
## Branch Management

### List All Branches

```sh
fit branch list
```

### Create a New Branch
```sh
fit branch create <branch_name>
```

### Delete a Branch
```sh
fit branch delete <branch_name>
```

### Checkout a Branch / Switch to a branch
```sh
fit branch checkout <branch_name>
```

### Create and Checkout a New Branch
```sh
fit branch checkout-new <branch_name>
```

## Example Workflow

### Initialize a new repository:

```sh
fit init
```
### Add files to the staging area:

```sh
fit add file1.txt
fit add file2.txt
```
### Commit the changes:

```sh
fit commit -m "Initial commit"
```
### Check the commit history:

```sh
fit log
```
### View the status of the working directory:

```sh
fit status
```
### Reset to a previous commit:

```sh
fit reset <commit-hash>
```

## Contributing
Contributions are welcome! Feel free to submit issues, fork the repository, and open pull request