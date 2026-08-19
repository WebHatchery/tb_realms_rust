# RustGames project itch.io publisher wrapper.
# Configuration lives in itch.json; shared behavior lives in the workspace
# rust_management/publish-itch.ps1 script.

param(
    [ValidateSet("all", "html5", "windows")]
    [string]$Channel = "all",
    [string]$ButlerPath = "",
    [string]$UserVersion = "",
    [switch]$Preview,
    [switch]$Status,
    [switch]$DryRun,
    [switch]$Help
)

$ErrorActionPreference = "Stop"
$rootPublisher = Join-Path (Split-Path $PSScriptRoot -Parent) "publish-itch.ps1"
if (-not (Test-Path $rootPublisher)) {
    Write-Error "RustGames itch publisher not found: $rootPublisher"
    exit 1
}

& $rootPublisher -ProjectDir $PSScriptRoot @PSBoundParameters
if (-not $?) { exit 1 }

