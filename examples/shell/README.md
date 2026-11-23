# raygun

My Illudium Q-36 space modulator!

# RUN

```sh
raygun
```

# PACKAGE

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
