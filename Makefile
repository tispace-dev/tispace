build:
	cargo build
clippy:
	RUSTFLAGS=-Dwarnings cargo clippy
run:
	cargo run
fmt:
	cargo fmt
clean:
	rm -f state.json
	cargo clean
