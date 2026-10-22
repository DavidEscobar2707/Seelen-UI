param([string]$Label,[int]$Sec=120)
$v=@(); $end=(Get-Date).AddSeconds($Sec)
while((Get-Date) -lt $end){ $b=Get-CimInstance -Namespace root\wmi -ClassName BatteryStatus|select -First 1; if($b.Discharging){$v+=$b.DischargeRate}; Start-Sleep 5 }
$s=$v|Measure-Object -Average -Minimum -Maximum
"{0}: n={1} avg={2:N0} mW min={3} max={4}" -f $Label,$s.Count,$s.Average,$s.Minimum,$s.Maximum