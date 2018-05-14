.PHONY: all install

all:
	cargo build --release

install:
	install -D target/release/lockd /usr/local/bin/lockd
	install -D target/release/lockctl /usr/local/bin/lockctl
	install -D lockctl.1 /usr/local/man/man1/lockctl.1
	install -D lockd.1 /usr/local/man/man1/lockd.1

uninstall:
	rm /usr/local/bin/lockd
	rm /usr/local/bin/lockctl
	rm /usr/local/man/man1/lockctl.1
	rm /usr/local/man/man1/lockd.1
