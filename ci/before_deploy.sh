#!/bin/sh

# This script takes care of building your crate and packaging it for release

set -ex

main() {
    test -f Cargo.lock || cargo generate-lockfile

    cross rustc --bin "$CRATE_NAME" --target "$TARGET" --release -- -C lto

    case $TARGET in
        x86_64-apple-darwin)
            cp "target/$TARGET/release/$CRATE_NAME" "$CRATE_NAME-macos"
        ;;
        x86_64-unknown-linux-gnu)
            cp "target/$TARGET/release/$CRATE_NAME" "$CRATE_NAME-linux"
        ;;
        x86_64-pc-windows-gnu)
            cp "target/$TARGET/release/$CRATE_NAME.exe" "$CRATE_NAME-windows.exe"
        ;;
        *)
            echo "error: Unknown target $TARGET"
            exit 1
        ;;
    esac
}

main
