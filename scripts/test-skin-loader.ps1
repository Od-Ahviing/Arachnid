param([Parameter(Mandatory)][string]$LoaderDirectory)
$ErrorActionPreference = 'Stop'
$repository = Split-Path $PSScriptRoot -Parent
$output = Join-Path $repository 'target/native-skins'
$loader = (Resolve-Path -LiteralPath $LoaderDirectory).Path
$manifest = Get-Content -LiteralPath (Join-Path $repository 'manifests/skins.json') -Raw | ConvertFrom-Json
foreach ($file in $manifest.loaders) {
    $path = Join-Path $loader $file.fileName
    if ((Get-Item -LiteralPath $path).Length -ne $file.bytes -or (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash -ne $file.sha256) { throw 'Loader does not match the reviewed manifest' }
}
New-Item -ItemType Directory -Path $output, "$output/scripts", "$output/plugins" -Force | Out-Null
Copy-Item -LiteralPath "$loader/dinput8.dll", "$loader/arachnid_skin.asi" -Destination $output -Force
Copy-Item -LiteralPath "$loader/arachnid_skin.asi" -Destination "$output/dinput8Hooked.dll" -Force
foreach ($folder in @($output, "$output/scripts", "$output/plugins")) {
    Copy-Item -LiteralPath "$loader/arachnid_skin.asi" -Destination "$folder/unknown.asi" -Force
}
[IO.File]::WriteAllText("$output/retex.ini", '')
[IO.File]::WriteAllText("$output/global.ini", "[globalsets]`nloadplugins=0`ndontloadfromdllmain=0`nforceentrypointhook=1`n[fileloader]`noverloadfromfolder=plugins`n")
& 'C:/Program Files/Microsoft Visual Studio/2022/Community/Common7/Tools/Launch-VsDevShell.ps1' -Arch x86 -HostArch amd64 -SkipAutomaticLocation
Push-Location $output
try {
    & cl.exe /nologo /EHsc /W4 /WX /MT (Join-Path $repository 'tests/native/skins/loader_harness.cpp') "/Fe:$output/loader-harness.exe" /link dinput8.lib dxguid.lib d3d9.lib '/MANIFEST:EMBED' '/MANIFESTDEPENDENCY:type=''win32'' name=''Microsoft.Windows.Common-Controls'' version=''6.0.0.0'' processorArchitecture=''*'' publicKeyToken=''6595b64144ccf1df'' language=''*'''
    if ($LASTEXITCODE -ne 0) { throw 'Original skin harness compilation failed' }
    $env:RETEX_CONFIG = "$output/retex.ini"
    & "$output/loader-harness.exe"
    if ($LASTEXITCODE -ne 0) { throw "Skin loader test failed: $LASTEXITCODE" }
} finally {
    Remove-Item Env:RETEX_CONFIG -ErrorAction SilentlyContinue
    Pop-Location
}
