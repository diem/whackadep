# Whack-a-Dep!

A dashboard to update your dependencies.
## Usage

To run the whole thing for development:

```sh
make
```

## Architecture

![whackadep architecture](architecture.png)

The architecture looks like the following:

- [web-frontend](web-frontend). This is the web dashboard written in [Vue.js](https://vuejs.org/) version 3. It queries the web backend to obtain information on dependencies.
- [web-backend](web-backend). This is the dashboard that you use to manage your dependencies. It is written with the [Rocket](https://rocket.rs/) web framework. It also serves a **metrics** API built on top of the [metrics](metrics) crate.
- [db](). This is the [Mongodb](https://www.mongodb.com/) database where information about dependencies throughout the lifetime of the codebase are persisted.
- [cronjobs](cronjobs). These are cronjobs that call the backend's metric API periodically in order to check if new dependency upgrades are available.

## Metrics

Metrics on dependencies are obtained via a **metrics** service within the [web-backend](web-backend).
The service is implemented using the [metrics](metrics) crate.

Essentially, what the service does when called is:

* Make sure it has a local copy of the [diem/diem repository](https://www.github.com/diem/diem).
* Pull the latest changes from the repository.
* Parse any dependency file (e.g. `Cargo.toml`) to obtain a list of dependencies.
* Check if any of these dependencies have updates.
* Store this information in mongodb under a new `_id`.
