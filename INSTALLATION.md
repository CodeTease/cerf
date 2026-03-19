# Package Registry Setup Guide

`cerf` version `0.2.0`

This project automatically publishes packages to [Cloudsmith](https://cloudsmith.io/~codetease/tools/). 
To easily install `cerf` and receive future updates naturally through your system's package manager, run the relevant setup script for your environment.

## Linux Distributions

### Debian & Ubuntu (APT)
To configure the APT repository and install the package:
```bash
curl -1sLf 'https://dl.cloudsmith.io/public/codetease/tools/setup.deb.sh' | sudo -E bash
sudo apt install cerf
```

### RHEL, CentOS & Fedora (RPM)
To configure the YUM/DNF repository and install the package:
```bash
curl -1sLf 'https://dl.cloudsmith.io/public/codetease/tools/setup.rpm.sh' | sudo -E bash
sudo dnf install cerf
```

### Alpine Linux (APK)
To configure the APK repository and install the package:
```bash
curl -1sLf 'https://dl.cloudsmith.io/public/codetease/tools/setup.alpine.sh' | sudo -E bash
apk add cerf
```

### Arch Linux (PKGBUILD)
You can build and install the package using the provided `PKGBUILD` artifact from GitHub Releases.
```bash
curl -LO https://github.com/cerfsh/cerf/releases/download/v0.2.0/cerf-0.2.0-archlinux-pkgbuild.tar.gz
tar -xzf cerf-0.2.0-archlinux-pkgbuild.tar.gz
makepkg -si
```

## macOS & Linux (Homebrew)
You can install the package using our custom Homebrew tap:
```bash
brew tap cerfsh/homebrew-tap
brew install cerf
```

## Windows (NuGet)
To install the package via NuGet in PowerShell, register the Cloudsmith feed and install it:
```powershell
Register-PackageSource -Name 'codetease/tools' -ProviderName NuGet -Location "https://nuget.cloudsmith.io/codetease/tools/v3/index.json"
Install-Package cerf -Source 'codetease/tools'
```

Chocolatey:
```powershell
choco source add -n codetease/tools -s https://nuget.cloudsmith.io/codetease/tools/v3/index.json
choco install cerf -s codetease/tools
```

PowerShell:
```powershell
Register-PackageSource -Name 'codetease/tools' -ProviderName NuGet -Location "https://nuget.cloudsmith.io/codetease/tools/v2/" -Trusted
Register-PSRepository -Name 'codetease/tools' -SourceLocation "https://nuget.cloudsmith.io/codetease/tools/v2/" -InstallationPolicy 'trusted'

Install-Package cerf -Source 'codetease/tools'
# Or
Install-Module cerf -Repository 'codetease/tools'
```

## Windows (Scoop)
You can install the package using our custom Scoop bucket:
```powershell
scoop bucket add scoop-bucket https://github.com/cerfsh/scoop-bucket
scoop install scoop-bucket/cerf
```


## Rust (Cargo - Cloudsmith)
To install from the Cloudsmith registry:
```bash
# Add the registry to your Cargo configuration
cat <<EOF >> ~/.cargo/config.toml
[registries.cloudsmith]
index = "sparse+https://cargo.cloudsmith.io/codetease/tools/"
EOF

cargo install cerf --registry cloudsmith
```

## Docker

Multi-architecture Docker images are available. You can pull the images from GitHub Container Registry (GHCR) or Cloudsmith.

### Alpine (Default)
Minimal size image based on Alpine Linux.
```bash
docker pull ghcr.io/cerfsh/cerf:0.2.0
# OR
docker pull ghcr.io/cerfsh/cerf:0.2.0-alpine
# OR (Cloudsmith)
docker pull docker.cloudsmith.io/codetease/tools/cerf:0.2.0
# OR
docker pull docker.cloudsmith.io/codetease/tools/cerf:0.2.0-alpine
```

### Debian Slim
Compatible image based on Debian Bookworm Slim.
```bash
docker pull ghcr.io/cerfsh/cerf:0.2.0-bookworm
# OR
docker pull docker.cloudsmith.io/codetease/tools/cerf:0.2.0-bookworm
```

### Dockerfile
To refer image after pulling, use this in your `Dockerfile`:
```dockerfile
# Alpine
FROM ghcr.io/cerfsh/cerf:0.2.0

# Debian Slim
FROM ghcr.io/cerfsh/cerf:0.2.0-bookworm
```