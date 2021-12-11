build:
	cargo build
clippy:
          cargo clippy
run:
	cargo run

clean:
	rm -f state.json
	cargo clean
