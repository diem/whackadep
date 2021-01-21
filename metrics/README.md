# Metrics

**what's that?**

* we want to compute metrics on diem/diem
* we also want to compute metrics on each dependency diem/diem uses by querying crates.io and github.com
* this every day, and maybe even on demand
* metrics are stored in a database
* in order to prevent errors from messing up with the database, metrics should be calculated first, and then once all the data has been retrieved we should update the db / commit.

the question is: **where do we compute these metrics?**.

1. we can either compute the metric in the web backend. 
  - We could run a service in a thread
  - receive requests on an API (and buffer them in an in-memory queue for example)
  - query 
2. or we could compute the metrics on a different service
  - this would allow us to isolate that service from the web backend
  - not sure what else is great about that way to architect it

So, following pattern 1:

* this crate is used by web-backend to perform metrics

## Architecture

* [external](external). Code that handles querying an external service (e.g. crates.io) to obtain information about a dependency.
* [languages](languages). Code that handles parsing and fetching dependencies in different languages or types of file.
* [db.rs](db.rs). Abstraction around the connection to the mongodb database.
* [git.rs](git.rs). Abstraction around the `git` tool.
* [lib.rs](lib.rs)