## cargo-fixture HTTP example

In the [fixture](./tests/fixture.rs), a very simple HTTP responder is run, listening on an OS-provided TCP port. The port number is then exposed in the the `HTTP_PORT` environment variable.

The [test code](./tests/test.rs) then reads this variable and performs a request against the server. Back in the fixture, the server is shut down after tests are run.
