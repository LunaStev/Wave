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

package-windows:
	zip wave-$(VERSION)-windows.zip $(TARGET_DIR)/$(WINDOWS_TARGET)/release/wave.exe

run:
	cargo run

clean:
	rm -rf ./target
