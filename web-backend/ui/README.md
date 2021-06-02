# Web-backend

The backend is serving an API with the following routes:

* `/` returns the list of routes 
* `/refresh?repo=<REPO>` sends a message to the [metrics service](metrics/) to start analyzing the given <REPO>
* `/dependencies?repo=<REPO>` retrieves the latest analysis done on <REPO>
* `/repos` retrieves all the repositories saved in the configuration
* `/add_repo` adds a new repository to the configuration

It is pretty simply: it uses the [Rocket](https://rocket.rs/) framework to serve the webpage, and the [metrics](metrics/) crate to read from storage or start analyses of dependencies.

## Running without Docker

If you're running Mongodb via the docker-compose command of the main [README](README.md), you can run this crate manually to test it via the following command:

```
MONGODB_URI="mongodb://root:password@localhost:27017" cargo run
```

the app will be served on a different port (8000) from the docker-compose command (8081), and will connect to the mongodb database served there.

