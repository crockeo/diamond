use regex::Regex;
use std::{
    path::Path,
    process::{ExitStatus, Stdio},
};
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

pub async fn push_branch(
    git_root: impl AsRef<Path>,
    remote: impl AsRef<str>,
    branch_name: impl AsRef<str>,
) -> anyhow::Result<()> {
    let (git_root, remote, branch_name) =
        (git_root.as_ref(), remote.as_ref(), branch_name.as_ref());

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

pub async fn is_ancestor_of(
    git_root: &Path,
    parent_branch: &str,
    branch: &str,
) -> anyhow::Result<bool> {
    let status = Command::new("git")
        .args(["merge-base", "--is-ancestor", parent_branch, branch])
        .current_dir(git_root)
        .status()
        .await?;
    Ok(status.success())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Remote {
    pub organization: String,
    pub repo: String,
}

impl Remote {
    fn parse(remote_url: &str) -> anyhow::Result<Self> {
        // TODO: make this support other, non-github providers
        let re = Regex::new(
            "(git@github.com:|https://github.com/)(?P<organization>[^/]+)/(?P<repo>[^/.]+)(\\.git)?",
        )?;
        let Some(captures) = re.captures(&remote_url) else {
            anyhow::bail!("Malformed remote URL: {remote_url}");
        };
        Ok(Remote {
            organization: captures["organization"].trim().to_owned(),
            repo: captures["repo"].trim().to_owned(),
        })
    }

    pub fn new_pr_url(&self, base_branch: &str, branch_to_merge: &str) -> String {
        format!(
            "https://github.com/{}/{}/compare/{base_branch}...{branch_to_merge}?expand=1",
            self.organization, self.repo,
        )
    }
}

pub async fn parse_remote(git_root: &Path, remote: &str) -> anyhow::Result<Remote> {
    let output = Command::new("git")
        .args(["remote", "get-url", remote])
        .current_dir(git_root)
        .output()
        .await?;

    let url = String::from_utf8(output.stdout)?;
    Remote::parse(&url)
}

pub async fn rebase(git_root: &Path, parent_branch: &str, branch: &str) -> anyhow::Result<()> {
    let status = Command::new("git")
        .args(["rebase", parent_branch, branch])
        .current_dir(git_root)
        .status()
        .await?;
    check_status(status)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_remote_url_ssh() -> anyhow::Result<()> {
        let remote = Remote::parse("git@github.com:crockeo/diamond")?;
        assert_eq!(
            remote,
            Remote {
                organization: "crockeo".to_owned(),
                repo: "diamond".to_owned(),
            },
        );
        Ok(())
    }

    #[test]
    fn test_parse_remote_url_https() -> anyhow::Result<()> {
        let remote = Remote::parse("https://github.com/crockeo/diamond")?;
        assert_eq!(
            remote,
            Remote {
                organization: "crockeo".to_owned(),
                repo: "diamond".to_owned(),
            },
        );
        Ok(())
    }
}
