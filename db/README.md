# Database

We chose [MongoDB](https://www.mongodb.com/) as database for a few reasons:

* **Flexibility**. We predict that we will continuously add new rules to the priority and risk engines, this translates into adding fields in what we store (or columns in a relational database). MongoDB allows us to do this easily, while remaining backward compatible with minimal effort.
* **Documents work**. A client needs all the data that we store in the DB, so instead of dividing this data into several tables and doing a bunch of joins, why not store everything in a single BSON document? This is essentially what we do. Note that this might not scale super well for mono repos, or for large repositories in general. Currently, a document is ~2.5MB, and MongoDB has a maximum document size of 15MB. At some point, it might make sense to just store these documents in a blob storage.
* **Queries supported**. We could have simply used the filesystem as well, but mongodb allows us to perform some non-heavy queriesm, and that's pretty handy!
* **Fun**. It's more fun, for a project that's not running at scale it is not that important.

## Databases

We only use one database: `whackadep` currently.


## Collections

We use two collections:

* `config`, to store configurations associated with every repository we are tracking.
* `dependencies`, to store the result of our analysis.
## Config

We store each repository configuration there as one document containing the address of the repository.

## Dependencies

We store the analysis of a repository as documents reprensenting an [Analaysis](https://github.com/diem/whackadep/blob/main/web-backend/metrics/src/analysis.rs#L19) structure:

```rust
struct Analysis {
    /// The full repository link (e.g. https://github.com/diem/diem.git)
    repository: String,
    /// The SHA-1 hash indicating the exact commit used to analyze the given repository.
    commit: String,
    /// the time at which the analysis was done
    timestamp: DateTime<Utc>,
    /// metadata about previous analysis
    previous_analysis: Option<PreviousAnalysis>,
    /// The result of the rust dependencies analysis
    rust_dependencies: RustAnalysis,
}
```

where `rust_dependencies` contain the result of the analysis on the rust dependencies:

```rust
struct RustAnalysis {
    /// Note that we do not use a map because the same dependency can be seen several times.
    /// This is due to different versions being used or/and being used directly and indirectly (transitively).
    dependencies: Vec<DependencyInfo>,

    /// the result of running cargo-audit
    rustsec: RustSec,

    /// A summary of the changes since last analysis
    change_summary: Option<ChangeSummary>,
}
```

For example, a dependency document in MongoDB looks like:

```json
{
    _id: ObjectId('6026301b006ce0a900d836af'),
    repository: 'https://github.com/diem/diem.git',
    commit: '4f8a6be752b312fcddc71ac22470aff96b574fc4\n',
    timestamp: '2021-02-12T07:36:59.655072700Z',
    previous_analysis: {
        commit: '4f8a6be752b312fcddc71ac22470aff96b574fc4\n',
        timestamp: '2021-02-12T02:57:56.105586500Z'
    },
    rust_dependencies: {
        dependencies: [
            {
                name: 'Inflector',
                version: '0.11.4',
                repo: {
                    'crates-io': true
                },
                dev: true,
                direct: false,
                update: null
            },
            {
                name: 'adler',
                version: '0.2.3',
                repo: {
                    'crates-io': true
                },
                dev: false,
                direct: false,
                update: {
                    versions: [
                        '1.0.0',
                        '1.0.1'
                    ],
                    update_metadata: {
                        changelog_url: null,
                        changelog_text: null,
                        commits_url: null,
                        commits: []
                    },
                    build_rs: false
                }
            },
        ]
    }
}
```
