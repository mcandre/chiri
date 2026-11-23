# CONFIGURATION

# FLAGS

## `-d`, `--debug`

Enables debug mode. Show additional logs.

In the event a problem, debug mode provides context for common resources, such as package configurations.

## `-h`, `--help`

Show usage menu.

## `-v`, `--version`

Show version banner.

# TOML

chiri looks for a configuration file `chiri.toml` in the current working directory.

Format: [Chiri](https://docs.rs/chiri/latest/chiri/struct.Chiri.html).

## `debug`

Default: `false`

Enables debug mode. Show additional logs.

Example:

```toml
debug = "true"
```

## `petal`

Required fields vary by distribution.

Package attribute table.

Format: Table of string to string.

Example:

```toml
[petal]
name = "raygun"
version = "0.0.1"
```

### `name`

Nonblank.

Commonly required across package managers.

Example:

```toml
[petal]
name = "raygun"
```

### `version`

Commonly required across package managers.

Example:

```toml
[petal]
name = "raygun"
version = "0.0.1"
```

Recommend [SemVer](https://semver.org/).

### `rev`

Often required by package managers.

Release increment, relative to an upstream `version`.

Example:

```toml
[petal]
name = "raygun"
version = "0.0.1"
rev = "1"
```

### `maintainer`

Often required by package managers.

Contact information for developer managing packages.

Format: RFC822.

Example:

```toml
[petal]
name = "raygun"
maintainer = "Marvin the Martian <marvin@mars.test>"
```

### `summary`

Often required by package managers.

A concise description of the package.

Example:

```toml
[petal]
name = "raygun"
summary = "Space modulator"
```

### `description`

Often required by package managers.

A brief description of the package.

Example:

```toml
[petal]
name = "raygun"
description = "Illudium Q-36 explosive space modulator"
```

### `copyright`

Generally recommended.

A Copyright identifier.

Example:

```toml
[petal]
name = "raygun"
copyright = "Copyright (C) 1948 Marvin the Martian"
```

### `license`

Often required by package managers.

A concise software license identifier.

Format: [SPDX](https://spdx.org/licenses/).

Example:

```toml
[petal]
name = "raygun"
license = "0BSD"
```

Operating systems often have separate conventions to manage longer license documentation.

### `url`

Often required by package managers.

Software homepage.

Example:

```toml
[petal]
url = "https://mars.test/"
```

### Other distro relative fields

A distro may have additional optional and/or required fields.

## `dest`

Optional.

Map of destination file path to source media file path.

Default: (empty)

Format: Table of target paths to source paths.

Example:

```toml
[dest]
"usr/bin" = "bin"
"etc/nginx/nginx.conf" = "nginx.conf"
# ...
```

## `pkg`

An array of package specifications.

Default: (empty)

Example:

```toml
[[pkg]]
kind = "appimage"

[[pkg]]
kind = "deb"

[[pkg]]
kind = "rpm"

# ...
```

### `kind`

Required, nonblank.

Package type.

Format: [Petal](https://docs.rs/chiri/latest/chiri/petal/enum.Petal.html) variant (lowercase).

Example:

```toml
kind = "tarball"
```

### `petal`

Default: (empty)

Per-package attribute table.

Overrides the global petal attribute table.

Example:

```toml
[[pkg]]
kind = "deb"
petal.chipset = "amd64"
petal.dependencies = "bash"
# ...
```

#### `chipset`

Nonblank.

OS target architecture identifier. Syntax relative to package manager.

Example:

```toml
[[pkg]]
kind = "deb"
petal.chipset = "amd64"
```

##### Common OS Architectures

| Petal          | ISA                   | Chipset                            |
| -------------- | --------------------- | ---------------------------------- |
| AppImage       | 64-bit ARM            | `aarch64`                          |
| AppImage       | 64-bit Intel          | `x86_64`                           |
| Deb            | 64-bit ARM            | `arm64`                            |
| Deb            | 64-bit Intel          | `amd64`                            |
| Deb            | (chipset independent) | `all`                              |
| FreeBSD        | 64-bit ARM            | `freebsd:<os-version>:aarch64`     |
| FreeBSD        | 64-bit Intel          | `freebsd:<os-version>:amd64`       |
| FreeBSD        | (chipset independent) | `freebsd:<os-version>:*`           |
| macOS          | 64-bit ARM            | `arm64`                            |
| macOS          | 64-bit Intel          | `x86_64`                           |
| macOS          | (chipset independent) | `universal`                        |
| Rpm            | 64-bit ARM            | `aarch64`                          |
| Rpm            | 64-bit Intel          | `x86_64`                           |
| Rpm            | (chipset indepdendet) | `noarch`                           |
| Tarball (UNIX) | 64-bit ARM            | `aarch64`, `arm64`, etc.           |
| Tarball (UNIX) | 64-bit Intel          | `x86_64`, `amd64`, etc.            |
| Tarball (UNIX) | (chipset independent) | `all`, `noarch`, `universal`, etc. |

Note: Some platforms / package managers / package manager versions may lack support for noarch-style semantics.

Unfortunately, there is no universal naming scheme for ISA's across Linux distros, nor BSD distros, nor programming languages, nor `uname -m` vs `dpkg-architecture -qDEB_HOST_ARCH`, let alone the wider *nix spectrum, let alone COMSPEC Windows. Let alone case sensitivity. Let alone 32-bit variants.

Robust package managers and compilers validate architecture names against a distribution-relative list when generating packages. Consult relevant software documentation and community forums in order to confirm the exact enumeration of permitted strings.

Freeform resources, such as tarballs, may follow a popular naming convention system, such as Debian or RHEL.

Warning: ISA naming convention systems often change at different places within the software project, from build configuration tags, to crosscompilation toolchains, to containers, to package managers. Verify the relevant ISA naming convention at each separate place in your codebases.

#### `dependencies`

Default: (empty)

Commonly required for applications large and small.

Syntax often varies by distro.

Example:

```toml
[[pkg]]
kind = "deb"
petal.dependencies = "bash"
```

### `dest`

Optional.

Override global petal dest file path map.

Default: (empty)

Example:

```toml
[[pkg]]
kind = "deb"
petal.chipset = "amd64"
dest."usr/bin" = "bin/x86_64-unknown-linux-musl"
```

Recommended for binary applications.
