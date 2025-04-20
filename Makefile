VERSION := $(shell grep '^version' Cargo.toml | sed 's/version = "\(.*\)"/\1/')

TARGET_DIR := ./target

# Linux
x86_LINUX_GNU_TARGET := x86_64-unknown-linux-gnu
x86_LINUX_MUSL_TARGET := x86_64-unknown-linux-musl
AARCH64_LINUX_GNU_TARGET := aarch64-unknown-linux-gnu

# Windows
WINDOWS_TARGET := x86_64-pc-windows-gnu

# MacOS
DARWIN_TARGETS := x86_64-apple-darwin aarch64-apple-darwin

install:
	rustup target add $(x86_LINUX_GNU_TARGET)
	rustup target add $(x86_LINUX_MUSL_TARGET)
	rustup target add $(AARCH64_LINUX_GNU_TARGET)


# Windows
build-windows:
	cargo build --target $(WINDOWS_TARGET) --release

# Linux
build-x86-linux-gnu:
	cargo build --target $(x86_LINUX_GNU_TARGET) --release

check-musl-gcc:
	@which musl-gcc > /dev/null || (echo "❌ musl-gcc not found. Please install musl-tools." && exit 1)

build-x86-linux-musl: check-musl-gcc
	cargo build --target $(x86_LINUX_MUSL_TARGET) --release

build-aarch64-linux-gnu:
	cargo build --target $(AARCH64_LINUX_GNU_TARGET) --release

# Windows
package-windows:
	zip wave-v$(VERSION)-windows.zip $(TARGET_DIR)/$(WINDOWS_TARGET)/release/wave.exe

# Linux
package-x86-linux-gnu:
	tar -czvf wave-v$(VERSION)-x86_64-linux-gnu.tar.gz -C $(TARGET_DIR)/$(x86_LINUX_GNU_TARGET)/release wavec

package-x86-linux-musl:
	tar -czvf wave-v$(VERSION)-x86_64-linux-musl.tar.gz -C $(TARGET_DIR)/$(x86_LINUX_MUSL_TARGET)/release wavec

package-aarch64-linux-gnu:
	tar -czvf wave-v$(VERSION)-aarch64-linux-gnu.tar.gz -C $(TARGET_DIR)/$(AARCH64_LINUX_GNU_TARGET)/release wavec

build-all: \
	build-x86-linux-gnu \
 	# build-aarch64-linux-gnu # build-windows # build-x86-linux-musl

package-all: \
 	package-x86-linux-gnu \
 	# package-aarch64-linux-gnu # package-windows # package-x86-linux-musl

release: build-all package-all

build-darwin:
	$(foreach target, $(DARWIN_TARGETS), cargo build --target $(target) --release;)

package-darwin:
	$(foreach target, $(DARWIN_TARGETS), tar -czvf wave-$(VERSION)-$(target).tar.gz -C $(TARGET_DIR)/$(target)/release wave)

clean:
	rm -rf $(TARGET_DIR)
