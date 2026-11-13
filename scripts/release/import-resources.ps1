[CmdletBinding()]
param(
    [Parameter(Mandatory)][uri]$Url,
    [Parameter(Mandatory)][ValidatePattern('\A[0-9a-fA-F]{64}\z')][string]$Sha256
)

$ErrorActionPreference = 'Stop'
if ($Url.Scheme -ne 'https' -or $Url.UserInfo) { throw 'Use an HTTPS resource archive URL.' }
$root = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '../..')).Path
$stage = Join-Path ([IO.Path]::GetTempPath()) "Arachnid-Resources-$([guid]::NewGuid())"
$null = New-Item -ItemType Directory -Path $stage
try {
    $archive = Join-Path $stage 'resources.zip'
    Invoke-WebRequest -Uri $Url -OutFile $archive -TimeoutSec 600
    if ((Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash -ne $Sha256) {
        throw 'The resource archive does not match its expected SHA-256.'
    }
    $extracted = Join-Path $stage 'extracted'
    [IO.Compression.ZipFile]::ExtractToDirectory($archive, $extracted)
    $folders = @('performance-preview-3', 'reshade-effects', 'skins-preview')
    $entries = @(Get-ChildItem -LiteralPath $extracted -Force)
    if ($entries.Count -ne $folders.Count -or
        @($entries | Where-Object { !$_.PSIsContainer -or $_.Name -cnotin $folders }).Count) {
        throw 'The archive must contain exactly the three prepared resource folders at its root.'
    }
    $destination = Join-Path $root 'src-tauri/resources'
    foreach ($folder in $folders) {
        if (Test-Path -LiteralPath (Join-Path $destination $folder)) {
            throw "Prepared resource folder already exists: $folder."
        }
    }
    foreach ($folder in $folders) {
        Move-Item -LiteralPath (Join-Path $extracted $folder) -Destination $destination
    }
} finally {
    if ((Get-Item -LiteralPath $stage).FullName -eq $stage) {
        Remove-Item -LiteralPath $stage -Recurse -Force
    }
}
