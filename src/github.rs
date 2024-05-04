pub async fn create_pull_request(
    organization: &str,
    repo: &str,
    base_branch: &str,
    branch: &str,
    title: &str,
    body: &str,
) -> anyhow::Result<()> {
    println!("{organization}, {repo}, {base_branch}, {branch}, {title}, {body}");
    Ok(())
}
