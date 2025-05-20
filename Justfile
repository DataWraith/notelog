default: test

test:
    cargo nextest run

release-build version:
    rm -f ./dist/*
    cargo zigbuild --color=always --release --target-dir "./target" --target "x86_64-unknown-linux-musl" --locked
    cp target/x86_64-unknown-linux-musl/release/notelog dist/
    cp LICENSE.md README.md dist/
    tar cavf dist/notelog-{{version}}-x86_64-unknown-linux-musl.tar.gz --remove-files --numeric-owner --no-xattrs --transform='s|dist/|notelog/|' dist/*
