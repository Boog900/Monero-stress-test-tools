# Monero Stress Testing Tools

This repo contains 2 binaries to assist in stress testing monerod:

- `conection-maker`: A binary that maintains a certain number of connection to a Monero node.
- `tx-spam`: A binary that spams a node with txs in blocks from another node.

## Usage

One of the ways to stress test `monerod` with these binaries is explained here, other methods may be possible:

The first step is to set up `monerod`. Increase the amount of connections per IP with `--max-connections-per-ip=XX` and
disconnect it from the network with `--out-peers 0`.

Example: `monerod --out-peers 0 --max-connections-per-ip=1000`

Now you will need to pop blocks to when the tx-pool was huge, i.e. `3139920`. If monerod is synced far from this height
I found this step must be done in steps, I popped ~2000 blocks at a time and cleared the tx-pool after every pop.

With `monerod` at height `3139920` start the connection maker:

```
cargo run -r --bin connection-maker -- --address 127.0.0.1:18080 --connections 100
```

Wait until monerod reports the amount of connections you specified with `status`.

> Even if you don't want any connections you need to at least start the connection-maker as it will trick monerod into
> thinking
> that it is synchronised. (You can exit the connection maintainer after this if you want no connections).

Now with the amount of connections you want you can start spamming the node:

```
cargo run -r --bin tx-spam -- --target-node http://127.0.0.1:18081 --data-node  http://XXXXXXX:18081  --start-height 3139920
```

Now you start monitoring monerod an you should see the tx-pool size increasing, I found starting at height `3139920`
allowed me
to grow the tx-pool to around 90MBs.