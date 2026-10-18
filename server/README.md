# server

Distributes match-running work across connected clients. The server itself
never spawns an engine or plays a game -- it only hands out jobs and
collects results; the `client` crate is what actually invokes the runner.

## Usage

```
server --port 9080 --http-port 8080 [--jobs jobs.json]
```

The server stays running and waits for work: `--jobs` is optional and only
loads jobs at startup, it isn't required to keep the server alive, and more
jobs can be submitted at any time while it's running (see `submit` below).

## Status page

Open `http://<host>:8080/` (the `--http-port`, separate from the worker
protocol's `--port`) in a browser for a live-updating table of every job:
game, match-up, progress bar, W-L-D, and finished/running status. It polls
`GET /api/jobs`, which returns the same data as JSON if you'd rather consume
it another way (`curl http://<host>:8080/api/jobs`).

`jobs.json` is an array of jobs, see `example_jobs.json`. Each job describes
a match between two engines (`player1`/`player2`) played as `num_games`
individual games, alternating which engine moves first, cycling through
`fens` two games at a time so both colours see the same opening. `tc` is the
engine's time control as a runner `--player` key: `tc=<ms>[+<inc ms>]`,
`st=<ms>` or `depth=<n>`, defaulting to `st=10`.

An engine can also have `options`: a map of option names to values, sent to
the engine as `setoption name <name> value <value>` in the order listed (the
same as the runner's `option.<Name>=<value>`). Values may be strings, numbers
or booleans; a name can't be empty or contain `=`, and neither can contain a
line break. A job with a bad option is rejected when it's submitted.

## Submitting jobs to a running server

```
server submit --port 9080 --jobs more_jobs.json
```

Connects to a running server, submits every job in the file, prints the id
each was assigned (or why it was rejected -- unknown game, empty `fens`, or
`num_games` of 0), and exits.

## Protocol

A client connects over TCP and exchanges newline-delimited JSON messages
(see the `protocol` crate):

- Client -> server: `Hello { name }`, `RequestTask`, `TaskResult { .. }`,
  `SubmitJob(JobSpec)`
- Server -> client: `Welcome`, `Task { .. }`, `NoWork`, `Shutdown`,
  `JobSubmitted(id)`, `Error(message)`

A typical worker session: connect, send `Hello`, then repeatedly send
`RequestTask` (retrying later on `NoWork`), play the returned `Task`
(via the runner), and send back a `TaskResult`. See `client` for the
implementation, or `submit` above for a job-submitting session instead.

If a client disconnects while holding a game it hasn't reported a result
for, that game is put back in the queue for another client to pick up.
