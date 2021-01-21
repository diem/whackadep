set -xe

cd $REPO_DIR

# update to latest
git pull

# get list of dependencies
cargo guppy select --kind ThirdParty > ../third_party.deps
cargo guppy select --kind DirectThirdParty > ../direct_third_party.deps

# guppy summaries
cargo x generate-summaries
cp target/summaries/summary-release.toml ../summary.json
