.PHONEY:
wasm:
	wasm-pack build --target web --features wasm
