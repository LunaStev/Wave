windows:
		cargo build --target x86_64-pc-windows-msvc --release

package-linux:
	tar -czvf wave-$(VERSION)-linux.tar.gz -C $(TARGET_DIR)/$(LINUX_TARGET)/release wave

darwin:
		cargo build --target x86_64-apple-darwin --release
		cargo build --target aarch64-apple-darwin --release

run:
	cargo run

clean:
	rm -rf ./target
