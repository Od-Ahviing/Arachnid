$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot '../../src-tauri/installer/tasm-prerequisites.ps1')

function Assert-Equal($Actual, $Expected) {
    if ($Actual -ne $Expected) {
        throw "Expected '$Expected', got '$Actual'."
    }
}

function Get-MicrosoftPackage($FileName, $DownloadPath, $Sha256) {
    $script:download = $FileName
    return $FileName
}

function Invoke-Setup($Path, $Arguments, [switch]$Elevated) {
    $script:setup = $Path
    $script:setupArguments = $Arguments
    $script:setupElevated = $Elevated.IsPresent
}

foreach ($windowsBuild in 19045, 26100, 27999) {
    $script:download = $null
    Install-DotNet35 $windowsBuild
    Assert-Equal $script:download $null
    Assert-Equal $script:setup (Join-Path $env:WINDIR 'System32\dism.exe')
    Assert-Equal $script:setupArguments '/Online /Enable-Feature /FeatureName:NetFx3 /All /Quiet /NoRestart'
    Assert-Equal $script:setupElevated $true
}

Install-DotNet35 28000
Assert-Equal $script:download 'DotNet35Setup.exe'
Assert-Equal $script:setup 'DotNet35Setup.exe'
Assert-Equal $script:setupElevated $true

$script:prerequisiteFailed = $false
$script:anyRestartRequired = $false
Invoke-Prerequisite 'Present' { $true } { throw 'Setup should not run.' }
Assert-Equal $script:prerequisiteFailed $false
Invoke-Prerequisite 'Restart' { $false } { $script:restartRequired = $true }
Assert-Equal $script:anyRestartRequired $true
Assert-Equal $script:prerequisiteFailed $false
Invoke-Prerequisite 'Missing after setup' { $false } {}
Assert-Equal $script:prerequisiteFailed $true
$script:nextSetupRan = $false
Invoke-Prerequisite 'Next component' { $script:nextSetupRan } { $script:nextSetupRan = $true }
Assert-Equal $script:nextSetupRan $true
