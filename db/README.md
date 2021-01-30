# Database

We chose [MongoDB](https://www.mongodb.com/) as database for a few reasons:

* We predict that we will continuously add new rules to the priority and risk engines, this translates into adding fields in what we store (or columns in a relational database). MongoDB allows us to do this easily, while remaining backward compatible with minimal effort.
* A client needs all the data that we store in the DB, so instead of dividing this data into several tables and doing a bunch of joins, why not store everything in a single BSON document? This is essentially what we do. Note that this might not scale super well for mono repos, or for large repositories in general. Currently, a document is ~2.5MB, and MongoDB has a maximum document size of 15MB. At some point, it might make sense to just store these documents in a blob storage.
* It's more fun, for a project that's not running at scale it is not that important.

## Analysis

We store the analysis of a repository as documents reprensenting an [Analaysis](https://github.com/diem/whackadep/blob/main/web-backend/metrics/src/analysis.rs#L12) structure:

```rust
pub struct Analysis {
    commit: String,
    rust_dependencies: RustAnalysis,
}
```

note that mongodb adds an `_id` field to every document that handily contains, among other data, the date.
```

