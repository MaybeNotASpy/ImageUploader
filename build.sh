RUSTFLAGS='-C link-arg=-s' cargo build --release --target x86_64-unknown-linux-musl
cp target/x86_64-unknown-linux-musl/release/image_uploader ./image_uploader