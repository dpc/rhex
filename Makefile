include Makefile.defs

default: $(DEFAULT_TARGET)

.PHONY: run test build doc clean release rrun
test build doc clean:
	cargo $@

simple:
	cargo run

release:
	cargo build --release

run:
	cargo run --release

.PHONY: docview
docview: doc
	xdg-open target/doc/$(PKG_NAME)/index.html
