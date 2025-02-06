gen-wasm-for-extension:
	wasm-pack build crates/cline-wasm --target bundler --out-dir ./extension/cline --release