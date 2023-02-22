default:
	cargo build --release

all:
	cargo build --relase

build:
	cargo build --relase

test:
	cargo run -- test && make test_clean

fix:
	cargo fix --allow-dirty

fmt:
	cargo fmt