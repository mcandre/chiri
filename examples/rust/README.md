# raygun

My Illudium Q-36 space modulator!

# RUNTIME REQUIREMENTS

(None)

# BUILDTIME REQUIREMENTS

* a UNIX-like environment (e.g. [WSL](https://learn.microsoft.com/en-us/windows/wsl/))
* [Docker](https://www.docker.com/) 28.0.1+
* POSIX compliant [make](https://pubs.opengroup.org/onlinepubs/9799919799/utilities/make.html)
* [Rust](https://www.rust-lang.org/en-US/)
* Provision additional dev tools with `make -f install.mk`

# INSTALL

```sh
make install
```

# RUN

```sh
raygun
```

# PUBLISH CRATE

```sh
make publish
```

# CROSSCOMPILE BINARIES

```sh
make crit
```

# PACKAGE BINARIES

```sh
make package
```

# INSTALL

Identify your operating system and chipset.

Apply a corresponding installer in [.chiri/artifacts/](.chiri/artifacts).

# UNINSTALL

The majority of package managers support automated uninstall procedures. Consult your particular package manager's documentation.

Other package managers may use manual uninstall procedures.

## AppImage

Locate your `*.AppImage` file.

```sh
rm <AppImage>
```

## macOS

```sh
sudo rm /usr/local/bin/raygun
sudo pkgutil --forget test.mars.raygun
```
