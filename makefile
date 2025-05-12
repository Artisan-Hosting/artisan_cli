# Makefile

TARGETS = \
	x86_64-unknown-linux-gnu \
	x86_64-pc-windows-gnu

.PHONY: all $(TARGETS)

all: $(TARGETS)

x86_64-unknown-linux-gnu:
	cargo build --release --target x86_64-unknown-linux-gnu

x86_64-pc-windows-gnu:
	cargo build --release --target x86_64-pc-windows-gnu

# clean:
# 	cargo clean
