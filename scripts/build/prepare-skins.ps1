param(
    [Parameter(Mandatory)][string]$Archive,
    [Parameter(Mandatory)][string]$LoaderDirectory,
    [Parameter(Mandatory)][string]$RetexSource,
    [Parameter(Mandatory)][string]$AsiLoaderSource
)
$ErrorActionPreference = 'Stop'
$repository = Split-Path (Split-Path $PSScriptRoot -Parent) -Parent
$manifest = Get-Content -LiteralPath (Join-Path $repository 'manifests/skins.json') -Raw | ConvertFrom-Json
$archivePath = (Resolve-Path -LiteralPath $Archive).Path
if ((Get-Item -LiteralPath $archivePath).Length -ne 593409812 -or (Get-FileHash -LiteralPath $archivePath -Algorithm SHA256).Hash -ne $manifest.archiveSha256) { throw 'Skin archive is not the reviewed version' }
$work = Join-Path (Split-Path $repository -Parent) ('Arachnid-GameData/research/mod-artifacts/skin-preparation-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $work -Force | Out-Null
$sevenZip = 'C:/Program Files/7-Zip/7z.exe'
& $sevenZip x $archivePath "-o$work" '-ir!*.tpf' '-ir!*.txt' '-ir!*.7z' '-ir!*.rar' -y
if ($LASTEXITCODE -ne 0) { throw 'Skin archive extraction failed' }
& $sevenZip x "$work/TASM 1 SKIN PACK/Hellfire Studios skins/TASM_1_PS4_Skin_by_kirillka.7z" "-o$work/kirillka" '-ir!*.tpf' -y
if ($LASTEXITCODE -ne 0) { throw 'Kirillka archive extraction failed' }
& $sevenZip x "$work/TASM 1 SKIN PACK/Mike_gens/TASM_PS4_Suit_V3_0.rar" "-o$work/mike" '-ir!*.tpf' -y
if ($LASTEXITCODE -ne 0) { throw 'Mike_gens archive extraction failed' }
$output = Join-Path $repository 'src-tauri/resources/skins-preview'
New-Item -ItemType Directory -Path $output -Force | Out-Null
function Publish-VerifiedFile($sourcePath, $descriptor) {
    $source = Get-Item -LiteralPath $sourcePath
    if ($source.Attributes -band [IO.FileAttributes]::ReparsePoint -or $source.Length -ne $descriptor.bytes -or (Get-FileHash -LiteralPath $source.FullName -Algorithm SHA256).Hash -ne $descriptor.sha256) { throw "Invalid source: $($descriptor.fileName)" }
    $destination = Join-Path $output $descriptor.fileName
    if (Test-Path -LiteralPath $destination) {
        if ((Get-FileHash -LiteralPath $destination -Algorithm SHA256).Hash -ne $descriptor.sha256) { throw "Existing modified file preserved: $destination" }
        return
    }
    Copy-Item -LiteralPath $source.FullName -Destination $destination
    if ((Get-FileHash -LiteralPath $destination -Algorithm SHA256).Hash -ne $descriptor.sha256) { throw 'Copy verification failed' }
}
foreach ($skin in $manifest.skins) { Publish-VerifiedFile (Join-Path $work $skin.archivePath) $skin }
foreach ($loader in $manifest.loaders) { Publish-VerifiedFile (Join-Path $LoaderDirectory $loader.fileName) $loader }
$notices = Join-Path $output 'notices'
New-Item -ItemType Directory -Path $notices -Force | Out-Null
Copy-Item -LiteralPath (Join-Path $RetexSource 'LICENSE-MIT') -Destination "$notices/Retex-MIT.txt"
Copy-Item -LiteralPath (Join-Path $RetexSource 'THIRD-PARTY-LICENSES.txt') -Destination "$notices/Retex-dependencies.txt"
Copy-Item -LiteralPath (Join-Path $AsiLoaderSource 'license') -Destination "$notices/Ultimate-ASI-Loader-MIT.txt"
Copy-Item -LiteralPath (Join-Path $AsiLoaderSource 'external/MemoryModule/LICENSE.txt') -Destination "$notices/MemoryModule-MPL-2.0.txt"
Copy-Item -LiteralPath (Join-Path $AsiLoaderSource 'external/d3d8to9/LICENSE.md') -Destination "$notices/d3d8to9-BSD.txt"
Copy-Item -LiteralPath (Join-Path $AsiLoaderSource 'external/injector/minhook/LICENSE.txt') -Destination "$notices/MinHook-BSD.txt"
Copy-Item -LiteralPath (Join-Path $AsiLoaderSource 'external/injector/license') -Destination "$notices/injector-license.txt"
Copy-Item -LiteralPath "$work/TASM 1 SKIN PACK/Readme.txt" -Destination "$notices/TexMod-readme.txt"
Write-Output "Prepared $($manifest.skins.Count) pinned packages locally."
