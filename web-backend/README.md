# Web-backend

If you're running Mongodb via the docker-compose command of the main [README](README.md), you can run this crate manually to test it via the following command:

```
MONGODB_URI="mongodb://root:password@localhost:27017" cargo run
```

the app will be served on a different port (8000) from the docker-compose command (8081), and will connect to the mongodb database served there.
