.PHONY: all install

all:
	cargo build --release

install:
	install target/release/lockd /usr/local/bin/lockd
	install target/release/lockctl /usr/local/bin/lockctl
	install lockctl.1 /usr/local/man/man1/lockctl.1
	install lockd.1 /usr/local/man/man1/lockd.1
