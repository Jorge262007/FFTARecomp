# Requires Docker Desktop using Linux containers; Windows PowerShell 5.1+.
[CmdletBinding()]
param(
    [ValidateSet('windows', 'linux')]
    [string]$Platform = 'windows',
    [string]$OutputDir
)
$ErrorActionPreference = 'Stop'
$Platform = $Platform.ToLowerInvariant()
$repoDir = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
if (-not $OutputDir) { $OutputDir = Join-Path $repoDir "dist/docker-$Platform" }
if ($OutputDir.Contains(',') -or $OutputDir.Contains("`n") -or $OutputDir.Contains("`r")) {
    throw 'The output directory must not contain commas or newlines.'
}
$exportDir = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($OutputDir)
if (-not (Get-Command docker -CommandType Application -ErrorAction SilentlyContinue)) {
    throw 'Docker Desktop with Buildx is required. Start Docker Desktop using Linux containers.'
}
New-Item -ItemType Directory -Force -Path $exportDir | Out-Null
$dockerArgs = @(
    'buildx', 'build',
    '--platform', 'linux/amd64',
    '--file', (Join-Path $repoDir "Dockerfile.$Platform"),
    '--target', 'export',
    '--output', "type=local,dest=$exportDir",
    '--progress', 'plain',
    $repoDir
)
& docker @dockerArgs
if ($LASTEXITCODE -ne 0) { throw "Docker $Platform build failed (exit $LASTEXITCODE)." }
Write-Host "Portable archive and BUILD-INFO.txt exported to $exportDir"
