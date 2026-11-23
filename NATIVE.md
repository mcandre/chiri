# NATIVE DEPENDENCIES

## Debian

```sh
sudo apt update
sudo apt install -y build-essential clang libacl1-dev libb2-dev libbz2-dev
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
