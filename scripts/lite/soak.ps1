param([int]$Minutes=60)
$out='C:\Users\USUARIO\Desktop\seelen-lite-wt\docs\soak-phase2.csv'
$exe='C:\Users\USUARIO\Desktop\seelen-lite-wt\target\release\seelen-ui.exe'
$app=Start-Process -FilePath $exe -WorkingDirectory (Split-Path $exe) -PassThru
Start-Sleep 60
"time,seelen_ws_mb,seelen_priv_mb,webview_ws_mb,webview_priv_mb,procs,cpu_pct_5min" | Set-Content $out
$cores=(Get-CimInstance Win32_Processor | Measure-Object NumberOfLogicalProcessors -Sum).Sum
$prev=$null; $prevT=$null
for($i=0;$i -le ($Minutes/5);$i++){
  if($app.HasExited){ "{0},EXITED code={1}" -f (Get-Date -f HH:mm),$app.ExitCode | Add-Content $out; break }
  $all=Get-CimInstance Win32_Process
  $ids=@($all | ? { $_.Name -eq 'seelen-ui.exe' -or ($_.Name -eq 'msedgewebview2.exe' -and $_.CommandLine -match 'seelen') })
  $raw=Get-CimInstance Win32_PerfRawData_PerfProc_Process | ? { $_.IDProcess -in $ids.ProcessId }
  $main=$raw | ? { $_.IDProcess -eq $app.Id }; $wv=$raw | ? { $_.IDProcess -ne $app.Id }
  $cpuT=($raw | measure PercentProcessorTime -Sum).Sum; $now=[datetime]::UtcNow; $cpu=''
  if($prev -ne $null){ $cpu=[math]::Round(100*($cpuT-$prev)/($now-$prevT).Ticks/$cores,3) }
  $prev=$cpuT; $prevT=$now
  "{0},{1:N1},{2:N1},{3:N1},{4:N1},{5},{6}" -f (Get-Date -f HH:mm),($main.WorkingSet/1MB),($main.PrivateBytes/1MB),(($wv|measure WorkingSet -Sum).Sum/1MB),(($wv|measure PrivateBytes -Sum).Sum/1MB),@($raw).Count,$cpu | Add-Content $out
  if($i -lt ($Minutes/5)){ for($s=0;$s -lt 300 -and -not $app.HasExited;$s+=5){ Start-Sleep 5 } }
}