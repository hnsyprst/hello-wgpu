## Instancing

Demonstrates shaders, lighting and instancing for multiple models.

![GIF showing screen recording of example output](img/output_small.gif)

Run the native version with `cargo run`:
```
cargo run --package instancing
```

The web version can be run using [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/):

```
wasm-pack build examples/instancing --target web
```