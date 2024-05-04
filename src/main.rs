mod database;
mod git;

use std::path::Path;
use std::path::PathBuf;
use structopt::StructOpt;

use crate::database::Database;

const RED: &'static str = "\x1b[1;31m";
const RESET: &'static str = "\x1b[1;0m";

#[derive(StructOpt)]
struct Opt {
    #[structopt(subcommand)]
    command: Mode,
}

#[derive(StructOpt)]
enum Mode {
    /// Initializes a repository to be ready to use with diamond.
    /// Requires that you specify the root branch of that repo,
    /// which is usually `master` or `main`.
    #[structopt()]
    Init(InitOpt),

    /// Creates a new branch with the provided name based on the current branch.
    #[structopt()]
    Create(CreateOpt),

    /// Fetches the most recent contents of the repo's primary branch
    /// and then restacks all of the tracked branches on top of the primary branch.
    #[structopt()]
    Sync,

    /// Submits the contents of the current stack to the remote repo.
    #[structopt()]
    Submit,

    /// Restacks the branches on the current stack onto the most recent version of the priamry branch.
    #[structopt()]
    Restack,

    /// Starts tracking the current branch inside of Diamond.
    /// If no `parent` is provided, assume that the current branch is based on `main`.
    #[structopt()]
    Track(TrackOpt),
}

#[derive(StructOpt)]
struct InitOpt {
    #[structopt(long)]
    remote: String,

    #[structopt(long)]
    root_branch: String,
}

#[derive(StructOpt)]
struct CreateOpt {
    #[structopt()]
    branch: String,
}

#[derive(StructOpt)]
struct TrackOpt {
    #[structopt(long)]
    parent: Option<String>,
}

fn main() -> anyhow::Result<()> {
    let opt = Opt::from_args();
    match &opt.command {
        Mode::Init(ref init_opt) => init(&opt, &init_opt),
        Mode::Create(ref create_opt) => create(&opt, &create_opt),
        Mode::Sync => sync(&opt),
        Mode::Submit => submit(&opt),
        Mode::Restack => restack(&opt),
        Mode::Track(ref track_opt) => track(&opt, track_opt),
    }?;
    Ok(())
}

fn init(_opt: &Opt, init_opt: &InitOpt) -> anyhow::Result<()> {
    let repo_root = git_repo_root(std::env::current_dir()?)?;
    let mut database = open_database(&repo_root)?;
    database.set_remote(&init_opt.remote)?;
    database.set_root_branch(&init_opt.root_branch)?;
    Ok(())
}

fn create(_opt: &Opt, create_opt: &CreateOpt) -> anyhow::Result<()> {
    let repo_root = git_repo_root(std::env::current_dir()?)?;
    let mut database = open_database(&repo_root)?;
    let current_branch = git::get_current_branch(&repo_root)?;
    git::create_branch(&repo_root, &create_opt.branch)?;
    database.create_branch(&current_branch, &create_opt.branch)?;
    Ok(())
}

fn submit(_opt: &Opt) -> anyhow::Result<()> {
    let repo_root = git_repo_root(std::env::current_dir()?)?;
    let mut database = open_database(&repo_root)?;
    let current_branch = git::get_current_branch(&repo_root)?;

    let Some(remote_name) = database.get_remote()? else {
        eprintln!("{RED}Cannot find remote. Configure repo with `dmd init`.{RESET}");
        return Ok(());
    };
    let remote = git::parse_remote(&repo_root, &remote_name)?;

    let branches_in_stack = database.get_branches_in_stack(&current_branch)?;
    for branch in branches_in_stack {
        git::push_branch(&repo_root, "origin", &branch.name)?;
        println!(
            "[{}] -> {}",
            &branch.name,
            remote.new_pr_url(&branch.parent, &branch.name),
        );
    }

    Ok(())
}

fn sync(_opt: &Opt) -> anyhow::Result<()> {
    let repo_root = git_repo_root(std::env::current_dir()?)?;
    let mut database = open_database(&repo_root)?;
    let current_branch = git::get_current_branch(&repo_root)?;

    let Some(remote) = database.get_remote()? else {
        anyhow::bail!("{RED}Cannot find origin. Is the repo initialized?{RESET}");
    };
    let Some(root_branch) = database.get_root_branch()? else {
        anyhow::bail!("{RED}Cannot find root branch. Configure repo with `dmd init`.{RESET}");
    };
    git::pull(&repo_root, &remote, &root_branch)?;

    let branches_in_stack = database.get_branches_in_stack(&current_branch)?;
    for branch in branches_in_stack {
        println!("Restacking `{}` onto `{}`...", branch.name, branch.parent);
        git::pull(&repo_root, &remote, &branch.name)?;
        git::rebase(&repo_root, &branch.parent, &branch.name)?;
    }

    Ok(())
}

fn restack(_opt: &Opt) -> anyhow::Result<()> {
    let repo_root = git_repo_root(std::env::current_dir()?)?;
    let mut database = open_database(&repo_root)?;
    let current_branch = git::get_current_branch(&repo_root)?;

    let branches_in_stack = database.get_branches_in_stack(&current_branch)?;
    for branch in branches_in_stack {
        println!("Restacking `{}` onto `{}`...", branch.name, branch.parent);
        git::rebase(&repo_root, &branch.parent, &branch.name)?;
    }

    Ok(())
}

fn track(_opt: &Opt, track_opt: &TrackOpt) -> anyhow::Result<()> {
    let repo_root = git_repo_root(std::env::current_dir()?)?;
    let mut database = Database::new(repo_root.join(".git").join("diamond.sqlite3"))?;
    let current_branch = git::get_current_branch(&repo_root)?;

    let Some(root_branch) = database.get_root_branch()? else {
        anyhow::bail!("{RED}Cannot find root branch. Configure repo with `dmd init`.{RESET}");
    };

    let parent = match &track_opt.parent {
        Some(parent) => parent.clone(),
        None => root_branch,
    };
    if !git::is_ancestor_of(&repo_root, &parent, &current_branch)? {
        anyhow::bail!("Cannot track {current_branch} as branching off of {parent}, because {parent} is not its ancestor.");
    }
    database.create_branch(&parent, &current_branch)?;
    Ok(())
}

fn git_repo_root(cwd: impl AsRef<Path>) -> anyhow::Result<PathBuf> {
    let cwd = cwd.as_ref();
    let mut candidate_path = Some(cwd);
    while let Some(path) = candidate_path {
        if path.join(".git").is_dir() {
            return Ok(path.to_owned());
        }
        candidate_path = path.parent();
    }
    anyhow::bail!("Working directory is not in a Git repo: {cwd:?}");
}

fn open_database(repo_root: &Path) -> anyhow::Result<Database> {
    Database::new(repo_root.join(".git").join("diamond.sqlite3"))
}
