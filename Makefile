.PHONY: all install

all:
	cargo build --release

install:
	install target/release/lockd /usr/local/bin/lockd
	install target/release/lockctl /usr/local/bin/lockctl
