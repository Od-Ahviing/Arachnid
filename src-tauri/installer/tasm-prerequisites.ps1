$ErrorActionPreference = 'Stop'

function Assert-MicrosoftSignature($Path) {
    $signature = Get-AuthenticodeSignature -LiteralPath $Path
    if ($signature.Status -ne 'Valid' -or
        $signature.SignerCertificate.GetNameInfo('SimpleName', $false) -ne 'Microsoft Corporation') {
        throw "Invalid Microsoft signature for $(Split-Path -Leaf $Path)."
    }
}

function Get-MicrosoftPackage($FileName, $DownloadPath, $Sha256) {
    $path = Join-Path $stage $FileName
    Write-Host "Downloading $FileName..."
    Invoke-WebRequest -Uri "https://download.microsoft.com/download/$DownloadPath/$FileName" -OutFile $path -UseBasicParsing -MaximumRedirection 0 -TimeoutSec 300
    if ((Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash -ne $Sha256) {
        throw "Unexpected SHA-256 for $FileName."
    }
    Assert-MicrosoftSignature $path
    return $path
}

function Invoke-Setup($Path, $Arguments, [switch]$Elevated) {
    $parameters = @{
        FilePath = $Path
        ArgumentList = $Arguments
        WorkingDirectory = Split-Path -Parent $Path
        Wait = $true
        PassThru = $true
        WindowStyle = 'Hidden'
    }
    if ($Elevated) {
        $parameters.Verb = 'RunAs'
    }
    $result = Start-Process @parameters
    if ($result.ExitCode -notin 0, 3010, 1641) {
        throw "$(Split-Path -Leaf $Path) exited with code $($result.ExitCode)."
    }
    if ($result.ExitCode -in 3010, 1641) {
        $script:restartRequired = $true
    }
}

function Test-DirectX {
    $systemX86 = if ([Environment]::Is64BitOperatingSystem) { 'SysWOW64' } else { 'System32' }
    $library = Join-Path $env:WINDIR "$systemX86\d3dx9_43.dll"
    return (Test-Path -LiteralPath $library -PathType Leaf)
}

function Test-VisualCpp {
    $assemblies = Get-ChildItem -LiteralPath (Join-Path $env:WINDIR 'WinSxS') -Directory -Filter 'x86_microsoft.vc90.crt_1fc8b3b9a1e18e3b_*'
    foreach ($assembly in $assemblies) {
        $version = [version]($assembly.Name.Split('_')[3])
        if ($version -ge [version]'9.0.30729.6161' -and
            (Test-Path -LiteralPath (Join-Path $assembly.FullName 'msvcr90.dll') -PathType Leaf) -and
            (Test-Path -LiteralPath (Join-Path $assembly.FullName 'msvcp90.dll') -PathType Leaf)) {
            return $true
        }
    }
    return $false
}

function Test-DotNet35 {
    $registry = [Microsoft.Win32.RegistryKey]::OpenBaseKey('LocalMachine', 'Registry32')
    try {
        $key = $registry.OpenSubKey('SOFTWARE\Microsoft\NET Framework Setup\NDP\v3.5')
        if ($null -eq $key) { return $false }
        try {
            return ($key.GetValue('Install') -eq 1)
        } finally {
            $key.Dispose()
        }
    } finally {
        $registry.Dispose()
    }
}

function Install-DirectX {
    $package = Get-MicrosoftPackage 'directx_Jun2010_redist.exe' '8/4/a/84a35bf1-dafe-4ae8-82af-ad2ae20b6b14' '053F76DCBB28802E23341B6A787E3B0791C0FA5C8D4D011B1044172DBF89C73B'
    $extract = Join-Path $stage 'DirectX'
    New-Item -ItemType Directory -Path $extract | Out-Null
    Invoke-Setup $package ('/Q /T:"{0}"' -f $extract)
    $setup = Join-Path $extract 'DXSETUP.exe'
    Assert-MicrosoftSignature $setup
    Invoke-Setup $setup '/silent' -Elevated
}

function Install-VisualCpp {
    $package = Get-MicrosoftPackage 'vcredist_x86.exe' '5/d/8/5d8c65cb-c849-4025-8e95-c396cafd8ae' '8742BCBF24EF328A72D2A27B693CC7071E38D3BB4B9B44DEC42AA3D2C8D61D92'
    Invoke-Setup $package '/q:a /r:n' -Elevated
}

function Install-DotNet35($WindowsBuild) {
    if ($WindowsBuild -lt 28000) {
        $dism = Join-Path $env:WINDIR 'System32\dism.exe'
        Invoke-Setup $dism '/Online /Enable-Feature /FeatureName:NetFx3 /All /Quiet /NoRestart' -Elevated
        return
    }

    $package = Get-MicrosoftPackage 'DotNet35Setup.exe' '298af580-974e-4001-a07d-bb200d7c5792' 'B969D1D0A662BDFD20F42AED9F0EDE21122723FBBCEE545F16DA7150FF01926D'
    Invoke-Setup $package '/quiet /norestart' -Elevated
}

function Invoke-Prerequisite($Name, [scriptblock]$IsInstalled, [scriptblock]$Install) {
    try {
        if (& $IsInstalled) {
            Write-Output "$Name is already installed."
            return
        }
        Write-Output "Installing $Name..."
        $script:restartRequired = $false
        & $Install
        if ($script:restartRequired) {
            $script:anyRestartRequired = $true
            Write-Output "$Name requires a restart."
        } elseif (& $IsInstalled) {
            Write-Output "$Name is installed."
        } else {
            throw 'The component was not detected after setup.'
        }
    } catch {
        $script:prerequisiteFailed = $true
        Write-Output "Could not install $Name. $($_.Exception.Message)"
    }
}

if ($MyInvocation.InvocationName -ne '.') {
    $tempRoot = (Resolve-Path -LiteralPath ([IO.Path]::GetTempPath())).ProviderPath
    $stage = Join-Path $tempRoot ('Arachnid-Prerequisites-' + [guid]::NewGuid())
    $stageCreated = $false
    $script:anyRestartRequired = $false
    $script:prerequisiteFailed = $false
    try {
        New-Item -ItemType Directory -Path $stage | Out-Null
        $stageCreated = $true
        [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
        $windowsBuild = [int](Get-ItemPropertyValue 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion' 'CurrentBuildNumber')
        Invoke-Prerequisite 'DirectX June 2010 (D3DX9 x86)' { Test-DirectX } { Install-DirectX }
        Invoke-Prerequisite 'Visual C++ 2008 SP1 x86' { Test-VisualCpp } { Install-VisualCpp }
        Invoke-Prerequisite '.NET Framework 3.5' { Test-DotNet35 } { Install-DotNet35 $windowsBuild }
    } catch {
        $script:prerequisiteFailed = $true
        Write-Output $_.Exception.Message
    } finally {
        $stageItem = if ($stageCreated) { Get-Item -LiteralPath $stage -Force -ErrorAction SilentlyContinue }
        if ($stageItem -and $stageItem.FullName -eq $stage -and
            ($stageItem.Attributes -band [IO.FileAttributes]::ReparsePoint) -eq 0) {
            Remove-Item -LiteralPath $stage -Recurse -Force -ErrorAction SilentlyContinue
        }
    }

    if ($script:anyRestartRequired -and $script:prerequisiteFailed) { exit 3011 }
    if ($script:anyRestartRequired) { exit 3010 }
    if ($script:prerequisiteFailed) { exit 1 }
    exit 0
}
