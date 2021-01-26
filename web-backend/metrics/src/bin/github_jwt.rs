use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::{
    env, fs,
    time::{Duration, SystemTime},
};

/// Our claims struct, it needs to derive `Serialize` and/or `Deserialize`
#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    iss: String, // Optional. Issuer
    exp: usize, // Required (validate_exp defaults to true in validation). Expiration time (as UTC timestamp)
    iat: usize, // Optional. Issued at (as UTC timestamp)
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        println!("usage: cargo run --bin github_jwt <PEM_FILE_PATH>");
        return;
    }

    let key = fs::read(&args[1]).unwrap();

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    let expiration_time = now.checked_add(Duration::from_secs(60 * 10)).unwrap();

    let my_claims = Claims {
        iss: "97730".to_string(),
        iat: now.as_secs() as usize,
        exp: expiration_time.as_secs() as usize,
    };

    println!("claim: {:?}", my_claims);

    let token = encode(
        &Header::new(Algorithm::RS256),
        &my_claims,
        &EncodingKey::from_rsa_pem(&key).unwrap(),
    )
    .unwrap();

    println!("token: {}", token);

    println!("example: $ curl -i -H \"Authorization: Bearer {}\" -H \"Accept: application/vnd.github.v3+json\" https://api.github.com/app", token);
}
