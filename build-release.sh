#!/bin/sh

# Generate zip file(s) to be uploaded to Github release page

BIN_NAME=tere

rm -rf release
mkdir release

# from https://github.com/Canop/broot/blob/master/version.sh
version=$(sed 's/^version = "\([^\"]*\)"/\1/;t;d' Cargo.toml | head -1)

echo "Building $BIN_NAME version $version"

#cargo clean
for target in \
	"x86_64-unknown-linux-gnu" \
	"x86_64-unknown-linux-musl" \
	"x86_64-pc-windows-gnu" #\
	# not tested yet (TODO: test)
	#"aarch64-unknown-linux-gnu" \
	#"x86_64-apple-darwin"
do
echo "Building $target"
cargo build --target=$target --release

cd target/$target/release
zip_name=$BIN_NAME-$version-$target.zip
zip $zip_name $BIN_NAME
cd -
mv -v target/$target/release/$zip_name release/
done
