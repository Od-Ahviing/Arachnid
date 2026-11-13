param(
    [string]$Compiler = 'C:/Program Files (x86)/Windows Kits/10/bin/10.0.26100.0/x64/fxc.exe'
)
$ErrorActionPreference = 'Stop'
if (-not (Test-Path -LiteralPath $Compiler)) { throw 'Windows SDK HLSL compiler not found' }
$repository = Split-Path $PSScriptRoot -Parent
$outputDirectory = Join-Path $repository 'target/color-shader-tests'
New-Item -ItemType Directory -Path $outputDirectory -Force | Out-Null

function Read-VertexShader([string]$Path, [bool]$LegacyProfile) {
    $source = Get-Content -LiteralPath $Path -Raw
    $match = [regex]::Match($source, '(?s)void FullscreenVertex\(.*?\n\}')
    if (-not $match.Success) { throw 'Fullscreen vertex entry point not found' }
    $vertex = $match.Value
    if ($LegacyProfile) {
        # ReShade 6.6.2 maps these semantics for Shader Model 3.
        $vertex = $vertex.Replace('SV_VertexID', 'TEXCOORD0').Replace('SV_Position', 'POSITION')
    }
    return $vertex
}

$oldPath = Join-Path $outputDirectory 'legacy.hlsl'
Read-VertexShader (Join-Path $repository 'tests/fixtures/shaders/ArachnidColor.v1.fx') $true | Set-Content -LiteralPath $oldPath -Encoding utf8
$diagnostics = & $Compiler /nologo /T vs_3_0 /E FullscreenVertex /Fo (Join-Path $outputDirectory 'legacy.cso') $oldPath 2>&1
if ($LASTEXITCODE -eq 0 -or ($diagnostics -join "`n") -notmatch 'X3535') { throw 'The original SM3 bitwise failure was not reproduced' }
Write-Output 'Original vertex shader: reproduced X3535 for vs_3_0.'

foreach ($profile in @('vs_3_0', 'vs_5_0')) {
    $path = Join-Path $outputDirectory "$profile.hlsl"
    Read-VertexShader (Join-Path $repository 'src-tauri/resources/shaders/ArachnidColor.fx') ($profile -eq 'vs_3_0') | Set-Content -LiteralPath $path -Encoding utf8
    & $Compiler /nologo /WX /T $profile /E FullscreenVertex /Fo (Join-Path $outputDirectory "$profile.cso") $path
    if ($LASTEXITCODE -ne 0) { throw "Corrected vertex shader failed: $profile" }
}
Write-Output 'Corrected vertex shader: SM3 and SM5 compilation passed. ReShade effect loading still requires an in-game check.'
