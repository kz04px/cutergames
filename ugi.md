# UGI
The UGI protocol based on the UCI protocol from chess.

---

## ugi
```
> ugi
< ugiok
```

---

## isready
```
isready
readyok
```

---

## query [question]
`query turn` ---> `response [int]`
`query gameover` ---> `response [true/false]`
`query result` ---> `response p1win/p2win/draw/none`

---

## Example
```
> ugi
< ugiok
> isready
< readyok
> quit
```
