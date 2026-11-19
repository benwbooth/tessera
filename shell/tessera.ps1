# Tessera shell integration for PowerShell: prompt and command marks
# (OSC 133), the working directory (OSC 7), the command line (OSC 1337
# TesseraCommand), the exported environment at each prompt (for Tessera's
# kernels and prompt), and what another shell handed over (folder, exports).
# (Loaded with `pwsh -NoExit -Command ". tessera.ps1"`, after the profile.)
if (-not $global:__TesseraLoaded) {
    $global:__TesseraLoaded = $true
    $global:__TesseraRunning = $false
    $global:__TesseraPrompt = $function:prompt

    function global:prompt {
        $ok = $?
        $code = if ($ok) { 0 } elseif ($LASTEXITCODE) { $LASTEXITCODE } else { 1 }
        # (Set-Location doesn't move the process: it should, for Tessera to
        # see the folder, and for the programs it starts.)
        try { [Environment]::CurrentDirectory = (Get-Location -PSProvider FileSystem).ProviderPath } catch {}
        $base = if ($env:XDG_RUNTIME_DIR) { $env:XDG_RUNTIME_DIR } else { '/tmp' }
        $d = Join-Path $base 'tessera-env'
        $f = Join-Path $d "$PID.apply"
        if (Test-Path -LiteralPath $f) {
            try { . $f } catch {}
            Remove-Item -Force -ErrorAction SilentlyContinue -LiteralPath $f, (Join-Path $d "$PID.cwd")
        }
        try {
            New-Item -ItemType Directory -Force -Path $d | Out-Null
            $envText = (Get-ChildItem env: | ForEach-Object { "$($_.Name)=$($_.Value)`0" }) -join ''
            [IO.File]::WriteAllText((Join-Path $d "$PID"), $envText)
        } catch {}
        $e = [char]27
        $a = [char]7
        $out = ''
        if ($global:__TesseraRunning) { $out += "$e]133;D;$code$a" }
        $global:__TesseraRunning = $false
        $out += "$e]7;file://$([Environment]::MachineName)$((Get-Location).ProviderPath)$a$e]133;A;redraw=0$a"
        $inner = if ($global:__TesseraPrompt) { & $global:__TesseraPrompt } else { "PS $((Get-Location).Path)> " }
        $out + $inner + "$e]133;B$a"
    }

    if (Get-Module -Name PSReadLine) {
        Set-PSReadLineKeyHandler -Chord Enter -ScriptBlock {
            $line = $null
            $cursor = $null
            [Microsoft.PowerShell.PSConsoleReadLine]::GetBufferState([ref]$line, [ref]$cursor)
            [Microsoft.PowerShell.PSConsoleReadLine]::AcceptLine()
            if ($line.Trim() -ne '') {
                $b = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($line))
                $e = [char]27
                $a = [char]7
                $envs = (Get-ChildItem env: | ForEach-Object { "$($_.Name)=$($_.Value)" }) -join "`0"
                $x = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes("$([Environment]::UserName)`0$line`0$envs"))
                [Console]::Write("$e]1337;TesseraExec=$x$a$e]1337;TesseraCommand=$b$a$e]133;C$a")
                $global:__TesseraRunning = $true
            }
        }
    }
}
