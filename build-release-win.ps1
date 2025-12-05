# This script builds both the Tauri GUI application and CLI tool.
#
# Usage:
#   .\build-release.ps1                    # Build both app and CLI
#   .\build-release.ps1 -AppOnly           # Build only the Tauri app
#   .\build-release.ps1 -CliOnly           # Build only the CLI
#   .\build-release.ps1 -SkipBuild         # Package existing artifacts only

param(
    [Parameter(Mandatory = $false)]
    [string]$OutputDir = "$PSScriptRoot\release",
    
    [Parameter(Mandatory = $false)]
    [switch]$SkipInstall = $false,
    
    [Parameter(Mandatory = $false)]
    [switch]$SkipBuild = $false,
    
    [Parameter(Mandatory = $false)]
    [switch]$AppOnly = $false,
    
    [Parameter(Mandatory = $false)]
    [switch]$CliOnly = $false,
    
    [Parameter(Mandatory = $false)]
    [switch]$Help
)

if ($Help) {
    Write-Host @"
Lighthouse Manager - Release Build Script

USAGE:
    .\build-release.ps1 [OPTIONS]

OPTIONS:
    -OutputDir <path>     Output directory for release artifacts (default: ./release)
    -SkipInstall          Skip pnpm install step
    -SkipBuild            Skip build steps (package existing artifacts only)
    -AppOnly              Build only the Tauri application
    -CliOnly              Build only the CLI
    -Help                 Show this help message

OUTPUTS:
    release/Lighthouse Manager.exe    GUI application
    release/lighthouse-manager.exe    CLI tool

"@
    exit 0
}

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Write-Step {
    param([string]$Message)
    Write-Host ""
    Write-Host "=== $Message ===" -ForegroundColor Cyan
}

function Write-Success {
    param([string]$Message)
    Write-Host $Message -ForegroundColor Green
}

function Write-Info {
    param([string]$Message)
    Write-Host $Message -ForegroundColor Yellow
}

try {
    Write-Host ""
    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host " Lighthouse Manager Release Build" -ForegroundColor Cyan
    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host ""
    
    $buildApp = -not $CliOnly
    $buildCli = -not $AppOnly
    
    # Build CLI
    if ($buildCli -and -not $SkipBuild) {
        Write-Step "Building CLI (lighthouse-manager)"
        Set-Location -Path $PSScriptRoot
        cargo build --release --package lighthouse-rs
        if ($LASTEXITCODE -ne 0) {
            throw "CLI build failed"
        }
        Write-Success "CLI build completed"
    }
    
    # Build Tauri App
    if ($buildApp) {
        $appDir = "$PSScriptRoot\lighthouse_app"
        Set-Location -Path $appDir
        
        if (-not $SkipInstall) {
            Write-Step "Installing frontend dependencies"
            pnpm install
            if ($LASTEXITCODE -ne 0) {
                throw "pnpm install failed"
            }
            Write-Success "Dependencies installed"
        }
        
        if (-not $SkipBuild) {
            Write-Step "Building Tauri application"
            pnpm tauri build
            if ($LASTEXITCODE -ne 0) {
                throw "Tauri build failed"
            }
            Write-Success "Tauri build completed"
        }
    }
    
    # Create release directory
    Write-Step "Packaging release artifacts"
    if (Test-Path $OutputDir) {
        Remove-Item -Path $OutputDir -Recurse -Force
    }
    New-Item -ItemType Directory -Path $OutputDir | Out-Null
    
    $artifacts = @()
    
    # Copy CLI executable
    if ($buildCli) {
        $cliPath = "$PSScriptRoot\target\release\lighthouse-rs.exe"
        if (Test-Path $cliPath) {
            Copy-Item -Path $cliPath -Destination "$OutputDir\lighthouse-manager.exe"
            $artifacts += "lighthouse-manager.exe"
            Write-Success "Copied lighthouse-manager.exe"
        }
    }
    
    # Copy App executable
    if ($buildApp) {
        $appPath = "$PSScriptRoot\target\release\lighthouse_app.exe"
        if (Test-Path $appPath) {
            Copy-Item -Path $appPath -Destination "$OutputDir\Lighthouse Manager.exe"
            $artifacts += "Lighthouse Manager.exe"
            Write-Success "Copied Lighthouse Manager.exe"
        }
    }
    
    if ($artifacts.Count -eq 0) {
        throw "No artifacts found. Build first or remove -SkipBuild flag."
    }
    
    # Summary
    Write-Step "Build complete!"
    Write-Host "Output: $OutputDir" -ForegroundColor Cyan
    Write-Host ""
    Get-ChildItem -Path $OutputDir | ForEach-Object {
        $size = "{0:N2} MB" -f ($_.Length / 1MB)
        Write-Host "  - $($_.Name) ($size)" -ForegroundColor White
    }
    
    exit 0
}
catch {
    Write-Host "ERROR: $($_.Exception.Message)" -ForegroundColor Red
    exit 1
}
finally {
    Set-Location -Path $PSScriptRoot
}
