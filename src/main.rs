use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt)]
struct Opt {
    config_path: Option<PathBuf>,

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

    /// Submits the contents of the current branch to the remote repo.
    /// If `stack` is provided: submit the contents of all branches on the current stack.
    #[structopt()]
    Submit(SubmitOpt),

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
        Command::Submit(ref submit_opt) => submit(&opt, &submit_opt).await,
        Command::Restack => restack(&opt).await,
    }?;
    Ok(())
}

async fn init(opt: &Opt, init_opt: &InitOpt) -> anyhow::Result<()> {
    todo!()
}

async fn create(opt: &Opt, create_opt: &CreateOpt) -> anyhow::Result<()> {
    todo!()
}

async fn sync(opt: &Opt) -> anyhow::Result<()> {
    todo!()
}

async fn submit(opt: &Opt, submit_opt: &SubmitOpt) -> anyhow::Result<()> {
    todo!()
}

async fn restack(opt: &Opt) -> anyhow::Result<()> {
    todo!()
}
