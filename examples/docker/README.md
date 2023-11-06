## cargo-fixture docker example

The [fixture](./tests/fixture.rs) uses the [dockertest](https://docs.rs/dockertest/) library.
It launches a postgres docker container and creates a table in the DB. The DB connection URI is shared in the `POSTGRES_URI` environment variable.
No that the container preparation may take some time upon first run as the image is pulled in the process.

The [test code](./tests/test.rs) reads the variable and connects to the DB.

Back in the fixture after the tests are run, the dockertest the docker container is stopped and removed.
