# Installation

## Precompiled binaries

Each time a new version of tsu is released, precompiled binaries are created for Linux, macOS, and
Windows. Instead of building from source, you may download these precompiled binaries and run tsu
straight out of the box. All release artifacts can be found on [GitHub](https://github.com/crhowell3/tsu/releases/latest).

### Linux

```shell
curl -L https://github.com/crhowell3/tsu/releases/download/v0.1.0/tsu-x86_64-unknown-linux-gnu.tar.gz | tar xz
chmod +x tsu
sudo mv tsu /usr/local/bin/
```

### macOS

```shell
curl -L https://github.com/crhowell3/tsu/releases/download/v0.1.0/tsu-x86_64-apple-darwin.tar.gz | tar xz
chmod +x tsu
sudo mv tsu /usr/local/bin/
```

### Windows

Download `tsu-x86_64-pc-windows-msvc.zip` and extract to a directory in your PATH.

## Build from source

Clone the tsu GitHub repository and build with `cargo`.

Requirements:

- [Rust toolchain](https://www.rust-lang.org/tools/install)
- [Git version control](https://git-scm.com/)

```shell
# Clone the repo
git clone https://github.com/crhowell3/tsu.git

cd tsu

# Run tests
cargo test -q

# Install
cargo install --path .
```
