# agent.ps1 — the w2m desktop agent in the Windows VM's console session (scripts/vm_guest.py).
#
# Windows keeps windows, input and the screen in the interactive session; SSH logons and services
# run elsewhere (review 2026-10-09 local VM, L3). This script runs once per logon as the console
# user (scheduled task w2m-desktop, trigger AtLogOn), preloads what desktop work needs, and answers
# on 127.0.0.1:47001 inside the guest. Protocol, one line each way: the request is a PowerShell
# script as UTF-16 base64; the reply is "<exit code> <output as UTF-8 base64>". Errors are
# exceptions ($ErrorActionPreference Stop); a script that writes an error record fails with 1.
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
Add-Type -AssemblyName System.Windows.Forms, System.Drawing, UIAutomationClient, UIAutomationTypes
Add-Type -Name Dpi -Namespace W2M -MemberDefinition '[DllImport("user32.dll")] public static extern bool SetProcessDPIAware();'
[void][W2M.Dpi]::SetProcessDPIAware()

function Save-Screen([string]$Path) {
    $b = [System.Windows.Forms.SystemInformation]::VirtualScreen
    $bmp = New-Object System.Drawing.Bitmap $b.Width, $b.Height
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.CopyFromScreen($b.Left, $b.Top, 0, 0, $bmp.Size)
    $g.Dispose()
    $bmp.Save($Path, [System.Drawing.Imaging.ImageFormat]::Png)
    $bmp.Dispose()
}

# The screen as PNG, returned as "<sha256 hex> <base64>" so the caller needs no file and no copy.
function Get-ScreenPng {
    $b = [System.Windows.Forms.SystemInformation]::VirtualScreen
    $bmp = New-Object System.Drawing.Bitmap $b.Width, $b.Height
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.CopyFromScreen($b.Left, $b.Top, 0, 0, $bmp.Size)
    $g.Dispose()
    $ms = New-Object IO.MemoryStream
    $bmp.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
    $bmp.Dispose()
    $bytes = $ms.ToArray()
    $hash = [BitConverter]::ToString([Security.Cryptography.SHA256]::Create().ComputeHash($bytes)).Replace('-', '').ToLower()
    "$hash " + [Convert]::ToBase64String($bytes)
}

$listener = [Net.Sockets.TcpListener]::new([Net.IPAddress]::Loopback, 47001)
$listener.Start()
while ($true) {
    $client = $listener.AcceptTcpClient()
    $stream = $client.GetStream()
    $reader = New-Object IO.StreamReader($stream)
    $writer = New-Object IO.StreamWriter($stream)
    $writer.AutoFlush = $true
    try {
        while ($null -ne ($line = $reader.ReadLine())) {
            $code = 0
            $global:LASTEXITCODE = 0
            try {
                $script = [Text.Encoding]::Unicode.GetString([Convert]::FromBase64String($line))
                $out = & ([scriptblock]::Create($script)) *>&1 | Out-String -Width 4096
                if ($LASTEXITCODE) { $code = $LASTEXITCODE }
            } catch {
                $out = ($_ | Out-String -Width 4096)
                $code = 1
            }
            $writer.WriteLine("$code " + [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes([string]$out)))
        }
    } catch {
    } finally {
        $client.Close()
    }
}
