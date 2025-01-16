VERSION := $(shell grep '^version' Cargo.toml | sed 's/version = "\(.*\)"/\1/')

TARGET_DIR := ./target
LINUX_TARGET := x86_64-unknown-linux-gnu
WINDOWS_TARGET := x86_64-pc-windows-gun
DARWIN_TARGETS := x86_64-apple-darwin aarch64-apple-darwin

build-windows:
	cargo build --target $(WINDOWS_TARGET) --release

build-linux:
	cargo build --target $(LINUX_TARGET) --release

package-linux:
	tar -czvf wave-$(VERSION)-linux.tar.gz -C $(TARGET_DIR)/$(LINUX_TARGET)/release wave

darwin:
		cargo build --target x86_64-apple-darwin --release
		cargo build --target aarch64-apple-darwin --release

run:
	cargo run

clean:
	rm -rf ./target
