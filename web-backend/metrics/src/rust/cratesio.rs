use anyhow::Result;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Crates {
    #[serde(rename = "crate")]
    pub crate_info: CrateInfo,
    pub versions: Vec<Version>,
}

#[derive(Deserialize, Debug)]
pub struct CrateInfo {
    pub repository: String,
}

#[derive(Deserialize, Debug)]
pub struct Version {
    pub num: String,
    pub created_at: String,
}

impl Crates {
    /// retrieves all versions published on crates.io for a given dependency
    pub async fn get_all_versions(name: &str) -> Result<Self> {
        let url = format!("https://crates.io/api/v1/crates/{}", name);

        let client = reqwest::Client::builder().user_agent("whackadep").build()?;

        let body = client.get(&url).send().await?.text().await?;
        serde_json::from_str(&body).map_err(anyhow::Error::msg)
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[tokio::test]
    async fn test_get_all_versions() {
        let creates_io = Crates::get_all_versions("serde").await.unwrap();

        let version_found = creates_io.versions.iter().find(|version| {
            version.num == "1.0.121" && version.created_at == "2021-01-23T21:17:54.177776+00:00"
        });
        assert!(version_found.is_some());
    }
}
