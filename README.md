# chiri: crossplatform package generator

[![Crate Downloads](https://badgen.net/crates/d/chiri?label=crate+downloads&icon=rust)](https://crates.io/crates/chiri) [![API Docs](https://badgen.net/static/docs/API/3eca2a?icon=docs.rs)](https://docs.rs/chiri/latest/chiri/) [![CI/CD](https://badgen.net/github/checks/mcandre/chiri?icon=github)](https://github.com/mcandre/chiri/actions) [![License](https://badgen.net/static/license/BSD/ab2b29?icon=freebsd)](LICENSE.md) [![AI](https://badgen.net/static/ai/Gemini|hint?color=886fbf&icon=google-gemini)](AI-DECLARATION.md) [![Donate](https://badgen.net/static/sponsor/%E2%99%A5?color=bf3988&icon=github)](https://github.com/sponsors/mcandre)

![sakura bloom](https://raw.githubusercontent.com/mcandre/chiri/main/chiri.webp)

# ABOUT

chiri builds installers for a wide variety of platforms.

Through chiri, we encourage more developers to publish crossplatform applications.

Scatter packages like so many leaves on the wind.

# EXAMPLES

```console
% cd examples/go

% chiri

% tree .chiri/artifacts
.chiri/artifacts
├── appimage
│   ├── raygun-0.0.1-1-aarch64.AppImage
│   └── raygun-0.0.1-1-x86_64.AppImage
├── deb
│   ├── raygun_0.0.1-1_amd64.deb
│   └── raygun_0.0.1-1_arm64.deb
...
```

chiri is programming language agnostic. For more programming language samples, see [examples](examples).

# DOWNLOAD

```sh
cargo install chiri
```

## Prerequisites

* [cargo](https://doc.rust-lang.org/cargo/)

For details on provisioning C/C++ components, see [NATIVE](NATIVE.md).

## Postinstall

Register `~/.cargo/bin` to `PATH` environment variable.

For details on tuning chiri, see [CONFIGURATION](CONFIGURATION.md).

For details on building from source, see [DEVELOPMENT](DEVELOPMENT.md).

# Package Managers

[Petals](https://docs.rs/chiri/latest/chiri/petal/enum.Petal.html)

* AppImage
* DEB
* FreeBSD
* macOS
* RPM
* Tarball

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
* [mcandre/tuco](https://github.com/mcandre/tuco) - an automated Go multiplatform compiler
* [msitools](https://gitlab.gnome.org/GNOME/msitools) - a Linux framework for generating Windows MSI packages
* [tree](https://en.wikipedia.org/wiki/Tree_(command)) - recursive directory browser
* [UTM](https://mac.getutm.app/) - a macOS emulator with support for NetBSD and other operating systems
* [xgo](https://github.com/techknowlogick/xgo) - an automated cGo multiplatform compiler

🌸
