param(
    [string]$AndroidSdk = $env:ANDROID_SDK_ROOT,
    [switch]$SkipAndroid
)

# Android build tools are developer prerequisites, not part of the repository/payload.
# Install Android SDK + NDK 28.2.13676358, a JDK, Python, Rust's
# aarch64-linux-android target and a host GCC/Clang. Set JAVA_HOME/PATH as needed.
$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot
if (-not $SkipAndroid) {
    if (-not $AndroidSdk) { $AndroidSdk = $env:ANDROID_HOME }
    if (-not $AndroidSdk -or -not (Test-Path -LiteralPath ([IO.Path]::Combine($AndroidSdk, 'ndk/28.2.13676358/toolchains/llvm/prebuilt/windows-x86_64/bin')))) {
        throw 'Android SDK/NDK is required. Install NDK 28.2.13676358 and set ANDROID_SDK_ROOT or pass -AndroidSdk PATH. Also required: JDK, Python, GCC/Clang in PATH and rustup target add aarch64-linux-android. These tools are not bundled. Use -SkipAndroid for desktop builds only.'
    }
}

Write-Host '==> [1/5] Building Windows workspace...' -ForegroundColor Cyan
$oldErrorAction = $ErrorActionPreference
$ErrorActionPreference = 'Continue'
& cargo build --release -p installer -p recomp -p ffta-play
$cargoExit = $LASTEXITCODE
$ErrorActionPreference = $oldErrorAction
if ($cargoExit -ne 0) { throw 'Windows workspace build failed' }

Write-Host '==> [2/5] Copying Windows installer and game...' -ForegroundColor Cyan
New-Item -ItemType Directory -Force (Join-Path $PSScriptRoot 'Installer') | Out-Null
New-Item -ItemType Directory -Force (Join-Path $PSScriptRoot 'Installer\payload') | Out-Null
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'target\release\installer.exe') -Destination (Join-Path $PSScriptRoot 'Installer\Installer.exe') -Force
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'target\release\recomp.exe') -Destination (Join-Path $PSScriptRoot 'Installer\payload\recomp.exe') -Force
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'target\release\ffta-play.exe') -Destination (Join-Path $PSScriptRoot 'Installer\payload\FFTARecomp.exe') -Force
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'gamedb.sqlite') -Destination (Join-Path $PSScriptRoot 'Installer\payload\gamedb.sqlite') -Force

Write-Host '==> [3/5] Building compact TinyCC compiler...' -ForegroundColor Cyan
& python (Join-Path $PSScriptRoot 'TinyCC\build.py')
if ($LASTEXITCODE -ne 0) { throw 'TinyCC Windows build failed' }
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'target\release\tcc') -Destination (Join-Path $PSScriptRoot 'Installer\payload') -Recurse -Force
if (Test-Path -LiteralPath (Join-Path $PSScriptRoot 'Installer\payload\toolchain')) {
    Write-Warning 'Legacy payload/toolchain is no longer needed. Remove it before packaging the Windows release.'
}

Write-Host '==> [4/5] Building Linux x86_64 and Steam Deck installers...' -ForegroundColor Cyan
# Keep the Linux cross-compilation environment out of the Android build.
& (Join-Path $PSHOME 'powershell.exe') -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot 'scripts\build-linux-windows.ps1') -OutputDir (Join-Path $PSScriptRoot 'Installer\Linux')
if ($LASTEXITCODE -ne 0) { throw 'Linux installer build failed' }
if (-not $SkipAndroid) {
    Write-Host '==> [5/5] Building Android ARM64 APK (external SDK/NDK and JDK required)...' -ForegroundColor Cyan
    $previousTargetDir = $env:CARGO_TARGET_DIR
    $oldErrorAction = $ErrorActionPreference
    try {
        # AndroidPort's builder expects its own target/ directory.
        Remove-Item Env:\CARGO_TARGET_DIR -ErrorAction SilentlyContinue
        $ErrorActionPreference = 'Continue'
        & python (Join-Path $PSScriptRoot 'AndroidPort\scripts\build_android.py') --sdk $AndroidSdk
        $androidExit = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $oldErrorAction
        if ($null -eq $previousTargetDir) { Remove-Item Env:\CARGO_TARGET_DIR -ErrorAction SilentlyContinue }
        else { $env:CARGO_TARGET_DIR = $previousTargetDir }
    }
    if ($androidExit -ne 0) { throw 'Android APK build failed' }
    $androidOutput = Join-Path $PSScriptRoot 'Installer\Android'
    New-Item -ItemType Directory -Force $androidOutput | Out-Null
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'AndroidPort\APK\FFTARecomp-android-arm64-preview.apk') -Destination $androidOutput -Force
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'AndroidPort\APK\SHA256.txt') -Destination $androidOutput -Force
} else {
    Write-Host '==> [5/5] Android skipped (-SkipAndroid).'
}
$packageArguments = @((Join-Path $PSScriptRoot 'scripts\package_release_outputs.py'))
if ($SkipAndroid) { $packageArguments += '--skip-android' }
& python @packageArguments
if ($LASTEXITCODE -ne 0) { throw 'Release packaging failed' }
Write-Host "Done: $PSScriptRoot\Installer (Windows), Installer\Linux (Linux/Steam Deck), Installer\Android (APK when enabled)." -ForegroundColor Green
