# Metrics

Crate to analyze dependencies.
See [project's README](../README.md).

## Architecture

Ideally, this would be easily extendable for any languages, but to move fast let's just create the code for rust and then seek to make it more generable and extandable.

Current directory structure at the time of this writing:

* [dependabot](dependabot). Contains scripts to use [dependabot](https://github.com/dependabot/dependabot-core/) (a useful library to check dependencies of a repo).
* [resources](resources). Contains results of guppy execution for test or to populate the database with some data.
* [src/bin](src/bin). Contains CLIs to populate the database with test data.
* [src/common](src/common). Analysis code relevant for any languages (e.g. querying github.com).
* [src/rust](src/rust). Code that handles parsing and fetching dependencies in different languages or types of file.
* [src/db.rs](src/db.rs). Abstraction around the mongodb database. Perhaps this should be a "model" thing.
* [src/git.rs](src/git.rs). Abstraction around the `git` tool.

## Documentation

In the root folder this command will generate and open doc:

```
make doc
```

## Testing

One can use the script [populate_test_data](bin/populate_test_data) to populate a mongodb instance with [testing data](resources/test).

```sh
$ MONGODB_URI="mongodb://root:password@localhost:27017" cargo run --bin populate_test_data
```

This will not work if you don't have rust, or if you haven't initialized dependabot:

```
cd dependabot
rbenv install 2.6.6
rbenv global 2.6.6
bundle install
```
