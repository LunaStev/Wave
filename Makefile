windows:
		cargo build --target x86_64-pc-windows-msvc --release

linux:
		cargo build --target x86_64-unknown-linux-gnu --release

darwin:
		cargo build --target x86_64-apple-darwin --release
		cargo build --target aarch64-apple-darwin --release

run:
	cargo run

clean:
	rm -rf ./target
