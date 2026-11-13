# chiri: lightweight software package generator

[![Crates.io Downloads (recent)](https://img.shields.io/crates/dr/chiri?label=crate%20downloads)](https://crates.io/crates/chiri) [![docs.rs](https://img.shields.io/docsrs/chiri)](https://docs.rs/chiri/latest/chiri/) [![license](https://img.shields.io/badge/license-BSD-0)](LICENSE.md) [![Donate](https://img.shields.io/badge/%E2%99%A5-Sponsor-BF3988)](https://github.com/sponsors/mcandre)

(Work in Progress)

花ちりて木間の寺と成にけり

_Flowers scattered, a wreathed temple arose._

—Yosa Buson

# ABOUT

chiri builds install media for a wide variety of platforms.

Through chiri, we encourage more developers to publish crossplatform applications.

May your userbase flourish.

# EXAMPLES

```console
% cd examples/go

% chiri

% tree .chiri/artifacts
...
```

chiri is programming language agnostic. For more programming language samples, see [examples](examples).

# DOWNLOAD

```sh
cargo install chiri
```

## Prerequisites

* [cargo](https://doc.rust-lang.org/cargo/)

## Postinstall

Register `~/.cargo/bin` to `PATH` environment variable.

For details on tuning chiri, see [CONFIGURATION](CONFIGURATION.md).

For details on building from source, see [DEVELOPMENT](DEVELOPMENT.md).

# Package Managers

* `alpine`
* `appimage`
* `deb`
* `freebsd`
* `gentoo`
* `macos`
* `netbsd`
* `openbsd`
* `rpm`
* `tarball`
* `windows`

# TIPS

Fully static [Go](https://go.dev/)/[musl](https://musl.libc.org/) binaries tend to improve portability for compiled applications.

# RESOURCES

Prior art, personal plugs, and tools for developing portable applications (including non-Rust projects)!

* [alien](https://joeyh.name/code/alien/) - an OS package converter
* [apple-platform-rs](https://github.com/indygreg/apple-platform-rs), [bomutils](https://github.com/hogliux/bomutils), [stuckliste](https://github.com/igankevich/stuckliste) - Apple Bom format research
* [cargo](https://doc.rust-lang.org/cargo/) - a crossplatform package manager for Rust projects
* [Docker](https://www.docker.com/) - a package manager for containers
* [fpm](https://github.com/jordansissel/fpm) - an older, host native package generator
* [go mod](https://go.dev/doc/modules/gomod-ref) - a crossplatform package manager for Go applications
* [goreleaser](https://goreleaser.com/) - a crossplatform binary & package generator
* [mcandre/crit](https://github.com/mcandre/crit) - an automated Rust multiplatform compiler
* [mcandre/rockhopper](https://github.com/mcandre/rockhopper) - a Docker based package generator
* [mcandre/tuco](https://github.com/mcandre/tuco) - an automated Go multiplatform compiler
* [msitools](https://gitlab.gnome.org/GNOME/msitools) - a Linux framework for generating Windows MSI packages
* [tree](https://en.wikipedia.org/wiki/Tree_(command)) - recursive directory browser
* [UTM](https://mac.getutm.app/) - a macOS emulator with support for NetBSD and other operating systems
* [xgo](https://github.com/techknowlogick/xgo) - an automated cGo multiplatform compiler

🌸
