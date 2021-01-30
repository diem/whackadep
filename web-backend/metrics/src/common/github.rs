use anyhow::Result;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::{
    fs,
    time::{Duration, SystemTime},
};
use tracing::debug;

/// The function will retrieve repository metadata (like stargazers_count).
/// It needs a Github personal access token (PAT) to function.
pub async fn get_repository_info(
    access_token: Option<String>,
) -> Result<octocrab::models::Repository> {
    // get access token from ENV
    let access_token = access_token.unwrap_or_else(|| {
        std::env::var("GITHUB_TOKEN").expect("a GITHUB_TOKEN environment variable is missing")
    });

    // create client
    let octocrab = octocrab::OctocrabBuilder::new()
        .personal_token(access_token)
        //        .base_url("https://api.github.com/")?
        .build()?;

    debug!("{:?}", octocrab);

    octocrab
        .get("https://api.github.com/app/", None::<&()>)
        .await
        .map_err(anyhow::Error::msg)
}

pub async fn get_access_token(key_path: &Path) -> Result<String> {
    #[derive(Debug, Serialize, Deserialize)]
    struct Claims {
        iss: String, // issuer
        exp: usize,  // expiration time (limited to 10 min)
        iat: usize,  // issued at
    }

    let key = fs::read(key_path)?;

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("getting time must work");
    let expiration_time = now
        .checked_add(Duration::from_secs(60 * 10))
        .expect("overflowed adding 10min to now");

    let my_claims = Claims {
        iss: "97730".to_string(),
        iat: now.as_secs() as usize,
        exp: expiration_time.as_secs() as usize,
    };

    let token = encode(
        &Header::new(Algorithm::RS256),
        &my_claims,
        &EncodingKey::from_rsa_pem(&key).unwrap(),
    )?;

    Ok(token)
}

#[cfg(test)]
mod tests {

    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_get_app_info() {
        let mut key_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        key_path.push("resources/keys/whackadep.2021-01-25.private-key.pem");

        let token = get_access_token(&key_path).await.unwrap();
        let repo = get_repository_info(Some(token)).await.unwrap();
        println!("{:?}", repo);
    }
}
