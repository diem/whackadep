# Metrics

Crate to analyze dependencies.
See [project's README](../README.md).
## Architecture

Ideally, this would be easily extendable for any languages, but to move fast let's just create the code for rust and then seek to make it more generable and extandable.

Current directory structure at the time of this writing:

* [external](external). Code that handles querying an external service (e.g. crates.io) to obtain information about a dependency.
* [languages](languages). Code that handles parsing and fetching dependencies in different languages or types of file.
* [db.rs](db.rs). Abstraction around the connection to the mongodb database.
* [git.rs](git.rs). Abstraction around the `git` tool.
