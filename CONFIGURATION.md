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

### `base_dir`

Override artifact base directory.

Examples:

```toml
[petal]
base_dir = "debian" # Package targets Debian
```

```toml
[petal]
base_dir = "debian-12" # Package targets Debian 12
```

```toml
[petal]
base_dir = "ubuntu" # Package targest Ubuntu
```

```toml
[petal]
base_dir = "popos" # Package targets Pop!_OS
```

Some package managers, such as RPM, may also encourage marking the artifact filename with product distribution variation identifiers. See the `dist` option.

### `dist`

Specify a particular product variant in artifact filenames.

Examples:

```toml
petal.dist = "linux" # Tarball targets Linux
```

```toml
petal.dist = "gnu" # Tarball targets glibc
```

```toml
petal.dist = "musl" # Tarball targets musl
```

Some package managers, such as DEB, may lack support for inserting product distribution variant identifers in package filenames. See the `base_dir` option.

### `filename`

Override artifact filename.

Example:

```toml
[petal]
filename = "raygun1.zip"
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

## `base_dir`

Override artifact base directory.

Examples:

```toml
petal.base_dir = "debian" # Package targets Debian
```

```toml
petal.base_dir = "debian-12" # Package targets Debian 12
```

```toml
petal.base_dir = "ubuntu" # Package targest Ubuntu
```

```toml
petal.base_dir = "popos" # Package targets Pop!_OS
```

Some package managers, such as RPM, may also encourage marking the artifact filename with product distribution variation identifiers. See the `dist` option.

## `dist`

Specify a particular product variant in artifact filenames.

Examples:

```toml
petal.dist = "fc40" # Package targets Fedora Core 40
```

```toml
petal.dist = "rhel10" # Package targets RHEL 10
```

```toml
petal.dist = "win11" # Package targets Windows 11
```

```toml
petal.dist = "reactos04" # Package targets ReactOS 0.4
```

```toml
petal.dist = "bash4" # Package targets GNU bash 4+
```

```toml
petal.dist = "linux" # Tarball targets Linux
```

```toml
petal.dist = "gnu" # Tarball targets glibc
```

```toml
petal.dist = "musl" # Tarball targets musl
```

Some package managers, such as DEB, may lack support for inserting product distribution variant identifers in package filenames. See the `base_dir` option.

## `filename`

Override artifact filename.

Example:

```toml
petal.filename = "raygun1.zip"
```

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

| Petal               | ISA                   | Chipset                          |
| ------------------- | --------------------- | -------------------------------- |
| AppImage            | 64-bit ARM            | `aarch64`                        |
| AppImage            | 64-bit Intel          | `x86_64`                         |
| Deb                 | 64-bit ARM            | `arm64`                          |
| Deb                 | 64-bit Intel          | `amd64`                          |
| Deb                 | 64-bit Loongson       | `loong64`                        |
| Deb                 | 64-bit PowerPC (BE)   | `ppc64`                          |
| Deb                 | 64-bit PowerPC LE     | `ppc64el`                        |
| Deb                 | 64-bit RISC_V         | `riscv64`                        |
| Deb                 | (chipset independent) | `all`                            |
| FreeBSD             | 64-bit ARM            | `freebsd:<os-version>:aarch64`   |
| FreeBSD             | 64-bit Intel          | `freebsd:<os-version>:amd64`     |
| FreeBSD             | 64-bit PowerPC (BE)   | `freebsd:<os-version>:powerpc64` |
| FreeBSD             | 64-bit PowerPC LE     | `freebsd:<os-version>:ppc64le`   |
| FreeBSD             | 64-bit RISC-V         | `freebsd:<os-version>:riscv64`   |
| FreeBSD             | (chipset independent) | `freebsd:<os-version>:*`         |
| macOS               | 64-bit ARM            | `arm64`                          |
| macOS               | 64-bit Intel          | `x86_64`                         |
| macOS               | (chipset independent) | `universal`                      |
| Rpm                 | 16-bit Intel          | `8086`, et. al.                  |
| Rpm                 | 32-bit Intel          | `i686`, et. al.                  |
| Rpm                 | 64-bit ARM            | `aarch64`                        |
| Rpm                 | 64-bit Intel          | `x86_64`                         |
| Rpm                 | 64-bit Loongson       | `loongarch64`                    |
| Rpm                 | 64-bit PowerPC LE     | `ppc64le`                        |
| Rpm                 | 64-bit RISC-V         | `riscv64`                        |
| Rpm                 | (chipset indepdendet) | `noarch`                         |
| Tarball (UNIX)      | 64-bit ARM            | `aarch64`, `arm64`, etc.         |
| Tarball (UNIX)      | 64-bit Intel          | `x86_64`, `amd64`, etc.          |
| Tarball (UNIX)      | 64-bit Loongson       | `loongarch64`                    |
| Tarball (UNIX)      | 64-bit PowerPC (BE)   | `ppc64`                          |
| Tarball (UNIX)      | 64-bit PowerPC LE     | `ppc64le`                        |
| Tarball (UNIX)      | 64-bit RISC-V         | `riscv64`                        |
| Tarball (UNIX)      | (chipset independent) | `noarch`, etc.                   |
| Windows (MSI)       | 64-bit ARM            | `Arm64`                          |
| Windows (MSI)       | 64-bit Intel          | `x64`                            |
| Zipball (DOS)       | 16-bit Intel          | `8086`, et. al.                  |
| Zipball (DOS)       | 32-bit Intel          | `80686`, et. al.                 |
| Zipball (DOS)       | (chipset independent) | `all`                            |
| Zipball (OS/2 Warp) | 16-bit Intel          | `8086`, et. al.                  |
| Zipball (OS/2 Warp) | 32-bit Intel          | `i686`, et. al.                  |
| Zipball (OS/2 Warp) | (chipset independent) | `noarch`                         |

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
