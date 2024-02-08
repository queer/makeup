# makeup

Pretty CLI / TUI interfaces.

MSRV 1.70.


## READ ME FIRST

makeup is still early-stage!!! Treat it like the alpha project it is.

[crates.io](https://crates.io/crates/makeup)

### Usage examples

See [`examples/`](https://github.com/queer/makeup/tree/mistress/makeup/examples)!

### Demos

`cargo run --example wave`

## Setup

Install [pre-commit](https://pre-commit.com/).

```bash
pre-commit install
pre-commit autoupdate
cargo install cargo-audit
```

## Features

- 60fps by default.
- Input and render are fully decoupled, ie input can NEVER block rendering.
- Message-passing-like architecture
  - Components are updated and rendered asynchronously.
  - Components must not reference each other directly, but instead communicate
    via message passing.
  - Component updates are just reading the message queue from the mailbox, and
    updating the component's state accordingly. makeup assumes that **any**
    potentially-blocking task will be moved out of the update/render loop via
    `tokio::spawn` or similar, and managed via message-passing.
- Render-backend-agnostic.
  - Render backends are async.
  - Default backends are memory and UNIX-compatible terminal.
  - Render backends can be implemented for other protocols!
    - Provided to the UI on instantiation.
    - Ideas: WASM + `<canvas>`? Multiplex over the network?
