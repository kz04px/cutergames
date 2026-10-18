# client

Connects to a `server`, requests tasks, and plays them by invoking the
`runner` binary as a subprocess -- one `cutergames --games 1` run per task,
against a temporary one-line `--fens` file holding the task's opening.

## Usage

```
client --host 127.0.0.1 --port 9080 --name worker1 --runner cutergames
```

`--runner` is the path to the `cutergames` runner binary (or just its name,
if it's on `PATH`). The client parses the runner's final "Score of ..." and
"Average game length: ..." lines from stdout to build a result, and reports
that back to the server.

If the connection to the server drops (including after a `Shutdown`
message), the client waits a few seconds and reconnects rather than
exiting, so it's safe to start before the server, or start the server
before any client engines are actually reachable.
