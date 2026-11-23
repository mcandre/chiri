# NATIVE DEPENDENCIES

Some Rust dependencies rely on native C/C++ resources.

## Tools

* [autotools](https://www.gnu.org/software/automake/manual/html_node/Autotools-Introduction.html)
* [clang](https://clang.llvm.org/)

## C/C++ Headers

* [BLAKE2](https://www.blake2.net/)
* [bzip2](https://sourceware.org/bzip2/)

## Debian

```sh
sudo apt-get update
sudo apt-get install -y build-essential clang libacl1-dev libb2-dev libbz2-dev
```

## FreeBSD

```sh
sudo pkg install -y autoconf automake libb2 libtool gmake
```

## macOS (Homebrew)

```sh
xcode-select --install
brew install bzip2 libb2
```

## RHEL

```sh
sudo dnf install -y @development-tools clang libacl1-devel libb2-devel libbz2-devel
```
