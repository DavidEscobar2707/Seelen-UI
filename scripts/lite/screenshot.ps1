Add-Type -AssemblyName System.Windows.Forms,System.Drawing
Add-Type 'using System;using System.Runtime.InteropServices;public class D{[DllImport("user32.dll")]public static extern bool SetProcessDPIAware();}'
[D]::SetProcessDPIAware() | Out-Null
$b=[System.Windows.Forms.Screen]::PrimaryScreen.Bounds
$bmp=New-Object System.Drawing.Bitmap $b.Width,$b.Height
$g=[System.Drawing.Graphics]::FromImage($bmp);$g.CopyFromScreen($b.Location,[System.Drawing.Point]::Empty,$b.Size)
$bmp.Save($args[0],[System.Drawing.Imaging.ImageFormat]::Png)