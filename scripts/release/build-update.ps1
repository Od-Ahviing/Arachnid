[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$Tag
)

$ErrorActionPreference = 'Stop'
$root = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '../..')).Path
Push-Location $root
try {
    $package = Get-Content -Raw package.json | ConvertFrom-Json
    $tauri = Get-Content -Raw src-tauri/tauri.conf.json | ConvertFrom-Json
    $workspace = Get-Content -Raw Cargo.toml
    $version = $package.version
    if ($version -notmatch '\A(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)\z' -or
        $Tag -cne "v$version" -or $tauri.version -cne $version -or
        $workspace -notmatch ('(?m)^version = "' + [regex]::Escape($version) + '"\r?$')) {
        throw 'The stable tag and all project versions must match.'
    }
    if ([string]::IsNullOrWhiteSpace($env:ARACHNID_UPDATER_PUBLIC_KEY) -or
        [string]::IsNullOrWhiteSpace($env:TAURI_SIGNING_PRIVATE_KEY)) {
        throw 'Configure the updater public key and Tauri signing secret before building.'
    }

    $repoUrl = 'https://github.com/Od-Ahviing/Arachnid'
    $output = Join-Path $root 'target/x86_64-pc-windows-msvc/release/bundle/nsis'
    $installerName = "Arachnid_${version}_x64-setup.exe"
    $signature = Join-Path $output "$installerName.sig"
    $configPath = Join-Path $root "target/update-config-$([guid]::NewGuid()).json"
    $null = New-Item -ItemType Directory -Force (Split-Path $configPath)
    @{
        bundle = @{ createUpdaterArtifacts = $true }
        plugins = @{
            updater = @{
                pubkey = $env:ARACHNID_UPDATER_PUBLIC_KEY.Trim()
                endpoints = @("$repoUrl/releases/latest/download/latest.json")
                requireSignedVersion = $true
            }
        }
    } | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $configPath -Encoding utf8NoBOM

    try {
        pnpm tauri build --ci --target x86_64-pc-windows-msvc --config src-tauri/tauri.offline.conf.json --config $configPath -- --locked
        if ($LASTEXITCODE -ne 0) { throw "Tauri build failed with exit code $LASTEXITCODE." }
    } finally {
        Remove-Item -LiteralPath $configPath
    }

    $signatureContent = (Get-Content -Raw -LiteralPath $signature).Trim()
    if (!$signatureContent) { throw 'Tauri generated an empty installer signature.' }
    @{
        version = $version
        platforms = @{
            'windows-x86_64' = @{
                signature = $signatureContent
                url = "$repoUrl/releases/download/$Tag/$installerName"
            }
        }
    } | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $output 'latest.json') -Encoding utf8NoBOM
} finally {
    Pop-Location
}
