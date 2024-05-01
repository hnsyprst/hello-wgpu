To target wasm (used by index.html), build with wasm-pack (https://rustwasm.github.io/wasm-pack/installer/):
 1 .`wasm-pack build --target web --out_dir web/pkg`
 2. copy index.html and /res into /web