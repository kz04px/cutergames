# CuterGames - WIP
A match runner for engines following the UGI protocol (UAI, UCI and TEI are
also supported).

---

## Supported Games
- Ataxx
- Chess
- Connect4
- Droptaxx
- Gomoku
- Isolation
- Pijersi
- Tak (6x6)

---

## Openings
`openings/` holds ready-made opening lists for `--fens`, one position per line
(blank lines and `#` comments are ignored), grouped by game and named by the
number of plies played from the start position, e.g. `openings/ataxx/ply3.txt`.

---

## Command Line Parameters

`--settings <path>`
`--tc <ms>[+<inc ms>]` / `--st <ms>` / `--depth <n>`
`--player name=<name> path=<path> proto=<ugi/uai/uci/tei> parameters=<params> <tc=<ms>[+<inc ms>]/st=<ms>/depth=<n>> option.<Name>=<value> debug`
`--game <ataxx/chess/connect4/droptaxx/gomoku/isolation/pijersi/tak>`
`--fens <path>`
`--games <number>`
`--threads <number>`
`--updates <number>`
`--trinomial alpha=<a> beta=<b> elo0=<e0> elo1=<e1> model=<normalized/logistic/bayesian> autostop`
`--pentanomial alpha=<a> beta=<b> elo0=<e0> elo1=<e1> model=<normalized/logistic> autostop`
`--results <path>`
`--resume <path>`
`--verbose`

`--tc`, `--st` and `--depth` set the time control for every player, and a
player's own `tc`, `st` or `depth` overrides it.

Run `cutergames --help` for the full list.

---

## SPRT
`--trinomial` tests on win/draw/loss counts and `--pentanomial` on pair results
(each opening is played with both colours). The calculation follows
[fastchess](https://github.com/Disservin/fastchess), so the same results give
the same LLR. `model` chooses what `elo0` and `elo1` mean:

- `normalized` (the default): normalised elo, nElo
- `logistic`: ordinary elo
- `bayesian`: BayesElo; trinomial only

`alpha` and `beta` are required, `elo0` must be less than `elo1`, and with
`autostop` the tournament ends when the LLR reaches either bound.

---

## Settings File
`--settings <path>` loads a JSON file (see `example.json` for every key) and
uses it as the starting point. Anything else on the command line then
overrides it, so a file can hold the usual setup and individual runs can change
a few values:

```
cutergames --settings match.json --games 100 --threads 4
```

- Keys: `players`, `game`, `openings`, `games`, `threads`, `updates`,
  `trinomial`, `pentanomial`, `verbose`, and one of `tc`, `st` or `depth` for
  every player's time control. Unknown keys are an error. The SPRT blocks take
  `enabled`, `autostop`, `alpha`, `beta`, `elo0`, `elo1` and `model`.
- A player takes `path` (required), `name`, `proto`, `parameters`, one of `tc`,
  `st` or `depth` (a string or number, as on the command line) to override the
  global time control, `debug` and `options`, a map of engine option names to
  values sent with `setoption`.
- `--player`, `--trinomial` and `--pentanomial` on the command line replace the
  file's value as a whole instead of merging with it.
- Relative paths, such as `openings`, are relative to the working directory.

## Results File

`--results <path>` saves the match to a JSON file, rewritten at every results
update and once more when the match ends. A match won't start if the file
already exists, so an earlier match's results can't be overwritten. The file
is a settings file with every setting written out, including any command line
overrides, plus a `results` block that `--settings` skips, so

```
cutergames --settings results.json --results rerun.json
```

plays the same match again. The `results` block holds the CuterGames version
and commit, start and finish times, the score (`wld` and the pentanomial
`pairs`, +2 to -2, from the first player's point of view), Elo and nElo, the
SPRT state, and crashes, timeouts and illegal moves per player. It also holds
the sha256 of each engine and the openings file, taken when the match started,
so a rerun can check it's using the same files.

The file is rewritten after every game. `status` in the `results` block is
`running` while the match plays, then `complete` once it reaches `--games` or
an SPRT stops it with `autostop`, or `stopped` if it ended before that, e.g.
with Ctrl-C. Until a match is complete, the `results` block also holds a
`resume` block with what's needed to continue it, which

```
cutergames --resume results.json
```

does, saving to the same file. Games that were being played when the match
stopped are played again, and the match continues from where it got to. Only
`--threads`, `--updates` and `--verbose` can be given with `--resume`, and it
refuses to continue if the engines or openings file have changed since the
match started. A match that was killed rather than stopped can still be
resumed, since its file is left as `running` with everything up to the last
finished game.

---

## Thanks
- Me
