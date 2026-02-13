## Nihil Elegans

Alright, I'll keep it simple, since this is merely a PoC and not professional grade work

### Commands
These apply to both the server and the client as the arguments are interchangeable
#### Running
```bash
cargo run --bin <server OR client>
```
#### Getting Help
```bash
cargo run --bin <server OR client> -- --help
```

### Examples
#### Example 1: (Running the server, but changing the default address and port to listen on)
```bash
# This would require root access
cargo run --bin server -- -a 0.0.0.0 -p 53
```
#### Example 2: (Running the client, but changing the default XOR encryption key)
```bash
# Be sure the server uses the same key or no bueno
cargo run --bin client -- --xor-key lachrymose
```
