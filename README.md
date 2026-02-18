# CuterGames - WIP
A match runner for engines following the UGI protocol.

---

## First Class Support
The rules for some games may be implemented:
- [x] Ataxx
- [ ] Chess

---

## Command Line Parameters
### --player
> The `--player` flag defines a new player.
> The name of the player.
> `name=name`
>
> The path to the executable.
> `path=/path/to/executable`
>
> The protocol being used (ugi/uai/uci)
> `proto=proto`

Example:
`--player name="Player Name" path=/players/example/executable proto=ugi`

### --threads
> The number of threads to run matches on simultaneously.

Example:
`--threads 4`

---

## Thanks
- Me
