# Dot-source this file, then call Invoke-DesktopProbe. Input-idle is diagnostic:
# console-subsystem programs can host a working desktop window.
function Initialize-DesktopProbeNative {
    if ('Cachella.DesktopProbeNative' -as [type]) { return }
    Add-Type -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading.Tasks;
namespace Cachella {
    public sealed class DesktopWindow {
        public long handle;
        public string title;
        public string window_class;
    }
    public static class DesktopProbeNative {
        private static Type elementType;
        private static Type conditionType;
        private static Type scopeType;
        private delegate bool EnumWindowsCallback(IntPtr window, IntPtr data);
        [DllImport("user32.dll")] private static extern bool EnumWindows(EnumWindowsCallback callback, IntPtr data);
        [DllImport("user32.dll")] private static extern bool IsWindowVisible(IntPtr window);
        [DllImport("user32.dll")] private static extern uint GetWindowThreadProcessId(IntPtr window, out uint processId);
        [DllImport("user32.dll", CharSet = CharSet.Unicode)] private static extern int GetWindowText(IntPtr window, StringBuilder text, int count);
        [DllImport("user32.dll", CharSet = CharSet.Unicode)] private static extern int GetClassName(IntPtr window, StringBuilder text, int count);
        [DllImport("user32.dll", SetLastError = true)] private static extern bool PostMessage(IntPtr window, uint message, IntPtr wparam, IntPtr lparam);
        public static DesktopWindow[] VisibleWindows(uint processId) {
            var result = new List<DesktopWindow>();
            EnumWindows(delegate(IntPtr window, IntPtr unused) {
                uint owner;
                GetWindowThreadProcessId(window, out owner);
                if (owner != processId || !IsWindowVisible(window)) return true;
                var klass = new StringBuilder(256);
                GetClassName(window, klass, klass.Capacity);
                if (String.Equals(klass.ToString(), "ConsoleWindowClass", StringComparison.Ordinal)) return true;
                var title = new StringBuilder(1024);
                GetWindowText(window, title, title.Capacity);
                result.Add(new DesktopWindow { handle = window.ToInt64(), title = title.ToString(), window_class = klass.ToString() });
                return true;
            }, IntPtr.Zero);
            return result.ToArray();
        }
        public static bool Close(long window) { return PostMessage(new IntPtr(window), 0x0010, IntPtr.Zero, IntPtr.Zero); }
        public static void ConfigureReadyReader(Assembly client, Assembly types) {
            // Keep references to the assemblies loaded by the PowerShell host.
            // Background tasks do not perform a separate load-context lookup.
            elementType = client.GetType("System.Windows.Automation.AutomationElement", true);
            conditionType = client.GetType("System.Windows.Automation.PropertyCondition", true);
            scopeType = types.GetType("System.Windows.Automation.TreeScope", true);
        }
        // UI Automation providers may stall. One background task runs per probe,
        // and the caller polls its completion inside the startup deadline.
        public static Task<bool> BeginReadyScan(long window, string readyText) {
            return Task.Run(delegate {
                var element = elementType.GetMethod("FromHandle").Invoke(null, new object[] { new IntPtr(window) });
                var name = elementType.GetField("NameProperty").GetValue(null);
                var condition = Activator.CreateInstance(conditionType, new object[] { name, readyText });
                var descendant = Enum.Parse(scopeType, "Descendants");
                return elementType.GetMethod("FindFirst").Invoke(element, new object[] { descendant, condition }) != null;
            });
        }
    }
}
'@
}

function Get-DesktopProbeSubsystem {
    param([string]$Executable)
    $stream = [IO.File]::OpenRead($Executable)
    $reader = [IO.BinaryReader]::new($stream)
    try {
        if ($stream.Length -lt 64 -or $reader.ReadUInt16() -ne 0x5A4D) { throw 'Missing DOS header.' }
        $stream.Position = 60
        $offset = $reader.ReadUInt32()
        if ($offset -lt 64 -or $offset -gt 1048576 -or ([long]$offset + 94) -gt $stream.Length) { throw 'PE header offset is outside the bounded header range.' }
        $stream.Position = $offset
        if ($reader.ReadUInt32() -ne 0x00004550) { throw 'Missing PE signature.' }
        $stream.Position = [long]$offset + 20
        if ($reader.ReadUInt16() -lt 70) { throw 'PE optional header is too short.' }
        $stream.Position = [long]$offset + 24
        $magic = $reader.ReadUInt16()
        if ($magic -ne 0x10B -and $magic -ne 0x20B) { throw 'Unsupported PE optional header.' }
        $stream.Position = [long]$offset + 92
        $value = $reader.ReadUInt16()
        $name = switch ($value) { 2 { 'windows_gui' } 3 { 'windows_console' } default { 'other' } }
        return [pscustomobject]@{ value = $value; name = $name }
    } finally { $reader.Dispose() }
}

function Invoke-DesktopProbe {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory = $true)][string]$Executable,
        [Parameter(Mandatory = $true)][string]$ReportDirectory,
        [string]$ExpectedTitle = 'Cachella',
        [string]$ReadyText = 'Workspace ready',
        [ValidateRange(1, 3600)][int]$StartupTimeoutSeconds = 60,
        [ValidateRange(1, 3600)][int]$CloseTimeoutSeconds = 30
    )
    $started = [DateTime]::UtcNow
    New-Item -ItemType Directory -Force -Path $ReportDirectory | Out-Null
    $reportPath = Join-Path $ReportDirectory 'desktop.json'
    $stdoutPath = Join-Path $ReportDirectory 'desktop.stdout.log'
    $stderrPath = Join-Path $ReportDirectory 'desktop.stderr.log'
    [IO.File]::WriteAllText($stdoutPath, '')
    [IO.File]::WriteAllText($stderrPath, '')
    $record = [ordered]@{
        passed = $false; executable = $Executable; process_id = $null
        expected_title = $ExpectedTitle; ready_text = $ReadyText
        startup_timeout_seconds = $StartupTimeoutSeconds; close_timeout_seconds = $CloseTimeoutSeconds
        pe_subsystem = $null; pe_subsystem_error = $null
        input_idle = [ordered]@{ outcome = 'not_observed'; result = $null; exception = $null; elapsed_ms = 0 }
        last_visible_windows = @(); matched_window = $null; ready_observed = $false
        readiness_error = $null; close_requested = $false; clean_close = $false
        exit_code = $null; exit_code_hex = $null; forced_cleanup = $false; cleanup_error = $null
        failure = $null; started_utc = $started.ToString('o'); elapsed_ms = 0
        stdout_log = $stdoutPath; stderr_log = $stderrPath
    }
    $process = $null
    $scan = $null
    $scanWindow = 0L
    try {
        if ([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT) { throw 'Windows desktop probe requires Windows.' }
        Initialize-DesktopProbeNative
        try { $record.pe_subsystem = Get-DesktopProbeSubsystem $Executable }
        catch { $record.pe_subsystem_error = $_.Exception.Message }
        # Framework UI Automation assemblies are also available to PowerShell 7
        # on hosted Windows images. Load dependencies before the background scan.
        if ($ReadyText) {
            try {
                foreach ($assembly in @('UIAutomationTypes', 'UIAutomationClient')) {
                    try { Add-Type -AssemblyName $assembly -ErrorAction Stop }
                    catch {
                        $dll = Join-Path $env:WINDIR "Microsoft.NET/Framework64/v4.0.30319/WPF/$assembly.dll"
                        Add-Type -Path $dll -ErrorAction Stop
                    }
                }
                $loaded = [AppDomain]::CurrentDomain.GetAssemblies()
                $client = @($loaded | Where-Object { $_.GetName().Name -eq 'UIAutomationClient' })[0]
                $types = @($loaded | Where-Object { $_.GetName().Name -eq 'UIAutomationTypes' })[0]
                [Cachella.DesktopProbeNative]::ConfigureReadyReader($client, $types)
            } catch { $record.readiness_error = $_.Exception.Message }
        }
        $process = Start-Process -FilePath $Executable -PassThru -RedirectStandardOutput $stdoutPath -RedirectStandardError $stderrPath
        $record.process_id = $process.Id
        $deadline = [DateTime]::UtcNow.AddSeconds($StartupTimeoutSeconds)
        $idleStarted = [DateTime]::UtcNow
        try {
            $record.input_idle.result = $process.WaitForInputIdle([Math]::Min(1000, $StartupTimeoutSeconds * 1000))
            $record.input_idle.outcome = 'returned'
        } catch {
            $record.input_idle.outcome = 'exception'
            $record.input_idle.exception = $_.Exception.Message
        }
        $record.input_idle.elapsed_ms = [int]([DateTime]::UtcNow - $idleStarted).TotalMilliseconds
        while ([DateTime]::UtcNow -lt $deadline) {
            $process.Refresh()
            if ($process.HasExited) {
                $record.exit_code = $process.ExitCode
                throw "Desktop exited during startup with code $($record.exit_code)."
            }
            $record.last_visible_windows = @([Cachella.DesktopProbeNative]::VisibleWindows([uint32]$process.Id))
            $windows = @($record.last_visible_windows | Where-Object { [string]::Equals($_.title, $ExpectedTitle, [StringComparison]::Ordinal) })
            if ($windows.Count -gt 0) {
                $record.matched_window = $windows[0]
                if (!$ReadyText) { $record.ready_observed = $true }
                elseif (!$record.readiness_error) {
                    if ($null -ne $scan -and $scan.IsCompleted) {
                        if ($scan.IsFaulted) { $record.readiness_error = $scan.Exception.GetBaseException().Message }
                        elseif ($scan.Result -and $scanWindow -eq $record.matched_window.handle) { $record.ready_observed = $true }
                        $scan = $null
                    }
                    if (!$record.ready_observed -and !$record.readiness_error -and $null -eq $scan) {
                        $scanWindow = $record.matched_window.handle
                        $scan = [Cachella.DesktopProbeNative]::BeginReadyScan($scanWindow, $ReadyText)
                    }
                }
                if ($record.ready_observed) { break }
            }
            Start-Sleep -Milliseconds 100
        }
        if ($null -eq $record.matched_window) { throw "Desktop did not create a visible owned window titled '$ExpectedTitle' before the startup deadline." }
        if (!$record.ready_observed) { throw "Desktop did not expose readiness text '$ReadyText' before the startup deadline. $($record.readiness_error)" }
        $record.close_requested = [Cachella.DesktopProbeNative]::Close($record.matched_window.handle)
        if (!$record.close_requested) { throw 'Desktop window-close request failed.' }
        if (!$process.WaitForExit($CloseTimeoutSeconds * 1000)) { throw 'Desktop did not exit before the close deadline.' }
        $record.exit_code = $process.ExitCode
        if ($record.exit_code -ne 0) { throw "Desktop exited after close with code $($record.exit_code)." }
        $record.clean_close = $true
        $record.passed = $true
    } catch { $record.failure = $_.Exception.Message }
    finally {
        # Keep ownership established at Start-Process; global process discovery
        # and UI Automation observations are never prerequisites for cleanup.
        if ($null -ne $process) {
            try {
                if (!$process.HasExited) {
                    $record.forced_cleanup = $true
                    $process.Kill($true)
                    if (!$process.WaitForExit(5000)) { throw 'Owned desktop process survived forced cleanup.' }
                }
                if ($null -eq $record.exit_code) { $record.exit_code = $process.ExitCode }
            } catch {
                $record.cleanup_error = $_.Exception.Message
                $record.passed = $false
            } finally { $process.Dispose() }
        }
        if ($null -ne $record.exit_code) {
            $unsignedExit = [BitConverter]::ToUInt32([BitConverter]::GetBytes([int]$record.exit_code), 0)
            $record.exit_code_hex = '0x{0:X8}' -f $unsignedExit
        }
        $record.elapsed_ms = [int]([DateTime]::UtcNow - $started).TotalMilliseconds
        $record | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $reportPath -Encoding utf8
    }
    return [pscustomobject]$record
}
