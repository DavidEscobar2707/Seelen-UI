param([int]$Seconds = 60, [string]$Label = "run", [string]$Out = "")
# Measures Seelen (seelen-ui, slu-service, slu) + the msedgewebview2 processes that belong to Seelen
# (matched by --user-data-dir under the Seelen cache dir). CPU from perf raw counters (works for elevated procs).
$ErrorActionPreference = "SilentlyContinue"
$cores = (Get-CimInstance Win32_Processor | Measure-Object NumberOfLogicalProcessors -Sum).Sum
function Get-SeelenPids {
  $all = Get-CimInstance Win32_Process
  $own = $all | Where-Object { $_.Name -match '^(seelen-ui|slu-service|slu)\.exe$' }
  $wv  = $all | Where-Object { $_.Name -eq 'msedgewebview2.exe' -and $_.CommandLine -match 'seelen' }
  @($own) + @($wv)
}
function Snap {
  $procs = Get-SeelenPids
  $raw = Get-CimInstance Win32_PerfRawData_PerfProc_Process | Where-Object { $_.IDProcess -in $procs.ProcessId }
  [pscustomobject]@{ Procs = $procs; Raw = $raw; T = [datetime]::UtcNow }
}
$a = Snap
Start-Sleep -Seconds $Seconds
$b = Snap
$elapsed100ns = ($b.T - $a.T).Ticks
$rows = foreach ($p in $b.Procs) {
  $r1 = $a.Raw | Where-Object IDProcess -eq $p.ProcessId | Select-Object -First 1
  $r2 = $b.Raw | Where-Object IDProcess -eq $p.ProcessId | Select-Object -First 1
  $cpu = $null
  if ($r1 -and $r2) { $cpu = [math]::Round(100.0 * ($r2.PercentProcessorTime - $r1.PercentProcessorTime) / $elapsed100ns / $cores, 3) }
  $type = if ($p.CommandLine -match '--type=([a-z-]+)') { $Matches[1] } else { 'main' }
  if ($p.CommandLine -match '--utility-sub-type=([\w.]+)') { $type += ':' + $Matches[1] }
  [pscustomobject]@{
    Name = $p.Name; Pid = $p.ProcessId; Type = $type
    WS_MB = [math]::Round($r2.WorkingSet / 1MB, 1)
    Private_MB = [math]::Round($r2.PrivateBytes / 1MB, 1)
    CPU_pct = $cpu
  }
}
$rows | Sort-Object Name, Type | Format-Table -AutoSize | Out-String -Width 200 | Write-Output
$sumWS = [math]::Round(($rows | Measure-Object WS_MB -Sum).Sum, 1)
$sumPriv = [math]::Round(($rows | Measure-Object Private_MB -Sum).Sum, 1)
$sumCpu = [math]::Round(($rows | Measure-Object CPU_pct -Sum).Sum, 3)
$bat = Get-CimInstance -Namespace root\wmi -ClassName BatteryStatus | Select-Object -First 1
$summary = [pscustomobject]@{
  Label = $Label; When = (Get-Date).ToString("s"); WindowSec = $Seconds; Processes = @($rows).Count
  WebViewProcesses = @($rows | Where-Object Name -eq 'msedgewebview2.exe').Count
  Sum_WorkingSet_MB = $sumWS; Sum_Private_MB = $sumPriv; Sum_CPU_pct_of_machine = $sumCpu
  OnBattery = [bool]$bat.Discharging; DischargeRate_mW = $bat.DischargeRate
}
$summary | Format-List | Out-String | Write-Output
if ($Out) { @{ summary = $summary; rows = $rows } | ConvertTo-Json -Depth 4 | Set-Content -Encoding utf8 $Out }
