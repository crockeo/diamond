use std::{path::Path, process::{ExitStatus, Stdio}};
use tokio::process::Command;

pub async fn get_current_branch(git_root: &Path) -> anyhow::Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--symbolic-full-name", "HEAD"])
        .current_dir(git_root)
        .output()
        .await?;
    check_status(output.status)?;
    let stdout = String::from_utf8(output.stdout)?;
    let Some(branch_name) = stdout.trim().strip_prefix("refs/heads/") else {
        anyhow::bail!("Malformed git ref, expected to startw ith `refs/heads/`: {stdout}");
    };
    Ok(branch_name.to_owned())
}

pub async fn create_branch(git_root: &Path, branch_name: &str) -> anyhow::Result<()> {
    let status = Command::new("git")
        .args(["checkout", "-b", branch_name])
        .current_dir(git_root)
        .status()
        .await?;
    check_status(status)?;
    Ok(())
}

pub async fn push_branch(git_root: &Path, remote: &str, branch_name: &str) -> anyhow::Result<()> {
    let refspec = format!("refs/heads/{branch_name}:refs/heads/{branch_name}");
    let status = Command::new("git")
        .args(["push", "--force-with-lease", remote, &refspec])
        .current_dir(git_root)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await?;
    check_status(status)?;
    Ok(())
}

fn check_status(status: ExitStatus) -> anyhow::Result<()> {
    if !status.success() {
        let status_message = if let Some(code) = status.code() {
            format!("with status code: {code}.")
        } else {
            "without a status code. It was probably killed via signal.".to_owned()
        };
        anyhow::bail!("Comamnd failed {status_message}");
    }
    Ok(())
}
