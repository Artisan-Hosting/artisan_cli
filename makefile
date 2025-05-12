# Makefile

TARGETS = \
	x86_64-unknown-linux-gnu \
	x86_64-pc-windows-gnu \
	x86_64-apple-darwin \
	aarch64-apple-darwin

.PHONY: all $(TARGETS)

all: $(TARGETS)

x86_64-unknown-linux-gnu:
	cargo build --release --target x86_64-unknown-linux-gnu

x86_64-pc-windows-gnu:
	cargo build --release --target x86_64-pc-windows-gnu

x86_64-apple-darwin:
	cargo build --release --target x86_64-apple-darwin

aarch64-apple-darwin:
	cargo build --release --target aarch64-apple-darwin

# clean:
# 	cargo clean
