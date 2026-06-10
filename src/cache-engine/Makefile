NAME := cache-engine
SRC := src/cache-engine/src

build:
	cargo build --release

run: build
	./target/release/$(NAME)

dev:
	cargo run

test:
	cargo test --all --verbose

bench:
	cargo test --benches -- --test-threads=1

fmt:
	cargo fmt -- --check

lint:
	cargo clippy -- -D warnings

clean:
	cargo clean
