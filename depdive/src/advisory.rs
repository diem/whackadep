//! This module abstracts interaction with rustsec advisory

use anyhow::Result;
use rustsec::database::{Database, Query};
use semver::Version;
use std::str::FromStr;

pub struct AdvisoryLookup {
    db: Database,
}

impl AdvisoryLookup {
    pub fn new() -> Result<Self> {
        Ok(Self {
            db: Database::fetch()?,
        })
    }

    pub fn get_crate_version_advisories(&self, name: &str, version: &Version) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use rustsec::advisory::id::Id;

    fn get_adivsory_lookup() -> AdvisoryLookup {
        AdvisoryLookup::new().unwrap()
    }

    #[test]
    fn test_advisory_lookup() {
        let lookup = get_adivsory_lookup();
        let advisory = lookup.db.get(&Id::from_str("RUSTSEC-2016-0005").unwrap());
        assert!(advisory.is_some());
    }
}
