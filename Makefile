PREFIX ?= /usr/local
DESTDIR ?=

.PHONY: build release install uninstall clean run

build:
	cargo build

release:
	cargo build --release

run:
	cargo run

install: release
	install -Dm755 target/release/openpatience $(DESTDIR)$(PREFIX)/bin/openpatience
	install -Dm644 openpatience.desktop $(DESTDIR)$(PREFIX)/share/applications/openpatience.desktop
	install -Dm644 README.md $(DESTDIR)$(PREFIX)/share/doc/openpatience/README.md
	@echo "Installed to $(DESTDIR)$(PREFIX)/bin/openpatience"

uninstall:
	rm -f $(PREFIX)/bin/openpatience
	rm -f $(PREFIX)/share/applications/openpatience.desktop
	rm -rf $(PREFIX)/share/doc/openpatience

clean:
	cargo clean
