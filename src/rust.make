default:
	cargo build --release

all:
	cargo build --relase

build:
	cargo build --relase

test:
	cargo run -- test

fix:
	cargo fix --allow-dirty

fmt:
	cargo fmt