default:
	cargo build --release

all:
	cargo build --release

build:
	cargo build --release

dev:
	RUST_LOG=debug cargo run

run:
	RUST_LOG=info cargo run

test:
	cargo run -- test

fix:
	cargo fix --allow-dirty

fmt:
	cargo fmt