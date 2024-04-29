mod database;
mod git;

use std::path::Path;
use std::path::PathBuf;
use structopt::StructOpt;

use crate::database::Database;

#[derive(StructOpt)]
struct Opt {
    #[structopt(subcommand)]
    command: Command,
}

#[derive(StructOpt)]
enum Command {
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
}

#[derive(StructOpt)]
struct InitOpt {
    #[structopt(long)]
    root_branch: String,
}

#[derive(StructOpt)]
struct CreateOpt {
    #[structopt()]
    branch: String,
}

#[derive(StructOpt)]
struct SubmitOpt {
    #[structopt(long)]
    stack: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let opt = Opt::from_args();
    match &opt.command {
        Command::Init(ref init_opt) => init(&opt, &init_opt).await,
        Command::Create(ref create_opt) => create(&opt, &create_opt).await,
        Command::Sync => sync(&opt).await,
        Command::Submit => submit(&opt).await,
        Command::Restack => restack(&opt).await,
    }?;
    Ok(())
}

async fn init(opt: &Opt, init_opt: &InitOpt) -> anyhow::Result<()> {
    let repo_root = git_repo_root(std::env::current_dir()?)?;
    let mut database = open_database(&repo_root)?;
    database.set_root_branch(&init_opt.root_branch)?;
    Ok(())
}

async fn create(opt: &Opt, create_opt: &CreateOpt) -> anyhow::Result<()> {
    let repo_root = git_repo_root(std::env::current_dir()?)?;
    let mut database = open_database(&repo_root)?;
    let current_branch = git::get_current_branch(&repo_root).await?;
    git::create_branch(&repo_root, &create_opt.branch).await?;
    database.create_branch(&current_branch, &create_opt.branch)?;
    Ok(())
}

async fn submit(opt: &Opt) -> anyhow::Result<()> {
    let repo_root = git_repo_root(std::env::current_dir()?)?;
    let mut database = open_database(&repo_root)?;
    let current_branch = git::get_current_branch(&repo_root).await?;
    let branches_in_stack = database.get_branches_in_stack(&current_branch)?;

    // TODO: this assumes we're always pushing to the remote "origin,"
    // but the repo may have a different remote name.
    // maybe set this up as part of `init`?
    for branch_in_stack in branches_in_stack {
        git::push_branch(&repo_root, "origin", &branch_in_stack).await?;
    }

    todo!()
}

async fn sync(opt: &Opt) -> anyhow::Result<()> {
    todo!()
}

async fn restack(opt: &Opt) -> anyhow::Result<()> {
    todo!()
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
