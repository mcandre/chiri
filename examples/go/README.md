# raygun

My Illudium Q-36 space modulator!

# RUNTIME REQUIREMENTS

(None)

# BUILDTIME REQUIREMENTS

* [bash](https://www.gnu.org/software/bash/)
* [Go](https://go.dev/)

# RUN

```sh
raygun
```

# CROSSCOMPILE BINARIES

```sh
./build
```

# PACKAGE BINARIES

```sh
chiri
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
