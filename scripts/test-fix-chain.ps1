$ErrorActionPreference = 'Stop'
$repository = Split-Path $PSScriptRoot -Parent
$developerShell = 'C:/Program Files/Microsoft Visual Studio/2022/Community/Common7/Tools/Launch-VsDevShell.ps1'
if (-not (Test-Path -LiteralPath $developerShell)) {
    throw 'The reviewed Visual Studio 2022 developer shell is unavailable'
}
& $developerShell -Arch x86 -HostArch amd64 -SkipAutomaticLocation
$output = Join-Path $repository 'target/native-fix-chain'
$present = Join-Path $output 'present'
$missing = Join-Path $output 'missing'
New-Item -ItemType Directory -Path $present, $missing -Force | Out-Null
$sources = Join-Path $repository 'tests/native/fix-chain'
Push-Location $output
try {
    & cl.exe /nologo /EHsc /W4 /WX /MT /LD (Join-Path $sources 'probe.cpp') /link "/DEF:$(Join-Path $sources 'probe.def')" "/OUT:$present/ar_dxvk.dll"
    if ($LASTEXITCODE -ne 0) { throw 'Original probe compilation failed' }
    & cl.exe /nologo /EHsc /W4 /WX /MT (Join-Path $sources 'path_harness.cpp') "/Fe:$present/path-harness.exe"
    if ($LASTEXITCODE -ne 0) { throw 'Original harness compilation failed' }
    Copy-Item -LiteralPath "$present/path-harness.exe" -Destination "$missing/path-harness.exe" -Force
    & "$present/path-harness.exe" present
    if ($LASTEXITCODE -ne 0) { throw "Present probe test failed: $LASTEXITCODE" }
    & "$missing/path-harness.exe" missing
    if ($LASTEXITCODE -ne 0) { throw "Missing probe test failed: $LASTEXITCODE" }
    & cl.exe /nologo /EHsc /W4 /WX /MT (Join-Path $sources 'log_path_harness.cpp') "/Fe:$output/log-path-harness.exe"
    if ($LASTEXITCODE -ne 0) { throw 'Original logging harness compilation failed' }
    & "$output/log-path-harness.exe"
    if ($LASTEXITCODE -ne 0) { throw "Logging path test failed: $LASTEXITCODE" }
} finally {
    Pop-Location
}
