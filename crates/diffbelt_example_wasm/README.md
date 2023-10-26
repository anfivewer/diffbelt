# Build

```
cargo rustc --release --target=wasm32-unknown-unknown -- -C target-feature=+multivalue
```

or

```
RUSTFLAGS="-C target-feature=+multivalue" cargo build --release --target wasm32-unknown-unknown
```

