param([Parameter(Mandatory = $true)][string]$ReportDirectory)

$ErrorActionPreference = 'Stop'
$repository = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$executable = Join-Path $repository 'target/debug/examples/lifecycle-smoke.exe'
$reportRoot = [IO.Path]::GetFullPath($ReportDirectory)
if (Test-Path -LiteralPath $reportRoot) { throw 'Use a new report directory for each run.' }
New-Item -ItemType Directory -Path $reportRoot | Out-Null
$fixture = Start-Process -FilePath $executable -ArgumentList ('"' + $reportRoot + '"') -WindowStyle Hidden -PassThru -RedirectStandardError (Join-Path $reportRoot 'stderr.txt') -RedirectStandardOutput (Join-Path $reportRoot 'stdout.txt')
$samples = [Collections.Generic.List[object]]::new()
$deadline = [DateTime]::UtcNow.AddSeconds(55)
while (-not $fixture.HasExited -and [DateTime]::UtcNow -lt $deadline) {
    $phasePath = Join-Path $reportRoot 'phase.txt'
    $phase = if (Test-Path -LiteralPath $phasePath) { [IO.File]::ReadAllText($phasePath) } else { 'starting' }
    $processes = @(Get-CimInstance Win32_Process)
    $descendants = [Collections.Generic.HashSet[uint32]]::new()
    [void]$descendants.Add([uint32]$fixture.Id)
    do {
        $added = $false
        foreach ($process in $processes) {
            if ($descendants.Contains([uint32]$process.ParentProcessId) -and $descendants.Add([uint32]$process.ProcessId)) { $added = $true }
        }
    } while ($added)
    $webviews = @($processes | Where-Object { $descendants.Contains([uint32]$_.ProcessId) -and $_.Name -eq 'msedgewebview2.exe' })
    $hostProcess = Get-Process -Id $fixture.Id -ErrorAction SilentlyContinue
    $samples.Add([pscustomobject]@{
        time = [DateTime]::UtcNow.ToString('o')
        phase = $phase
        webviewProcesses = $webviews.Count
        webviewWorkingSetMiB = [math]::Round((($webviews | Measure-Object WorkingSetSize -Sum).Sum / 1MB), 2)
        hostWorkingSetMiB = [math]::Round(($hostProcess.WorkingSet64 / 1MB), 2)
    })
    Start-Sleep -Milliseconds 500
    $fixture.Refresh()
}
if (-not $fixture.HasExited) { $fixture.Kill(); throw 'The original fixture exceeded its deadline.' }
$samples | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $reportRoot 'resources.json')
$samples | Group-Object phase | ForEach-Object {
    [pscustomobject]@{
        phase = $_.Name
        samples = $_.Count
        webviewMin = ($_.Group.webviewProcesses | Measure-Object -Minimum).Minimum
        webviewMax = ($_.Group.webviewProcesses | Measure-Object -Maximum).Maximum
        webviewMiBMin = ($_.Group.webviewWorkingSetMiB | Measure-Object -Minimum).Minimum
        webviewMiBMax = ($_.Group.webviewWorkingSetMiB | Measure-Object -Maximum).Maximum
        hostMiBMin = ($_.Group.hostWorkingSetMiB | Measure-Object -Minimum).Minimum
        hostMiBMax = ($_.Group.hostWorkingSetMiB | Measure-Object -Maximum).Maximum
    }
} | Format-Table -AutoSize
$reportRoot
$failurePath = Join-Path $reportRoot 'failure.txt'
if (Test-Path -LiteralPath $failurePath) { throw [IO.File]::ReadAllText($failurePath) }
if (-not (Test-Path -LiteralPath (Join-Path $reportRoot 'phase.txt'))) { Get-Content -LiteralPath (Join-Path $reportRoot 'stderr.txt'); throw "Fixture failed before initialization: $($fixture.ExitCode)" }
$result = [IO.File]::ReadAllText((Join-Path $reportRoot 'phase.txt'))
if ($result -ne 'passed') { throw "Fixture did not complete: $result" }
'Lifecycle and background copy: passed'
