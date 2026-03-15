# Package Registry Setup Guide

`{{bin}}` version `{{version}}`

[IF cloudsmith.enable]
This project automatically publishes packages to [Cloudsmith](https://cloudsmith.io/~{{repo_path}}/). 
To easily install `{{bin}}` and receive future updates naturally through your system's package manager, run the relevant setup script for your environment.

## Linux Distributions

### Debian & Ubuntu (APT)
To configure the APT repository and install the package:
```bash
curl -1sLf 'https://dl.cloudsmith.io/public/{{repo_path}}/setup.deb.sh' | sudo -E bash
sudo apt install {{bin}}
```

### RHEL, CentOS & Fedora (RPM)
To configure the YUM/DNF repository and install the package:
```bash
curl -1sLf 'https://dl.cloudsmith.io/public/{{repo_path}}/setup.rpm.sh' | sudo -E bash
sudo dnf install {{bin}}
```

### Alpine Linux (APK)
To configure the APK repository and install the package:
```bash
curl -1sLf 'https://dl.cloudsmith.io/public/{{repo_path}}/setup.alpine.sh' | sudo -E bash
apk add {{bin}}
```
[/IF]

[IF archlinux.enable]
### Arch Linux (PKGBUILD)
You can build and install the package using the provided `PKGBUILD` artifact from GitHub Releases.
```bash
curl -LO {{repository}}/releases/download/v{{version}}/{{bin}}-{{version}}-archlinux-pkgbuild.tar.gz
tar -xzf {{bin}}-{{version}}-archlinux-pkgbuild.tar.gz
makepkg -si
```
[/IF]

[IF brew.enable]
## macOS & Linux (Homebrew)
You can install the package using our custom Homebrew tap:
```bash
brew tap {{brew_tap}}
brew install {{bin}}
```
[/IF]

[IF nuget.enable]
## Windows (NuGet)
To install the package via NuGet in PowerShell, register the Cloudsmith feed and install it:
```powershell
Register-PackageSource -Name '{{repo_path}}' -ProviderName NuGet -Location "https://nuget.cloudsmith.io/{{repo_path}}/v3/index.json"
Install-Package {{bin}} -Source '{{repo_path}}'
```

Chocolatey:
```powershell
choco source add -n {{repo_path}} -s https://nuget.cloudsmith.io/{{repo_path}}/v3/index.json
choco install {{bin}} -s {{repo_path}}
```

PowerShell:
```powershell
Register-PackageSource -Name '{{repo_path}}' -ProviderName NuGet -Location "https://nuget.cloudsmith.io/{{repo_path}}/v2/" -Trusted
Register-PSRepository -Name '{{repo_path}}' -SourceLocation "https://nuget.cloudsmith.io/{{repo_path}}/v2/" -InstallationPolicy 'trusted'

Install-Package {{bin}} -Source '{{repo_path}}'
# Or
Install-Module {{bin}} -Repository '{{repo_path}}'
```
[/IF]

[IF scoop.enable]
## Windows (Scoop)
You can install the package using our custom Scoop bucket:
```powershell
scoop bucket add {{scoop_bucket_name}} https://github.com/{{scoop_bucket}}
scoop install {{scoop_bucket_name}}/{{bin}}
```
[/IF]

[IF crate.crates_io]
## Rust (Cargo)
You can install the package directly from `crates.io`:
```bash
cargo install {{bin}}
```
[/IF]

[IF crate.cloudsmith]
## Rust (Cargo - Cloudsmith)
To install from the Cloudsmith registry:
```bash
# Add the registry to your Cargo configuration
cat <<EOF >> ~/.cargo/config.toml
[registries.cloudsmith]
index = "sparse+https://cargo.cloudsmith.io/{{repo_path}}/"
EOF

cargo install {{bin}} --registry cloudsmith
```
[/IF]

[IF docker.enable]
## Docker

[IF ghcr_and_cloudsmith]
Multi-architecture Docker images are available. You can pull the images from GitHub Container Registry (GHCR) or Cloudsmith.
[/IF]
[IF ghcr_only]
Multi-architecture Docker images are available on GitHub Container Registry (GHCR).
[/IF]
[IF cloudsmith_only]
Multi-architecture Docker images are available on Cloudsmith registry.
[/IF]

### Alpine (Default)
Minimal size image based on Alpine Linux.
```bash
[IF ghcr.enable]
docker pull ghcr.io/{{github_org}}/{{bin}}:{{version}}
# OR
docker pull ghcr.io/{{github_org}}/{{bin}}:{{version}}-alpine
[/IF]
[IF ghcr_and_cloudsmith]
# OR (Cloudsmith)
[/IF]
[IF docker.cloudsmith.enable]
docker pull docker.cloudsmith.io/{{repo_path}}/{{bin}}:{{version}}
# OR
docker pull docker.cloudsmith.io/{{repo_path}}/{{bin}}:{{version}}-alpine
[/IF]
```

### Debian Slim
Compatible image based on Debian Bookworm Slim.
```bash
[IF ghcr.enable]
docker pull ghcr.io/{{github_org}}/{{bin}}:{{version}}-bookworm
[/IF]
[IF ghcr_and_cloudsmith]
# OR
[/IF]
[IF docker.cloudsmith.enable]
docker pull docker.cloudsmith.io/{{repo_path}}/{{bin}}:{{version}}-bookworm
[/IF]
```

### Dockerfile
To refer image after pulling, use this in your `Dockerfile`:
```dockerfile
# Alpine
[IF ghcr.enable]
FROM ghcr.io/{{github_org}}/{{bin}}:{{version}}
[/IF]
[IF cloudsmith_only]
FROM docker.cloudsmith.io/{{repo_path}}/{{bin}}:{{version}}
[/IF]

# Debian Slim
[IF ghcr.enable]
FROM ghcr.io/{{github_org}}/{{bin}}:{{version}}-bookworm
[/IF]
[IF cloudsmith_only]
FROM docker.cloudsmith.io/{{repo_path}}/{{bin}}:{{version}}-bookworm
[/IF]
```
[/IF]
