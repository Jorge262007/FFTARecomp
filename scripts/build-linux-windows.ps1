param([string]$OutputDir, [switch]$BuildOnly, [switch]$InstallerOnly)

$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
if (-not $OutputDir) { $OutputDir = Join-Path $repo 'Installer\Linux' }
$target = Join-Path $repo 'target'
$downloads = Join-Path $target 'linux-cross-downloads'
$sysroot = Join-Path $target 'linux-sysroot'
$toolchain = Join-Path $target 'linux-toolchain'
New-Item -ItemType Directory -Force -Path @($downloads, $sysroot, $toolchain, $OutputDir) | Out-Null

function Get-VerifiedDownload([string]$Name, [string]$Url, [string]$Hash) {
    $path = Join-Path $downloads $Name
    if ((-not (Test-Path -LiteralPath $path)) -or ((Get-FileHash -Algorithm SHA256 -LiteralPath $path).Hash -ne $Hash)) {
        Write-Host "Downloading $Name..."
        Invoke-WebRequest -UseBasicParsing -Uri $Url -OutFile $path
    }
    if ((Get-FileHash -Algorithm SHA256 -LiteralPath $path).Hash -ne $Hash) {
        throw "Download checksum mismatch: $Name"
    }
    return $path
}

if ($env:FFTA_ZIG_EXE -and (Test-Path -LiteralPath $env:FFTA_ZIG_EXE)) {
    $zig = $env:FFTA_ZIG_EXE
} else {
$zigArchive = Get-VerifiedDownload 'zig-windows-x86_64-0.13.0.zip' `
    'https://ziglang.org/download/0.13.0/zig-windows-x86_64-0.13.0.zip' `
    'D859994725EF9402381E557C60BB57497215682E355204D754EE3DF75EE3C158'
$zig = Join-Path $toolchain 'zig-windows-x86_64-0.13.0\zig.exe'
if (-not (Test-Path -LiteralPath $zig)) {
    Expand-Archive -LiteralPath $zigArchive -DestinationPath $toolchain -Force
}
}
if ($env:FFTA_LINUX_SYSROOT) { $sysroot = $env:FFTA_LINUX_SYSROOT }

$packages = @(
    @('libasound2-dev.deb', 'https://archive.ubuntu.com/ubuntu/pool/main/a/alsa-lib/libasound2-dev_1.2.6.1-1ubuntu1_amd64.deb', 'C302D0F11457C4B95943B029520505613079CA6C34E2AE18BA1B4B53BF0DE7DF'),
    @('libasound2.deb', 'https://archive.ubuntu.com/ubuntu/pool/main/a/alsa-lib/libasound2_1.2.6.1-1ubuntu1_amd64.deb', 'CB208A762817BDE5B83D1D52B7B89C747E9B484C857EA114BF97BACC1B29ECAE'),
    @('libudev-dev.deb', 'https://archive.ubuntu.com/ubuntu/pool/main/s/systemd/libudev-dev_249.11-0ubuntu3_amd64.deb', '1423F4C78806CAB848C7DFC6CAFBDB86F25C82EA8C66E08B69129544CA1A891D'),
    @('libudev1.deb', 'https://archive.ubuntu.com/ubuntu/pool/main/s/systemd/libudev1_249.11-0ubuntu3_amd64.deb', '289E2BA75F458B489435530E6C17E45101556330F1E74ADB8AE3DA3674419B15')
)
$lib = Join-Path $sysroot 'usr\lib\x86_64-linux-gnu'
if (-not ((Test-Path (Join-Path $lib 'libasound.so')) -and (Test-Path (Join-Path $lib 'libudev.so')))) {
    $tar = Join-Path $env:SystemRoot 'System32\tar.exe'
    foreach ($package in $packages) {
        $deb = Get-VerifiedDownload $package[0] $package[1] $package[2]
        $work = Join-Path $downloads ([IO.Path]::GetFileNameWithoutExtension($package[0]))
        New-Item -ItemType Directory -Force $work | Out-Null
        & $tar -xf $deb -C $work
        if ($LASTEXITCODE -ne 0) { throw "Cannot unpack $deb" }
        $data = Get-ChildItem -LiteralPath $work -Filter 'data.tar.*' | Select-Object -First 1
        if (-not $data) { throw "Missing data archive in $deb" }
        # Windows cannot create Debian symlinks. Extract regular files, then copy the two linker names.
        $oldErrorAction = $ErrorActionPreference
        $ErrorActionPreference = 'Continue'
        try { & $tar -xf $data.FullName -C $sysroot 2>$null } finally { $ErrorActionPreference = $oldErrorAction }
    }
    Copy-Item -LiteralPath (Join-Path $lib 'libasound.so.2.0.0') -Destination (Join-Path $lib 'libasound.so') -Force
    Copy-Item -LiteralPath (Join-Path $lib 'libudev.so.1.7.2') -Destination (Join-Path $lib 'libudev.so') -Force
}
foreach ($name in @('libasound.so', 'libudev.so', 'pkgconfig\alsa.pc', 'pkgconfig\libudev.pc')) {
    if (-not (Test-Path -LiteralPath (Join-Path $lib $name))) { throw "Missing Linux sysroot file: $name" }
}

$pkgConfig = Get-Command pkg-config.exe -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Source -First 1
if (-not $pkgConfig) {
    foreach ($candidate in @('E:\msys64\mingw64\bin\pkg-config.exe', 'C:\msys64\mingw64\bin\pkg-config.exe')) {
        if (Test-Path -LiteralPath $candidate) { $pkgConfig = $candidate; break }
    }
}
if (-not $pkgConfig) { throw 'Install MSYS2 mingw64 pkg-config (pkgconf) or add pkg-config.exe to PATH.' }

$installed = & rustup target list --installed
if ($installed -notcontains 'x86_64-unknown-linux-gnu') {
    $oldErrorAction = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    & rustup target add x86_64-unknown-linux-gnu
    $rustupExit = $LASTEXITCODE
    $ErrorActionPreference = $oldErrorAction
    if ($rustupExit -ne 0) { throw 'Could not install the Linux Rust target' }
}

$env:ZIG_GLOBAL_CACHE_DIR = Join-Path $target 'zig-cache-global'
$env:ZIG_LOCAL_CACHE_DIR = Join-Path $target 'zig-cache-local'
$env:FFTA_ZIG_EXE = $zig
$env:FFTA_LINUX_LIB_DIR = $lib
$wrapper = Join-Path $target 'linux-zig-linker.exe'
$oldErrorAction = $ErrorActionPreference
$ErrorActionPreference = 'Continue'
& $zig cc -target x86_64-windows-gnu -O2 -o $wrapper (Join-Path $PSScriptRoot 'linux-zig-linker.c')
$zigExit = $LASTEXITCODE
$ErrorActionPreference = $oldErrorAction
if ($zigExit -ne 0) { throw 'Could not build the Zig linker wrapper' }

$env:PKG_CONFIG_ALLOW_CROSS = '1'
$env:PKG_CONFIG_DONT_DEFINE_PREFIX = '1'
$env:PKG_CONFIG_SYSROOT_DIR = ($sysroot -replace '\\', '/')
$env:PKG_CONFIG_LIBDIR = ((Join-Path $lib 'pkgconfig') -replace '\\', '/')
$env:PKG_CONFIG = $pkgConfig
$env:CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER = $wrapper
$env:CC_x86_64_unknown_linux_gnu = $wrapper
$env:CARGO_TARGET_DIR = Join-Path $target 'linux-portable-build'

Push-Location $repo
try {
    $oldErrorAction = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    $buildPackages = @("-p", "installer")
    if (-not $InstallerOnly) { $buildPackages += @("-p", "recomp", "-p", "ffta-play") }
    & cargo build --release --target x86_64-unknown-linux-gnu @buildPackages
    $cargoExit = $LASTEXITCODE
    $ErrorActionPreference = $oldErrorAction
    if ($cargoExit -ne 0) { throw 'Linux build failed' }
} finally { Pop-Location }

$release = Join-Path $env:CARGO_TARGET_DIR 'x86_64-unknown-linux-gnu\release'
if (-not $InstallerOnly) {
& python (Join-Path $repo 'TinyCC\build.py') --platform linux --zig $zig --cc (Split-Path $pkgConfig -Parent | Join-Path -ChildPath 'gcc.exe') --output (Join-Path $release 'tcc\libtcc.so')
if ($LASTEXITCODE -ne 0) { throw 'TinyCC Linux build failed' }
}

if ($BuildOnly) {
    Write-Host 'Linux release build complete; no release package was created.'
    return
}
$release = Join-Path $env:CARGO_TARGET_DIR 'x86_64-unknown-linux-gnu\release'
$payload = Join-Path $OutputDir 'payload'
New-Item -ItemType Directory -Force $payload | Out-Null
Copy-Item -LiteralPath (Join-Path $release 'installer') -Destination (Join-Path $OutputDir 'FFTAInstaller') -Force
Copy-Item -LiteralPath (Join-Path $release 'installer') -Destination (Join-Path $OutputDir 'FFTAInstaller-SteamDeck') -Force
if (-not $InstallerOnly) {
Copy-Item -LiteralPath (Join-Path $release 'recomp') -Destination (Join-Path $payload 'recomp') -Force
Copy-Item -LiteralPath (Join-Path $release 'ffta-play') -Destination (Join-Path $payload 'FFTARecomp') -Force
Copy-Item -LiteralPath (Join-Path $repo 'gamedb.sqlite') -Destination (Join-Path $payload 'gamedb.sqlite') -Force
Copy-Item -LiteralPath (Join-Path $release 'tcc') -Destination $payload -Recurse -Force
}
# The launcher reads this from the top of both archives; the installer ignores it.
Copy-Item -LiteralPath (Join-Path $repo 'recomplauncher.json') -Destination (Join-Path $OutputDir 'recomplauncher.json') -Force
foreach ($required in @('recomp', 'FFTARecomp', 'gamedb.sqlite', 'tcc/libtcc.so')) {
    if (-not (Test-Path -LiteralPath (Join-Path $payload $required))) { throw "Missing payload file: $required. Build once without -InstallerOnly." }
}
$gnuTar = Join-Path (Split-Path (Split-Path (Split-Path $pkgConfig -Parent) -Parent) -Parent) 'usr\bin\tar.exe'
if (-not (Test-Path -LiteralPath $gnuTar)) { throw 'MSYS2 GNU tar is required to retain Linux executable permissions.' }
function ConvertTo-MsysPath([string]$Path) {
    $full = [IO.Path]::GetFullPath($Path)
    return '/' + $full.Substring(0, 1).ToLowerInvariant() + $full.Substring(2).Replace('\', '/')
}
$archive = Join-Path $OutputDir 'FFTAInstaller-linux-x86_64.tar.gz'
& $gnuTar -C (ConvertTo-MsysPath $OutputDir) --exclude=payload/toolchain --mode=a+rx -czf (ConvertTo-MsysPath $archive) FFTAInstaller recomplauncher.json payload/recomp payload/FFTARecomp payload/gamedb.sqlite payload/tcc/libtcc.so
if ($LASTEXITCODE -ne 0) { throw 'Could not package Linux installer' }
Write-Host "Linux installer: $archive" -ForegroundColor Green

# Build a separate play binary for the Steam Deck package. The ordinary Linux
# payload above stays on the default feature set and keeps its existing menu
# shortcuts. Only the Steam Deck archive gets the North/Y menu shortcut.
$oldErrorAction = $ErrorActionPreference
$ErrorActionPreference = 'Continue'
& cargo build --manifest-path (Join-Path $repo 'Cargo.toml') --release --target x86_64-unknown-linux-gnu -p recomp --features steamdeck
$deckCargoExit = $LASTEXITCODE
$ErrorActionPreference = $oldErrorAction
if ($deckCargoExit -ne 0) { throw 'Steam Deck game build failed' }

$deckStage = Join-Path $target ('steamdeck-package-' + [Guid]::NewGuid().ToString('N'))
$deckStagePayload = Join-Path $deckStage 'payload'
New-Item -ItemType Directory -Force -Path $deckStagePayload | Out-Null
Copy-Item -LiteralPath (Join-Path $OutputDir 'FFTAInstaller-SteamDeck') -Destination (Join-Path $deckStage 'FFTAInstaller-SteamDeck') -Force
Copy-Item -LiteralPath (Join-Path $OutputDir 'recomplauncher.json') -Destination (Join-Path $deckStage 'recomplauncher.json') -Force
Get-ChildItem -LiteralPath $payload -Force | Copy-Item -Destination $deckStagePayload -Recurse -Force
Copy-Item -LiteralPath (Join-Path $env:CARGO_TARGET_DIR 'x86_64-unknown-linux-gnu\release\recomp') -Destination (Join-Path $deckStagePayload 'recomp') -Force

$deckArchive = Join-Path $OutputDir 'FFTAInstaller-SteamDeck-linux-x86_64.tar.gz'
& $gnuTar -C (ConvertTo-MsysPath $deckStage) --exclude=payload/toolchain --mode=a+rx -czf (ConvertTo-MsysPath $deckArchive) FFTAInstaller-SteamDeck recomplauncher.json payload/recomp payload/FFTARecomp payload/gamedb.sqlite payload/tcc/libtcc.so
if ($LASTEXITCODE -ne 0) { throw 'Could not package Steam Deck installer' }
Write-Host "Steam Deck installer: $deckArchive" -ForegroundColor Green

$targetFull = [IO.Path]::GetFullPath($target).TrimEnd('\') + '\'
$stageFull = [IO.Path]::GetFullPath($deckStage)
if (-not $stageFull.StartsWith($targetFull, [StringComparison]::OrdinalIgnoreCase)) {
    throw "Refusing to remove staging outside target: $stageFull"
}
Remove-Item -LiteralPath $stageFull -Recurse -Force
