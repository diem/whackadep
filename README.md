# Whack-a-Dep!

A dashboard to update your dependencies.
## Usage

```sh
$ docker-compose build
$ docker-compose up
```

## Architecture

The docker compose file sets up 

- **web-backend**. This is the dashboard that you use to manage your dependencies. It is written with the [Rocket](https://rocket.rs/) web framework.
- **cronjobs**. These are cronjobs that need to run periodically in order to check if new dependency upgrades are available. It is based on [guppy](https://github.com/facebookincubator/cargo-guppy).
- **db**. This is the [PostgreSQL](https://www.postgresql.org/) database where information about dependencies throughout the lifetime of the codebase are persisted.

In addition, the file structure include the following folders:

- **web-frontend**. This is the web UI written in [Vue.js](https://vuejs.org/).
