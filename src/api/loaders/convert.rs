use crate::app::models::Paper;

pub fn toml_to_paper(path: &str) -> anyhow::Result<Paper> {
    let content = std::fs::read_to_string(path)?;
    let paper: Paper = toml::from_str(&content)?;
    Ok(paper)
}