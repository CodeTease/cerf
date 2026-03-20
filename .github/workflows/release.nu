#!/usr/bin/env nu

# Description:
# A generalized script for cross-compiling and packaging Rust projects.
# Originally from NuShell repo, but this script optimized for
# general-purpose and more features.

def hr-line [--blank_line(-b)] {
    print $"(ansi g)---------------------------------------------------------------------------->(ansi reset)"
    if $blank_line { char nl }
}

def format_template [
    template_path: string
    context: record
] {
    let eval_condition = {|cond: string|
        if ($cond | str starts-with "!") {
            let c = ($cond | str substring 1..)
            let val = (try { $context | get $c } catch { false })
            not $val
        } else {
            (try { $context | get $cond } catch { false })
        }
    }

    mut filtered_lines = []
    mut skip_stack = []

    for line in (open --raw $template_path | lines) {
        let start_match = ($line | parse -r '^\s*\[IF\s+(?<condition>[a-zA-Z0-9._!-]+)\]\s*$')
        if ($start_match | is-not-empty) {
            let cond = $start_match.0.condition
            let is_cond_true = (do $eval_condition $cond)
            let parent_skip = if ($skip_stack | is-empty) { false } else { $skip_stack | last }
            $skip_stack = ($skip_stack | append ($parent_skip or not $is_cond_true))
            continue
        }
        
        let end_match = ($line | parse -r '^\s*\[/IF\]\s*$')
        if ($end_match | is-not-empty) {
            if not ($skip_stack | is-empty) {
                $skip_stack = ($skip_stack | drop 1)
            }
            continue
        }
        
        let skip_line = if ($skip_stack | is-empty) { false } else { $skip_stack | last }
        
        if not $skip_line {
            $filtered_lines = ($filtered_lines | append $line)
        }
    }

    $filtered_lines | str join (char nl)
}

def main [command?: string] {
    let cmd = ($command | default "build")
    match $cmd {
        "build" => { run_build }
        "publish" => { run_publish }
        _ => { print $"::error::Unknown command: ($cmd)"; exit 1 }
    }
}

def run_build [] {
    let config_file = "release.toml"
    if not ($config_file | path exists) {
        print $"::error::($config_file) not found."
        exit 1
    }
    let config = (open $config_file)

    let bin     = (try { $config.metadata.bin } catch { "" })
    let version = (try { $config.metadata.version } catch { "" })

    if ($bin | is-empty) or ($version | is-empty) {
        print "::error::'metadata.bin' or 'metadata.version' is missing or empty in release.toml"
        exit 1
    }

    let os      = $env.OS
    let target  = $env.TARGET

    # Target Early Exit
    let is_target_enabled = (try { $config.targets | get $target } catch { false })
    if $is_target_enabled != true {
        print $"Target ($target) is not enabled in release.toml. Skipping build."
        exit 0
    }

    let src     = $env.GITHUB_WORKSPACE
    let dist    = $"($env.GITHUB_WORKSPACE)/output"

    print "Debugging info:"
    print { project: $bin, version: $version, os: $os, target: $target, src: $src, dist: $dist }
    hr-line -b

    let USE_UBUNTU = ($os | str starts-with "ubuntu")

    print $"(char nl)Packaging ($bin) v($version) for ($target)..."
    hr-line -b

    if not ('Cargo.lock' | path exists) {
        cargo generate-lockfile
    }

    let cargo_build_project = {
        print $"Building ($bin) for ($target)..."
        cargo build --release --all --target $target
    }

    # --- Build Environment Setup ---
    if $os in ['macos-latest'] or $USE_UBUNTU {
        if $USE_UBUNTU {
            sudo apt update
            # Generic dependencies for compilation
        }

        match $target {
            'aarch64-unknown-linux-gnu' => {
                sudo apt-get install gcc-aarch64-linux-gnu -y
                $env.CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER = 'aarch64-linux-gnu-gcc'
                do $cargo_build_project
            }

            'riscv64gc-unknown-linux-gnu' => {
                sudo apt-get install gcc-riscv64-linux-gnu -y
                $env.CARGO_TARGET_RISCV64GC_UNKNOWN_LINUX_GNU_LINKER = 'riscv64-linux-gnu-gcc'
                do $cargo_build_project
            }

            's390x-unknown-linux-gnu' => {
                sudo apt-get install gcc-s390x-linux-gnu -y
                $env.CARGO_TARGET_S390X_UNKNOWN_LINUX_GNU_LINKER = 's390x-linux-gnu-gcc'
                do $cargo_build_project
            }

            'powerpc64le-unknown-linux-gnu' => {
                sudo apt-get install gcc-powerpc64le-linux-gnu -y
                $env.CARGO_TARGET_POWERPC64LE_UNKNOWN_LINUX_GNU_LINKER = 'powerpc64le-linux-gnu-gcc'
                do $cargo_build_project
            }

            'aarch64-unknown-linux-musl' => {
                aria2c https://github.com/nushell/integrations/releases/download/build-tools/aarch64-linux-musl-cross.tgz
                tar -xf aarch64-linux-musl-cross.tgz -C $env.HOME
                $env.PATH = ($env.PATH | split row (char esep) | prepend $"($env.HOME)/aarch64-linux-musl-cross/bin")
                $env.CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER = 'aarch64-linux-musl-gcc'
                do $cargo_build_project
            }

            'loongarch64-unknown-linux-gnu' => {
                aria2c https://github.com/loongson/build-tools/releases/download/2025.08.08/x86_64-cross-tools-loongarch64-binutils_2.45-gcc_15.1.0-glibc_2.42.tar.xz
                tar xf x86_64-cross-tools-loongarch64-*.tar.xz
                $env.PATH = ($env.PATH | split row (char esep) | prepend $"($env.PWD)/cross-tools/bin")
                $env.CARGO_TARGET_LOONGARCH64_UNKNOWN_LINUX_GNU_LINKER = 'loongarch64-unknown-linux-gnu-gcc'
                $env.RUSTFLAGS = "-C target-feature=+crt-static"
                do $cargo_build_project
            }

            'loongarch64-unknown-linux-musl' => {
                aria2c https://github.com/LoongsonLab/oscomp-toolchains-for-oskernel/releases/download/loongarch64-linux-musl-cross-novec/loongarch64-linux-musl-cross-novec.tgz
                tar -xf loongarch64-linux-musl-cross-novec.tgz
                $env.PATH = ($env.PATH | split row (char esep) | prepend $'($env.PWD)/loongarch64-linux-musl-cross-novec/bin')
                $env.CARGO_TARGET_LOONGARCH64_UNKNOWN_LINUX_MUSL_LINKER = "loongarch64-linux-musl-gcc"
                # Workaround for Rust 1.87 TLS issues: abort strategy to bypass TLS-dependent panic handling
                $env.RUSTFLAGS = "-C panic=abort -C target-feature=+crt-static"
                do $cargo_build_project
            }

            'armv7-unknown-linux-gnueabihf' => {
                sudo apt-get install pkg-config gcc-arm-linux-gnueabihf -y
                $env.CARGO_TARGET_ARMV7_UNKNOWN_LINUX_GNUEABIHF_LINKER = 'arm-linux-gnueabihf-gcc'
                do $cargo_build_project
            }

            'armv7-unknown-linux-musleabihf' => {
                aria2c https://github.com/nushell/integrations/releases/download/build-tools/armv7r-linux-musleabihf-cross.tgz
                tar -xf armv7r-linux-musleabihf-cross.tgz -C $env.HOME
                $env.PATH = ($env.PATH | split row (char esep) | prepend $'($env.HOME)/armv7r-linux-musleabihf-cross/bin')
                $env.CARGO_TARGET_ARMV7_UNKNOWN_LINUX_MUSLEABIHF_LINKER = 'armv7r-linux-musleabihf-gcc'
                do $cargo_build_project
            }

            'i686-unknown-linux-gnu' => {
                sudo apt-get install gcc-multilib -y
                do $cargo_build_project
            }

            _ => {
                if $USE_UBUNTU { sudo apt install musl-tools -y }
                do $cargo_build_project
            }
        }
    }

    # --- Windows Build ---
    if $os =~ 'windows' {
        match $target {
            'x86_64-pc-windows-gnu' => {
                print "Downloading MinGW toolchain..."
                curl.exe -L -o mingw.7z "https://github.com/niXman/mingw-builds-binaries/releases/download/15.2.0-rt_v13-rev1/x86_64-15.2.0-release-posix-seh-ucrt-rt_v13-rev1.7z"
                print "Extracting MinGW toolchain..."
                7z x mingw.7z -y -omingw | ignore
                $env.PATH = ($env.PATH | split row (char esep) | prepend $"($env.PWD)/mingw/mingw64/bin")
                $env.CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER = 'gcc'
                do $cargo_build_project
            }
            _ => {
                do $cargo_build_project
            }
        }
    }

    # --- Packaging Artifacts ---
    let suffix = if $os =~ 'windows' { '.exe' } else { '' }
    let executable_pattern = $"target/($target)/release/($bin)*($suffix)"

    cd $src
    mkdir $dist

    # Clean up build artifacts
    rm -rf ...(glob $"target/($target)/release/*.d")

    print $"(char nl)Copying release files..."
    hr-line

    let files_config = (try { $config.metadata.files } catch { { include: ["README.md", "LICENSE"], exclude: [] } })
    let include_globs = (try { $files_config.include } catch { ["README.md", "LICENSE"] })
    let exclude_globs = (try { $files_config.exclude } catch { [] })

    # Start with the executable(s)
    mut assets = (glob $executable_pattern)

    # Add included files
    for pattern in $include_globs {
        let matches = (glob $pattern)
        $assets = ($assets | append $matches)
    }

    $assets = ($assets | uniq)

    # Remove excluded files
    for pattern in $exclude_globs {
        let excluded = (glob $pattern)
        $assets = ($assets | where {|it| $it not-in $excluded })
    }

    $assets | each {|it| if ($it | path exists) { cp -rv $it $dist } } | flatten

    # --- Create Archive ---
    cd $dist
    print $"(char nl)Creating release archive..."
    hr-line

    let release_name = $"($bin)-($version)-($target)"

    if $os in ['macos-latest'] or $USE_UBUNTU {
        let archive = $"($dist)/($release_name).tar.gz"
        mkdir $release_name
        let files_to_archive = (ls | where name != $release_name and name !~ '\.(deb|rpm|apk)$' | get name)
        $files_to_archive | each {|it| mv $it $release_name }
        tar -czf $archive $release_name
        echo $"archive=($archive)(char nl)" o>> $env.GITHUB_OUTPUT
    } else if $os =~ 'windows' {
        let archive = $"($dist)/($release_name).zip"
        # Exclude .msi and .zip files to prevent including the installer or the archive itself
        let files = (glob * | where ($it | path parse | get extension | $in not-in ['msi', 'zip']))
        7z a $archive ...$files
        if ($archive | path exists) {
            let normalized_archive = ($archive | str replace --all '\' '/')
            echo $"archive=($normalized_archive)(char nl)" o>> $env.GITHUB_OUTPUT
        }

        # Optional: Windows MSI packaging
        let msi_enabled = (try { $config.msi.enable } catch { false })
        let tpl_wxs = $"($env.GITHUB_WORKSPACE)/.github/workflows/templates/main.template.wxs"
        let tpl_wixproj = $"($env.GITHUB_WORKSPACE)/.github/workflows/templates/build.template.wixproj"
        let tpl_wxl = $"($env.GITHUB_WORKSPACE)/.github/workflows/templates/main.template.wxl"

        if $msi_enabled and ($tpl_wxs | path exists) and ($tpl_wixproj | path exists) {
            let can_build_msi = [dotnet wix] | all { (which $in | length) > 0 }
            if $can_build_msi and (try { wix --version | split row . | first | into int } catch { 0 }) >= 6 {
                print $"(char nl)Building MSI package..."
                let wix_dir = $"($src)/wix"
                if not ($wix_dir | path exists) { mkdir $wix_dir }

                let maintainer = (try { $config.metadata.maintainer } catch { "Maintainer" })
                let wxs_content = (open --raw $tpl_wxs | str replace --all "{{maintainer}}" $maintainer)
                $wxs_content | save --force $"($wix_dir)/main.wxs"
                cp $tpl_wixproj $"($wix_dir)/build.wixproj"
                if ($tpl_wxl | path exists) { cp $tpl_wxl $"($wix_dir)/main.wxl" }

                cd $src; cd wix; mkdir $bin
                # Copy only base assets to the target folder, excluding any existing archives or installers
                ls $dist | where type == file | where ($it.name | path parse | get extension | $in not-in ['msi', 'zip']) | each {|it| cp -r $it.name $"($bin)/" }

                # Generate LICENSE.rtf for WiX UI (requires RTF format)
                let license_file = $"($bin)/LICENSE"
                if ($license_file | path exists) {
                    let license_text = (open --raw $license_file | str replace --all "\n" "\\line ")
                    let rtf_content = $"{\\rtf1\\ansi\\deff0{\\fonttbl{\\f0\\fnil\\fcharset0 Arial;}}\\viewkind4\\uc1\\pard\\lang1033\\f0\\fs22 ($license_text)\\par}"
                    $rtf_content | save --force $"($bin)/LICENSE.rtf"
                }

                # Calculate WiX architecture
                let arch = match $target {
                    'x86_64-pc-windows-msvc' | 'x86_64-pc-windows-gnu'  => 'x64'
                    'i686-pc-windows-msvc' | 'i686-pc-windows-gnu'      => 'x86'
                    'aarch64-pc-windows-msvc' | 'aarch64-pc-windows-gnu' => 'arm64'
                    _ => 'x64'
                }

                let _hash = ($bin | hash md5)
                let upgrade_code = $"($_hash | str substring 0..7)-($_hash | str substring 8..11)-($_hash | str substring 12..15)-($_hash | str substring 16..19)-($_hash | str substring 20..31)"

                # Fix execution of dotnet build avoiding dummy executable copy-paste
                with-env { PROJECT_NAME: $bin, PROJECT_VERSION: $version, UPGRADE_CODE: $upgrade_code } {
                    dotnet build -c Release $"-p:Platform=($arch)"
                }

                let wix_msi   = (glob **/*.msi | where $it =~ $bin | get 0)
                let final_msi = $"($dist)/($release_name).msi"
                mv $wix_msi $final_msi
                echo $"msi=($final_msi | str replace --all '\' '/')(char nl)" o>> $env.GITHUB_OUTPUT
            }
        }

        # NuGet packaging
        let nuget_enabled = (try { $config.nuget.enable } catch { false })
        if $nuget_enabled and $target == "x86_64-pc-windows-msvc" {
            let n_template = $"($env.GITHUB_WORKSPACE)/.github/workflows/templates/Nuspec.template.xml"
            if ($n_template | path exists) {
                print $"(char nl)Building NuGet package..."
                let authors = (try { $config.nuget.authors } catch { (try { $config.metadata.maintainer } catch { "Maintainer" }) })
                let description = (try { $config.metadata.description } catch { "" })
                let repo = (try { $config.metadata.repository } catch { "" })

                let n_content = (open --raw $n_template
                    | str replace --all "{{bin}}" $bin
                    | str replace --all "{{version}}" $version
                    | str replace --all "{{authors}}" $authors
                    | str replace --all "{{description}}" $description
                    | str replace --all "{{repository}}" $repo)
                
                let nuspec_file = $"($dist)/($bin).nuspec"
                $n_content | save --force $nuspec_file
                
                try {
                    ^nuget pack $nuspec_file -OutputDirectory $dist
                    print $"Created NuGet package in ($dist)"
                } catch {
                    print "Failed to create NuGet package. Is nuget.exe available?"
                }
            } else {
                print $"Warning: ($n_template) not found. Skipping NuGet package."
            }
        }
    }

}

def run_publish [] {
    let config_file = "release.toml"
    if not ($config_file | path exists) {
        print $"::error::($config_file) not found."
        exit 1
    }
    let config = (open $config_file)

    let is_tag = ($env.REF? | default "" | str starts-with "refs/tags/")
    let bin_version = (try { $config.metadata.version } catch { "" })
    let tag_name = if $is_tag { ($env.REF | str replace 'refs/tags/' '') } else { $"v($bin_version)" }
    let is_prerelease = ($tag_name | str contains -i "beta") or ($tag_name | str contains -i "rc")
    let clean_version = ($tag_name | str replace --regex '^v' '')

    
    let dist = $"($env.GITHUB_WORKSPACE)/output"
    if not ($dist | path exists) {
        print $"::error::Output directory ($dist) not found. Build is required or artifacts download failed."
        exit 1
    }
    # 0. Generate Installers
    let installer_enabled = (try { $config.installer.enable } catch { false })
    if $installer_enabled {
        print $"(char nl)[Installer] Generating installer scripts..."
        hr-line
        let bin = (try { $config.metadata.bin } catch { "" })
        let version = (try { $config.metadata.version } catch { "" })
        let repo = (try { $config.metadata.repository } catch { "" })
        let features = (try { $config.installer.features } catch { [] })
        let p_linux = (try { $config.installer.path } catch { "~/.local/bin" })
        let p_win = (try { $config.installer.path-win } catch { "C:/bin" })

        let target_keys = (try { $config.targets | columns } catch { [] })
        let targets_context = ($target_keys | reduce -f {} {|it, acc| $acc | insert $"target.($it)" (try { $config.targets | get $it } catch { false }) })

        if "sh" in $features {
            let tpl_sh = ".github/workflows/templates/installer.template.sh"
            if ($tpl_sh | path exists) {
                let content = (format_template $tpl_sh $targets_context | str replace --all "{{bin}}" $bin | str replace --all "{{version}}" $version | str replace --all "{{repository}}" $repo | str replace --all "{{path}}" $p_linux)
                $content | save --force $"($dist)/install.sh"
                print $"Generated ($dist)/install.sh"
            } else {
                print $"Warning: ($tpl_sh) not found."
            }
        }

        if "ps1" in $features {
            let tpl_ps1 = ".github/workflows/templates/installer.template.ps1"
            if ($tpl_ps1 | path exists) {
                let content = (format_template $tpl_ps1 $targets_context | str replace --all "{{bin}}" $bin | str replace --all "{{version}}" $version | str replace --all "{{repository}}" $repo | str replace --all "{{path-win}}" $p_win)
                $content | save --force $"($dist)/install.ps1"
                print $"Generated ($dist)/install.ps1"
            } else {
                print $"Warning: ($tpl_ps1) not found."
            }
        }
    }

    # 0.5. Generate nFPM Packages
    let bin_name = (try { $config.metadata.bin } catch { "" })
    let bin_version = (try { $config.metadata.version } catch { "" })
    let use_nfpm = (try { $config.nfpm.enable } catch { false })

    if $use_nfpm and (which nfpm | is-not-empty) {
        print $"(char nl)[nFPM] Building Linux packages from artifacts..."
        hr-line
        
        let tpl_nfpm = ".github/workflows/templates/nfpm.template.yaml"
        if not ($tpl_nfpm | path exists) {
            print $"::error::($tpl_nfpm) not found. Skipping nFPM packaging."
        } else {
            let maintainer = (try { $config.metadata.maintainer } catch { "Maintainer" })
            let description = (try { $config.metadata.description } catch { "" })
            let vendor = (try { $config.metadata.vendor } catch { "CodeTease" })
            let homepage = (try { $config.metadata.homepage } catch { "" })
            let license = (try { $config.metadata.license } catch { "MIT" })
            let repo = (try { $config.metadata.repository } catch { "" })

            let n_context = {
                "nfpm.enable": $use_nfpm,
            }

            let n_content = (format_template $tpl_nfpm $n_context
                | str replace --all "{{bin}}" $bin_name
                | str replace --all "{{maintainer}}" $maintainer
                | str replace --all "{{description}}" $description
                | str replace --all "{{vendor}}" $vendor
                | str replace --all "{{homepage}}" $homepage
                | str replace --all "{{repository}}" $repo
                | str replace --all "{{license}}" $license)
            
            $n_content | save --force $"($env.GITHUB_WORKSPACE)/nfpm.yaml"
            print $"Generated ($env.GITHUB_WORKSPACE)/nfpm.yaml"

            let archives = (glob $"($dist)/*.tar.gz" | where {|f| ($f | path basename) =~ "linux" })
            
            for archive in $archives {
                let base = ($archive | path basename | str replace ".tar.gz" "")
                let target_str = ($base | str replace $"($bin_name)-($bin_version)-" "")
                
                let nfpm_arch = match $target_str {
                    'x86_64-unknown-linux-gnu' | 'x86_64-unknown-linux-musl' => 'amd64'
                    'i686-unknown-linux-gnu' => '386'
                    'aarch64-unknown-linux-gnu' | 'aarch64-unknown-linux-musl' => 'arm64'
                    'armv7-unknown-linux-gnueabihf' | 'armv7-unknown-linux-musleabihf' => 'arm7'
                    's390x-unknown-linux-gnu' => 's390x'
                    'powerpc64le-unknown-linux-gnu' => 'ppc64le'
                    _ => ''
                }
                
                if $nfpm_arch != "" {
                    print $"[nFPM] Processing ($target_str) for ($nfpm_arch)..."
                    let tmp_dir = $"($dist)/tmp_($target_str)"
                    mkdir $tmp_dir
                    
                    # Extract the tar.gz exactly into it
                    tar -xzf $archive -C $tmp_dir
                    
                    let bin_path = $"($tmp_dir)/($base)/($bin_name)"
                    
                    if ($bin_path | path exists) {
                        cp -v $bin_path $"($env.GITHUB_WORKSPACE)/($bin_name)"
                        
                        with-env { ARCH: $nfpm_arch, VERSION: $bin_version } {
                            cd $env.GITHUB_WORKSPACE
                            
                            let is_musl = ($target_str | str contains "musl")
                            let packagers = if $is_musl {
                                ["apk"]
                            } else {
                                ["deb", "rpm"]
                            }
                            
                            $packagers | each {|packager|
                                let pkg_file = $"($dist)/($bin_name)-($bin_version)-($target_str).($packager)"
                                print $"  -> Packaging ($packager)..."
                                nfpm pkg --packager $packager --target $pkg_file
                            }
                        }
                    }
                    
                    rm -rf $tmp_dir
                    rm -f $"($env.GITHUB_WORKSPACE)/($bin_name)"
                }
            }
            rm -f $"($env.GITHUB_WORKSPACE)/nfpm.yaml"
        }
    }

    # 0.75 Generate PKGBUILD for Arch Linux
    let arch_enabled = (try { $config.archlinux.enable } catch { false })
    let p_template = ".github/workflows/templates/PKGBUILD.template"
    if $arch_enabled and ($p_template | path exists) {
        print $"(char nl)[Arch Linux] Generating PKGBUILD and .SRCINFO..."
        hr-line

        let bin_name = (try { $config.metadata.bin } catch { "" })
        let bin_version = (try { $config.metadata.version } catch { "" })
        let repo = (try { $config.metadata.repository } catch { "" })
        let maintainer = (try { $config.metadata.maintainer } catch { "Maintainer" })
        let description = (try { $config.metadata.description } catch { "" })
        let license = (try { $config.metadata.license } catch { "MIT" })

        # Calculate SHA256 of the linux archives
        let x86_archive = $"($dist)/($bin_name)-($bin_version)-x86_64-unknown-linux-gnu.tar.gz"
        let arch_archive = $"($dist)/($bin_name)-($bin_version)-aarch64-unknown-linux-gnu.tar.gz"

        let sha256_x86 = if ($x86_archive | path exists) {
            try { ^sha256sum $x86_archive | split row ' ' | first } catch { "SKIP" }
        } else { "SKIP" }
        
        let sha256_aarch64 = if ($arch_archive | path exists) {
            try { ^sha256sum $arch_archive | split row ' ' | first } catch { "SKIP" }
        } else { "SKIP" }

        let has_x86_64 = (try { $config.targets | get "x86_64-unknown-linux-gnu" } catch { false })
        let has_aarch64 = (try { $config.targets | get "aarch64-unknown-linux-gnu" } catch { false })
        let p_context = {
            "arch.x86_64": $has_x86_64,
            "arch.aarch64": $has_aarch64
        }
        let p_content = (format_template $p_template $p_context 
            | str replace --all "{{bin}}" $bin_name 
            | str replace --all "{{version}}" $bin_version 
            | str replace --all "{{repository}}" $repo
            | str replace --all "{{maintainer}}" $maintainer
            | str replace --all "{{description}}" $description
            | str replace --all "{{license}}" $license
            | str replace --all "{{sha256_x86_64}}" $sha256_x86
            | str replace --all "{{sha256_aarch64}}" $sha256_aarch64)
        
        $p_content | save --force $"($dist)/PKGBUILD"
        print $"Generated ($dist)/PKGBUILD"

        # Generate .SRCINFO
        print "Generating .SRCINFO via Docker..."
        try {
            ^docker run --rm -v $"($dist):/pkg" archlinux /bin/bash -c "HOST_UID=$(stat -c %u /pkg/PKGBUILD); HOST_GID=$(stat -c %g /pkg/PKGBUILD); useradd -m build && pacman -Sy --noconfirm base-devel sudo git && cp /pkg/PKGBUILD /home/build/ && chown -R build:build /home/build && cd /home/build && sudo -u build makepkg --printsrcinfo > .SRCINFO && cp .SRCINFO /pkg/ && chown $HOST_UID:$HOST_GID /pkg/.SRCINFO"
            print $"Generated ($dist)/.SRCINFO"

            let arch_pkg = $"($bin_name)-($bin_version)-archlinux-pkgbuild.tar.gz"
            tar -czf $"($dist)/($arch_pkg)" -C $dist PKGBUILD .SRCINFO
            print $"Generated ($dist)/($arch_pkg)"
        } catch {
            print "Failed to generate .SRCINFO via docker"
        }
    }

    # 0.85 Generate Homebrew Formula
    let brew_enabled = (try { $config.brew.enable } catch { false })
    let f_template = ".github/workflows/templates/Formula.template.rb"
    if $brew_enabled and ($f_template | path exists) {
        print $"(char nl)[Homebrew] Generating Formula..."
        hr-line

        let bin_name = (try { $config.metadata.bin } catch { "" })
        let class_name = ($bin_name | split row '-' | each { |it| $it | str capitalize } | str join '')
        let bin_version = (try { $config.metadata.version } catch { "" })
        let repo = (try { $config.metadata.repository } catch { "" })
        let homepage = (try { $config.metadata.homepage } catch { "" })
        let description = (try { $config.metadata.description } catch { "" })
        let license = (try { $config.metadata.license } catch { "MIT" })



        let url_mac_amd   = $"($repo)/releases/download/($tag_name)/($bin_name)-($bin_version)-x86_64-apple-darwin.tar.gz"
        let url_mac_arm   = $"($repo)/releases/download/($tag_name)/($bin_name)-($bin_version)-aarch64-apple-darwin.tar.gz"
        let url_linux_amd = $"($repo)/releases/download/($tag_name)/($bin_name)-($bin_version)-x86_64-unknown-linux-gnu.tar.gz"
        let url_linux_arm = $"($repo)/releases/download/($tag_name)/($bin_name)-($bin_version)-aarch64-unknown-linux-gnu.tar.gz"

        let f_mac_amd = (try { glob $"($dist)/*x86_64*apple-darwin*.tar.gz" | first } catch { "" })
        let hash_mac_amd = if $f_mac_amd != "" { try { ^sha256sum $f_mac_amd | split row ' ' | first } catch { "SKIP" } } else { "SKIP" }

        let f_mac_arm = (try { glob $"($dist)/*aarch64*apple-darwin*.tar.gz" | first } catch { "" })
        let hash_mac_arm = if $f_mac_arm != "" { try { ^sha256sum $f_mac_arm | split row ' ' | first } catch { "SKIP" } } else { "SKIP" }

        let f_linux_amd = (try { glob $"($dist)/*x86_64*unknown-linux-gnu*.tar.gz" | first } catch { "" })
        let hash_linux_amd = if $f_linux_amd != "" { try { ^sha256sum $f_linux_amd | split row ' ' | first } catch { "SKIP" } } else { "SKIP" }

        let f_linux_arm = (try { glob $"($dist)/*aarch64*unknown-linux-gnu*.tar.gz" | first } catch { "" })
        let hash_linux_arm = if $f_linux_arm != "" { try { ^sha256sum $f_linux_arm | split row ' ' | first } catch { "SKIP" } } else { "SKIP" }

        let f_content = (open --raw $f_template
            | str replace --all "{{class_name}}" $class_name
            | str replace --all "{{description}}" $description
            | str replace --all "{{homepage}}" $homepage
            | str replace --all "{{version}}" $bin_version
            | str replace --all "{{license}}" $license
            | str replace --all "{{bin}}" $bin_name
            | str replace --all "{{url_mac_amd}}" $url_mac_amd
            | str replace --all "{{sha256_mac_amd}}" $hash_mac_amd
            | str replace --all "{{url_mac_arm}}" $url_mac_arm
            | str replace --all "{{sha256_mac_arm}}" $hash_mac_arm
            | str replace --all "{{url_linux_amd}}" $url_linux_amd
            | str replace --all "{{sha256_linux_amd}}" $hash_linux_amd
            | str replace --all "{{url_linux_arm}}" $url_linux_arm
            | str replace --all "{{sha256_linux_arm}}" $hash_linux_arm)
        
        $f_content | save --force $"($dist)/($bin_name).rb"
        print $"Generated ($dist)/($bin_name).rb"
    }

    # 0.90 Generate Scoop Manifest
    let scoop_enabled = (try { $config.scoop.enable } catch { false })
    let s_template = ".github/workflows/templates/Scoop.template.json"
    if $scoop_enabled and ($s_template | path exists) {
        print $"(char nl)[Scoop] Generating Manifest..."
        hr-line

        let bin_name = (try { $config.metadata.bin } catch { "" })
        let bin_version = (try { $config.metadata.version } catch { "" })
        let repo = (try { $config.metadata.repository } catch { "" })
        let homepage = (try { $config.metadata.homepage } catch { "" })
        let description = (try { $config.metadata.description } catch { "" })
        let license = (try { $config.metadata.license } catch { "MIT" })



        let url_win_x64   = $"($repo)/releases/download/($tag_name)/($bin_name)-($bin_version)-x86_64-pc-windows-msvc.zip"
        let url_win_x86   = $"($repo)/releases/download/($tag_name)/($bin_name)-($bin_version)-i686-pc-windows-msvc.zip"
        let url_win_arm64 = $"($repo)/releases/download/($tag_name)/($bin_name)-($bin_version)-aarch64-pc-windows-msvc.zip"

        let f_win_x64 = (try { glob $"($dist)/*x86_64-pc-windows-msvc*.zip" | first } catch { "" })
        let hash_win_x64 = if $f_win_x64 != "" { try { ^sha256sum $f_win_x64 | split row ' ' | first } catch { "SKIP" } } else { "SKIP" }

        let f_win_x86 = (try { glob $"($dist)/*i686-pc-windows-msvc*.zip" | first } catch { "" })
        let hash_win_x86 = if $f_win_x86 != "" { try { ^sha256sum $f_win_x86 | split row ' ' | first } catch { "SKIP" } } else { "SKIP" }

        let f_win_arm64 = (try { glob $"($dist)/*aarch64-pc-windows-msvc*.zip" | first } catch { "" })
        let hash_win_arm64 = if $f_win_arm64 != "" { try { ^sha256sum $f_win_arm64 | split row ' ' | first } catch { "SKIP" } } else { "SKIP" }

        let s_content = (open --raw $s_template
            | str replace --all "{{description}}" $description
            | str replace --all "{{homepage}}" $homepage
            | str replace --all "{{version}}" $bin_version
            | str replace --all "{{license}}" $license
            | str replace --all "{{bin}}" $bin_name
            | str replace --all "{{repository}}" $repo
            | str replace --all "{{url_win_x64}}" $url_win_x64
            | str replace --all "{{sha256_win_x64}}" $hash_win_x64
            | str replace --all "{{url_win_x86}}" $url_win_x86
            | str replace --all "{{sha256_win_x86}}" $hash_win_x86
            | str replace --all "{{url_win_arm64}}" $url_win_arm64
            | str replace --all "{{sha256_win_arm64}}" $hash_win_arm64)
        
        $s_content | save --force $"($dist)/($bin_name).json"
        print $"Generated ($dist)/($bin_name).json"
    }

    # 0.95 Generate Registry Info
    let cloudsmith_enabled = (try { $config.cloudsmith.enable } catch { false })
    let docs_path = (try { $config.cloudsmith.docs_path } catch { "REGISTRY.md" })
    let docs_file = ($docs_path | path basename)
    let r_template = ".github/workflows/templates/Registry.template.md"
    if $cloudsmith_enabled and $docs_path != "" and ($r_template | path exists) {
        print $"(char nl)[Registry] Generating Registry instructions..."
        hr-line

        let bin_name = (try { $config.metadata.bin } catch { "" })
        let bin_version = (try { $config.metadata.version } catch { "" })
        let repo_path = (try { $config.cloudsmith.repo } catch { "" })
        let repo_url = (try { $config.metadata.repository } catch { "" })
        let github_org = (try { $config.docker.github_org } catch {
            ($env.GITHUB_REPOSITORY? | default "codetease/cli-dummy" | split row "/" | first | str downcase)
        })

        let has_docker = (try { $config.docker.enable } catch { false })
        let registries = (try { $config.docker.registries } catch { [] })
        let has_ghcr = "ghcr" in $registries
        let has_cloudsmith = "cloudsmith" in $registries

        let brew_tap = (try { $config.brew.tap } catch { "" })
        let scoop_bucket = (try { $config.scoop.bucket } catch { "" })
        let scoop_bucket_name = if ($scoop_bucket | is-empty) { "" } else { $scoop_bucket | split row "/" | last }

        let cloudsmith_targets = (try { $config.cloudsmith.targets } catch { {} })
        
        let deb_target = (try { $cloudsmith_targets.deb } catch { "ubuntu/any-version" })
        let deb_parts = ($deb_target | split row "/")
        let apt_distro = (try { $deb_parts | first } catch { "ubuntu" })
        let apt_codename = (try { $deb_parts | last } catch { "any-version" })

        let rpm_target = (try { $cloudsmith_targets.rpm } catch { "el/any-version" })
        let rpm_parts = ($rpm_target | split row "/")
        let rpm_distro = (try { $rpm_parts | first } catch { "el" })
        let rpm_codename = (try { $rpm_parts | last } catch { "any-version" })

        let apk_target = (try { $cloudsmith_targets.apk } catch { "alpine/any-version" })
        let apk_parts = ($apk_target | split row "/")
        let apk_distro = (try { $apk_parts | first } catch { "alpine" })
        let apk_codename = (try { $apk_parts | last } catch { "any-version" })

        let context = {
            "cloudsmith.enable": $cloudsmith_enabled,
            "docker.enable": $has_docker,
            "ghcr.enable": ($has_docker and $has_ghcr),
            "docker.cloudsmith.enable": ($has_docker and $has_cloudsmith),
            "ghcr_only": ($has_docker and $has_ghcr and not $has_cloudsmith),
            "cloudsmith_only": ($has_docker and $has_cloudsmith and not $has_ghcr),
            "ghcr_and_cloudsmith": ($has_docker and $has_ghcr and $has_cloudsmith),
            "nuget.enable": (try { $config.nuget.enable } catch { false }),
            "archlinux.enable": (try { $config.archlinux.enable } catch { false }),
            "scoop.enable": (try { $config.scoop.enable } catch { false }),
            "brew.enable": (try { $config.brew.enable } catch { false }),
            "crate.enable": (try { $config.crate.enable } catch { false }),
            "crate.crates_io": (try { $config.crate.enable and ("crates.io" in $config.crate.registries) } catch { false }),
            "crate.cloudsmith": (try { $config.crate.enable and ("cloudsmith" in $config.crate.registries) } catch { false })
        }

        let r_content = (format_template $r_template $context
            | str replace --all "{{bin}}" $bin_name
            | str replace --all "{{version}}" $bin_version
            | str replace --all "{{repo_path}}" $repo_path
            | str replace --all "{{repository}}" $repo_url
            | str replace --all "{{github_org}}" $github_org
            | str replace --all "{{brew_tap}}" $brew_tap
            | str replace --all "{{scoop_bucket}}" $scoop_bucket
            | str replace --all "{{scoop_bucket_name}}" $scoop_bucket_name
            | str replace --all "{{apt_distro}}" $apt_distro
            | str replace --all "{{apt_codename}}" $apt_codename
            | str replace --all "{{rpm_distro}}" $rpm_distro
            | str replace --all "{{rpm_codename}}" $rpm_codename
            | str replace --all "{{apk_distro}}" $apk_distro
            | str replace --all "{{apk_codename}}" $apk_codename)
        
        $r_content | save --force $"($dist)/($docs_file)"
        print $"Generated ($dist)/($docs_file)"

        do {
            let target_docs_path = $"($env.GITHUB_WORKSPACE)/($docs_path)"
            let target_dir = ($target_docs_path | path dirname)
            if not ($target_dir | path exists) { mkdir $target_dir }
            
            cp -f $"($dist)/($docs_file)" $target_docs_path
            
            print "Checking for changes in registry docs..."
            cd $env.GITHUB_WORKSPACE
            let actor = "github-actions[bot]"
            let email = "41898282+github-actions[bot]@users.noreply.github.com"
            try {
                git config --local user.name $actor
                git config --local user.email $email
                git add $docs_path
                let status = (git status --porcelain $docs_path)
                if ($status | is-not-empty) {
                    print $"Committing changes for ($docs_path)..."
                    git commit -m $"update registry docs for v($bin_version)"
                    
                    # Fix: detached HEAD push by specifying target branch
                    let target_branch = if $is_tag {
                        # Default to the primary branch for tag-triggered releases
                        (try { ^gh repo view --json defaultBranchRef --template '{{.defaultBranchRef.name}}' } catch { "main" })
                    } else {
                        ($env.GITHUB_REF_NAME? | default ($env.REF? | default "refs/heads/main" | split row "/" | last))
                    }
                    print $"[Git] Pushing changes to ($target_branch)..."
                    git push origin $"HEAD:($target_branch)"
                    print "[Git] Successfully committed and pushed registry docs."
                } else {
                    print "[Git] No changes in registry docs to commit."
                }
            } catch {|err|
                print $"::warning::Failed to commit or push registry docs: ($err.msg)"
            }
        }
    }

    # 0.98 Build and Push Docker Images
    let docker_enabled = (try { $config.docker.enable } catch { false })
    let registries = (try { $config.docker.registries } catch { [] })
    let templates = (try { $config.docker.templates } catch { [] })
    mut docker_release_notes = []

    if $docker_enabled and ($templates | is-not-empty) and ($registries | is-not-empty) {
        print $"(char nl)[Docker] Building and Pushing Images..."
        hr-line

        let bin_name = (try { $config.metadata.bin } catch { "" })
        let bin_version = (try { $config.metadata.version } catch { "" })
        let image_name = (try { $config.docker.image_name } catch { $bin_name })
        


        let has_ghcr = "ghcr" in $registries
        let has_cloudsmith = "cloudsmith" in $registries

        if $has_ghcr {
            if ($env.GITHUB_TOKEN? | is-not-empty) and ($env.GITHUB_ACTOR? | is-not-empty) {
                print "Logging into ghcr.io..."
                $env.GITHUB_TOKEN | docker login ghcr.io -u $env.GITHUB_ACTOR --password-stdin
            } else {
                print "Warning: GITHUB_TOKEN or GITHUB_ACTOR is missing. GHCR login skipped."
            }
        }

        if $has_cloudsmith {
            if ($env.CLOUDSMITH_API_KEY? | is-not-empty) {
                # Cloudsmith user is not always the workspace name
                let cloudsmith_docker_username = ($config.docker.cloudsmith_docker_username | default "")
                print "Logging into docker.cloudsmith.io..."
                $env.CLOUDSMITH_API_KEY | docker login docker.cloudsmith.io -u $cloudsmith_docker_username --password-stdin
            } else {
                 print "Warning: CLOUDSMITH_API_KEY is missing. Cloudsmith login skipped."
            }
        }

        $docker_release_notes = ($docker_release_notes | append "### 🐳 Docker Images")
        $docker_release_notes = ($docker_release_notes | append "")
        $docker_release_notes = ($docker_release_notes | append "Multi-architecture Docker images are available in the following registries:")
        $docker_release_notes = ($docker_release_notes | append "")

        for tpl in $templates {
            print $"Preparing context for ($tpl)..."
            let target_dir = $"($dist)/docker_build_($tpl)"
            mkdir $"($target_dir)/amd64"
            mkdir $"($target_dir)/arm64"
            
            let is_alpine = $tpl == "alpine"
            let suffix = if $is_alpine { "musl" } else { "gnu" }
            
            let linux_amd64_tar = $"($dist)/($bin_name)-($bin_version)-x86_64-unknown-linux-($suffix).tar.gz"
            let linux_arm64_tar = $"($dist)/($bin_name)-($bin_version)-aarch64-unknown-linux-($suffix).tar.gz"
            
            mut available_platforms = []
            
            if ($linux_amd64_tar | path exists) {
                tar -xzf $linux_amd64_tar -C $"($target_dir)/amd64"
                let extracted_dir = $"($bin_name)-($bin_version)-x86_64-unknown-linux-($suffix)"
                mv $"($target_dir)/amd64/($extracted_dir)/($bin_name)" $"($target_dir)/amd64/($bin_name)"
                $available_platforms = ($available_platforms | append "linux/amd64")
            }
            if ($linux_arm64_tar | path exists) {
                tar -xzf $linux_arm64_tar -C $"($target_dir)/arm64"
                let extracted_dir = $"($bin_name)-($bin_version)-aarch64-unknown-linux-($suffix)"
                mv $"($target_dir)/arm64/($extracted_dir)/($bin_name)" $"($target_dir)/arm64/($bin_name)"
                $available_platforms = ($available_platforms | append "linux/arm64")
            }
            
            if ($available_platforms | is-empty) {
                print "Warning: No linux archives found to build Docker image."
                continue
            }
            
            let platforms = ($available_platforms | str join ",")
            
            let d_template = $".github/workflows/templates/Dockerfile.($tpl).template"
            let d_content = (open --raw $d_template | str replace --all "{{bin}}" $bin_name | str replace --all "{{version}}" $bin_version)
            let d_file = $"($target_dir)/Dockerfile"
            $d_content | save --force $d_file
            
            mut build_args = ["buildx" "build" "--push" "--platform" $platforms "-f" $d_file "--provenance=false" "--sbom=false"]
            
            $docker_release_notes = ($docker_release_notes | append $"**Variant:** `($tpl)`")
            $docker_release_notes = ($docker_release_notes | append "```bash")

            for reg in $registries {
                let full_image = if $reg == "ghcr" {
                    let repo_owner = ($env.GITHUB_REPOSITORY? | default "codetease/cli-dummy" | split row "/" | first | str downcase)
                    $"ghcr.io/($repo_owner)/($image_name)"
                } else if $reg == "cloudsmith" {
                    let repo_path = (try { $config.cloudsmith.repo } catch { "codetease/tools" })
                    $"docker.cloudsmith.io/($repo_path)/($image_name)"
                } else {
                    $image_name
                }
                
                let variant_tags = if $is_prerelease {
                    match $tpl {
                        "alpine" => [$clean_version, $"($clean_version)-alpine"],
                        "debian-slim" => [$"($clean_version)-bookworm"],
                        _ => [$"($clean_version)-($tpl)"]
                    }
                } else {
                    match $tpl {
                        "alpine" => ["latest", $clean_version, "alpine", $"($clean_version)-alpine"],
                        "debian-slim" => ["latest-bookworm", $"($clean_version)-bookworm"],
                        _ => [$tpl, $"($clean_version)-($tpl)"]
                    }
                }
                
                for t in $variant_tags {
                    $build_args = ($build_args | append ["-t" $"($full_image):($t)"])
                }

                # Use the versioned tag for release notes
                let note_tag = ($variant_tags | where {|it| $it == $clean_version or $it == $"($clean_version)-bookworm" or $it == $"($clean_version)-($tpl)" } | first)
                $docker_release_notes = ($docker_release_notes | append $"docker pull ($full_image):($note_tag)")
            }
            
            $docker_release_notes = ($docker_release_notes | append "```")
            $docker_release_notes = ($docker_release_notes | append "")

            $build_args = ($build_args | append $target_dir)
            
            print $"Running docker ($build_args | str join ' ')"
            try {
                ^docker ...$build_args
                print $"Successfully built and pushed ($tpl) image."
                rm -rf $target_dir
            } catch {
                print $"Error: Docker build/push failed for ($tpl)"
                rm -rf $target_dir
            }
        }
    }

    # 1. GitHub Release
    if $is_tag {
        print $"(char nl)[GitHub] Creating Release Draft & Uploading Assets..."
        hr-line

        
        let bin = (try { $config.metadata.bin } catch { "" })
        let version = (try { $config.metadata.version } catch { "" })
        let features = (try { $config.installer.features } catch { [] })

        let matrix_str = ($env.MATRIX? | default "[]")
        let target_names = if ($matrix_str == "[]" or ($matrix_str | is-empty)) {
            {}
        } else {
            let matrix_items = ($matrix_str | from json)
            $matrix_items | reduce -f {} {|it, acc| 
                let dname = (try { $it.display_name } catch { $it.target })
                $acc | insert $it.target $dname 
            }
        }

        let assets_for_table = (ls $dist | where type == file | get name | where {|f|
            let base = ($f | path basename)
            not ($base in ['install.sh', 'install.ps1', 'PKGBUILD', '.SRCINFO']) and not ($base | str ends-with ".sha256")
        })

        mut rows = ["| Operating System & Architecture | Format | Checksum (SHA256) |", "|---|---|---|"]
        
        mut has_i686 = false
        mut has_s390x = false

        for file in $assets_for_table {
            let base = ($file | path basename)
            let sha = (try { ^sha256sum $file | split row ' ' | first } catch { "UNKNOWN" })
            
            let ext = if ($base | str ends-with ".tar.gz") {
                ".tar.gz"
            } else if ($base | str ends-with ".zip") {
                ".zip"
            } else if ($base | str ends-with ".msi") {
                ".msi"
            } else if ($base | str ends-with ".deb") {
                ".deb"
            } else if ($base | str ends-with ".rpm") {
                ".rpm"
            } else if ($base | str ends-with ".apk") {
                ".apk"
            } else {
                ""
            }

            if $ext != "" {
                let without_prefix = ($base | str replace $"($bin)-($version)-" "")
                let target_str = ($without_prefix | str replace $ext "")
                
                if ($target_str | str starts-with "i686") { $has_i686 = true }
                if ($target_str | str starts-with "s390x") { $has_s390x = true }

                let os_arch = (try { $target_names | get $target_str } catch { $target_str })
                $rows = ($rows | append $"| ($os_arch) | `($ext)` | `($sha)` |")
            }
        }
        
        mut notes_lines = []
        $notes_lines = ($notes_lines | append "### 📦 Target List")
        $notes_lines = ($notes_lines | append "")
        $notes_lines = ($notes_lines | append $rows)
        $notes_lines = ($notes_lines | append "")

        if ($docker_release_notes | length) > 0 {
            $notes_lines = ($notes_lines | append $docker_release_notes)
        }

        if $installer_enabled {
            let github_repo = ($env.GITHUB_REPOSITORY? | default "OWNER/REPO")
            $notes_lines = ($notes_lines | append "### 🚀 Quick Installer Guide")
            $notes_lines = ($notes_lines | append "")
            
            if "ps1" in $features {
                $notes_lines = ($notes_lines | append "**Windows**:")
                $notes_lines = ($notes_lines | append "Instructions on using the command to execute `install.ps1`. This script automatically handles decompression and checks the CPU architecture (AMD64/ARM64).")
                $notes_lines = ($notes_lines | append "```powershell")
                $notes_lines = ($notes_lines | append $"irm 'https://github.com/($github_repo)/releases/latest/download/install.ps1' | iex")
                $notes_lines = ($notes_lines | append "```")
                $notes_lines = ($notes_lines | append "")
            }
            if "sh" in $features {
                $notes_lines = ($notes_lines | append "**Linux/macOS**:")
                $notes_lines = ($notes_lines | append "Instructions to execute the `install.sh`. This script automatically detects the OS (Linux/Darwin) and architecture to load the correct assets from the repository.")
                $notes_lines = ($notes_lines | append "```bash")
                $notes_lines = ($notes_lines | append $"curl -fsSL 'https://github.com/($github_repo)/releases/latest/download/install.sh' | bash")
                $notes_lines = ($notes_lines | append "```")
                $notes_lines = ($notes_lines | append "")
            }
        }

        if ($dist | path join $docs_file | path exists) {
            let github_repo = ($env.GITHUB_REPOSITORY? | default "OWNER/REPO")
            $notes_lines = ($notes_lines | append $"To install via package managers \(APT, RPM, APK, NuGet\), please download [($docs_file)]\(https://github.com/($github_repo)/releases/download/($tag_name)/($docs_file)\) to view the instructions.")
            $notes_lines = ($notes_lines | append "")
        }

        if $has_i686 or $has_s390x {
            $notes_lines = ($notes_lines | append "### ⚠️ Additional Information")
            $notes_lines = ($notes_lines | append "")
            if $has_i686 {
                $notes_lines = ($notes_lines | append "> **Note on `i686`**: This is an older legacy architecture. Support might be limited or deprecated in the future.")
                $notes_lines = ($notes_lines | append "")
            }
            if $has_s390x {
                $notes_lines = ($notes_lines | append "> **Note on `s390x`**: This is a Big Endian risk architecture. Proceed with caution as some libraries may assume Little Endian.")
                $notes_lines = ($notes_lines | append "")
            }
        }

        let notes_file = $"($dist)/RELEASE_NOTES.md"
        ($notes_lines | str join (char nl)) | save --force $notes_file

        # Check if release exists
        let release_exists = (try { gh release view $tag_name | complete } catch { {exit_code: 1} })
        let prerelease_flag = if $is_prerelease { ["--prerelease"] } else { [] }

        if $release_exists.exit_code != 0 {
            gh release create $tag_name --draft --title $"($tag_name)" --notes-file $notes_file ...$prerelease_flag
        } else {
            gh release edit $tag_name --notes-file $notes_file ...$prerelease_flag
        }
        
        # Upload all assets in dist (files only)
        let assets = (ls $dist | where type == file | get name | where {|f| not ($f | path basename | $in in ["RELEASE_NOTES.md", "PKGBUILD", ".SRCINFO"])})
        if ($assets | is-not-empty) {
            gh release upload $tag_name ...$assets --clobber
        } else {
             print "No assets found to upload to GitHub."
        }
    } else {
        print "Not a tag push. Skipping GitHub Release."
    }

    # 2. Cloudsmith
    let cloudsmith_enabled = (try { $config.cloudsmith.enable } catch { false })
    let repo = (try { $config.cloudsmith.repo } catch { "codetease/tools" })
    let targets_mapping = (try { $config.cloudsmith.targets } catch { { deb: "ubuntu/noble", rpm: "el/9", apk: "alpine/any-version" } })

    let can_publish = $cloudsmith_enabled and ($env.CLOUDSMITH_API_KEY? | is-not-empty) and $is_tag

    if $can_publish {
        print $"(char nl)[Cloudsmith] Publishing Packages..."
        hr-line
        
        if (which cloudsmith | is-empty) {
            print "::error::Cloudsmith CLI not found."
            exit 1
        }

        let pkgs = (try { ls $dist | where type == file | get name | where { |f| $f =~ '\.(deb|rpm|apk|nupkg)$' } } catch { [] })

        if ($pkgs | is-not-empty) {
            for pkg in $pkgs {
                let ext = ($pkg | path parse | get extension)

                let fallback_path = $"($repo)/any/version"
                let target_path_suffix = (try { $targets_mapping | get $ext } catch { "" })
                
                let target_path = if $ext == "nupkg" {
                    $repo
                } else if ($target_path_suffix | is-empty) {
                    $fallback_path
                } else {
                    $"($repo)/($target_path_suffix)"
                }

                let pkg_type = if $ext == "apk" { "alpine" } else if $ext == "nupkg" { "nuget" } else { $ext }
                
                print $"[Cloudsmith] Pushing ($ext) to ($target_path)..."
                cloudsmith push $pkg_type $target_path ($pkg | path expand) -k $env.CLOUDSMITH_API_KEY
            }
        } else {
            print "[Cloudsmith] Skipping publish: No linux packages found in output directory."
        }
    } else {
        print "[Cloudsmith] Skipping publish: Conditions not met (disabled, missing API key, or not a tag)."
    }

    # 3. Crate Publish
    let crate_enabled = (try { $config.crate.enable } catch { false })
    let crate_registries = (try { $config.crate.registries } catch { [] })
    if $crate_enabled and $is_tag {
        print $"(char nl)[Crate] Publishing to registries: ($crate_registries | str join ', ')..."
        hr-line
        
        # crates.io
        if "crates.io" in $crate_registries {
            if ($env.CARGO_REGISTRY_TOKEN? | is-not-empty) {
                print "[Crate] Publishing to crates.io..."
                try {
                    cargo publish --token $env.CARGO_REGISTRY_TOKEN
                    print "[Crate] Successfully published to crates.io"
                } catch {
                    print "::warning::[Crate] Failed to publish to crates.io"
                }
            } else {
                print "[Crate] Skipping crates.io publish: CARGO_REGISTRY_TOKEN not found."
            }
        }

        # cloudsmith
        if "cloudsmith" in $crate_registries {
            if $can_publish {
                print "[Crate] Packaging for Cloudsmith..."
                try {
                    cargo package
                    let crate_files = (glob target/package/*.crate)
                    if ($crate_files | is-not-empty) {
                        let crate_file = $crate_files.0
                        print $"[Crate] Pushing ($crate_file) to Cloudsmith ($repo)..."
                        cloudsmith push cargo $repo ($crate_file | path expand) -k $env.CLOUDSMITH_API_KEY
                    } else {
                        print "::warning::[Crate] No .crate file found in target/package."
                    }
                } catch {|err|
                    print $"::warning::[Crate] Failed to package or push to Cloudsmith: ($err.msg)"
                }
            } else {
                print "[Crate] Skipping cloudsmith publish: API Key not found or not a tag push."
            }
        }
    }
}