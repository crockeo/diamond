use regex::Regex;
use std::path::PathBuf;
use std::process::Command;
use std::{
    path::Path,
    process::{ExitStatus, Stdio},
};

pub struct BranchGuard {
    git_root: PathBuf,
    original_branch: Option<String>,
}

impl BranchGuard {
    pub fn release(mut self) -> anyhow::Result<()> {
        self.release_impl()
    }

    fn release_impl(&mut self) -> anyhow::Result<()> {
        let Some(original_branch) = self.original_branch.take() else {
            anyhow::bail!("Somehow something has already taken ");
        };
        checkout(&self.git_root, &original_branch)?;
        Ok(())
    }
}

impl Drop for BranchGuard {
    fn drop(&mut self) {
        if self.original_branch.is_some() {
            self.release_impl()
                .expect("Failed to move back to original Git branch during BranchGuard drop.");
        }
    }
}

pub fn using_branch(git_root: &Path, branch: &str) -> anyhow::Result<BranchGuard> {
    checkout(git_root, branch)?;
    let _original_branch = get_current_branch(git_root)?;
    let guard = BranchGuard {
        git_root: git_root.to_owned(),
        original_branch: Some(branch.to_owned()),
    };
    Ok(guard)
}

fn checkout(git_root: &Path, branch: &str) -> anyhow::Result<()> {
    let status = Command::new("git")
        .args(["checkout", branch])
        .current_dir(git_root)
        .status()?;
    check_status(status)?;
    Ok(())
}

pub fn get_current_branch(git_root: &Path) -> anyhow::Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--symbolic-full-name", "HEAD"])
        .current_dir(git_root)
        .output()?;
    check_status(output.status)?;
    let stdout = String::from_utf8(output.stdout)?;
    let Some(branch_name) = stdout.trim().strip_prefix("refs/heads/") else {
        anyhow::bail!("Malformed git ref, expected to startw ith `refs/heads/`: {stdout}");
    };
    Ok(branch_name.to_owned())
}

pub fn create_branch(git_root: &Path, branch_name: &str) -> anyhow::Result<()> {
    let status = Command::new("git")
        .args(["checkout", "-b", branch_name])
        .current_dir(git_root)
        .status()?;
    check_status(status)?;
    Ok(())
}

pub fn push_branch(
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
        .status()?;
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

pub fn is_ancestor_of(git_root: &Path, parent_branch: &str, branch: &str) -> anyhow::Result<bool> {
    let status = Command::new("git")
        .args(["merge-base", "--is-ancestor", parent_branch, branch])
        .current_dir(git_root)
        .status()?;
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

pub fn parse_remote(git_root: &Path, remote: &str) -> anyhow::Result<Remote> {
    let output = Command::new("git")
        .args(["remote", "get-url", remote])
        .current_dir(git_root)
        .output()?;

    let url = String::from_utf8(output.stdout)?;
    Remote::parse(&url)
}

pub fn rebase(git_root: &Path, parent_branch: &str, branch: &str) -> anyhow::Result<()> {
    let status = Command::new("git")
        .args(["rebase", parent_branch, branch])
        .current_dir(git_root)
        .status()?;
    check_status(status)?;
    Ok(())
}

pub fn pull(git_root: &Path, origin: &str, branch: &str) -> anyhow::Result<()> {
    let guard = using_branch(git_root, branch)?;
    let status = Command::new("git")
        .args(["pull", "--ff-only", "--no-edit", origin, branch])
        .current_dir(git_root)
        .status()?;
    guard.release()?;
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
