Add-Type @"
using System;using System.Runtime.InteropServices;using System.Text;
public class W{
[DllImport("user32.dll")]public static extern bool SetProcessDPIAware();
[DllImport("user32.dll")]public static extern bool SetCursorPos(int x,int y);
[DllImport("user32.dll")]public static extern void mouse_event(int f,int x,int y,int d,int e);
[DllImport("user32.dll")]public static extern void keybd_event(byte k,byte s,int f,int e);
[DllImport("user32.dll")]public static extern IntPtr FindWindow(string c,string t);
[DllImport("user32.dll")]public static extern bool IsWindowVisible(IntPtr h);
[DllImport("user32.dll")]public static extern int GetSystemMetrics(int i);
}
"@
[W]::SetProcessDPIAware()|Out-Null
function TaskbarVisible { [W]::IsWindowVisible([W]::FindWindow('Shell_TrayWnd',$null)) }
function RClick($x,$y){ [W]::SetCursorPos($x,$y); Start-Sleep -m 200; [W]::mouse_event(8,0,0,0,0); [W]::mouse_event(16,0,0,0,0) }
function Esc { [W]::keybd_event(0x1B,0,0,0); [W]::keybd_event(0x1B,0,2,0) }
function SeelenWv { (Get-CimInstance Win32_Process -Filter "Name='msedgewebview2.exe'" | ? CommandLine -match 'seelen' | measure).Count }
