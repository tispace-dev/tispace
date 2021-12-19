build:
	cargo build
release:
	cargo build --release
clippy:
	RUSTFLAGS=-Dwarnings cargo clippy
run:
	cargo run
fmt:
	cargo fmt
test:
	cargo test
clean:
	rm -f state.json
	cargo clean
