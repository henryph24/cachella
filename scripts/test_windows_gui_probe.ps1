param(
    [Parameter(Mandatory = $true)][string]$WorkDirectory
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
if ([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT) { throw 'Native desktop probe fixtures require Windows.' }
. (Join-Path $PSScriptRoot 'windows_gui_probe.ps1')
New-Item -ItemType Directory -Force -Path $WorkDirectory | Out-Null
$work = (Resolve-Path -LiteralPath $WorkDirectory).Path
$source = Join-Path $work 'DesktopFixture.cs'
$compiler = Join-Path $env:WINDIR 'Microsoft.NET/Framework64/v4.0.30319/csc.exe'
if (!(Test-Path -LiteralPath $compiler -PathType Leaf)) { throw "Missing Windows Framework C# compiler: $compiler" }

# Compile the same program with the two PE subsystems. The console variant
# pauses before creating its message queue, exposing the old input-idle gate.
@'
using System;
using System.Drawing;
using System.IO;
using System.Threading;
using System.Windows.Forms;
class DesktopFixture {
    [STAThread]
    static int Main() {
        string name = Path.GetFileNameWithoutExtension(Application.ExecutablePath);
        Console.WriteLine("fixture started: " + name);
        if (name == "ExitEarly") {
            Console.Error.WriteLine("intentional fixture exit 42");
            return 42;
        }
        Thread.Sleep(600);
        Application.EnableVisualStyles();
        using (var window = new Form()) {
            window.Text = "Cachella";
            window.Size = new Size(480, 180);
            var label = new Label();
            label.AutoSize = true;
            label.Location = new Point(24, 24);
            label.Text = name == "NeverReady" ? "Starting workspace" : "Workspace ready";
            label.AccessibleName = label.Text;
            window.Controls.Add(label);
            Application.Run(window);
        }
        Console.WriteLine("fixture closed normally");
        return 0;
    }
}
'@ | Set-Content -LiteralPath $source -Encoding utf8

foreach ($fixture in @(
    @{ name = 'ConsoleWindow'; target = 'exe' },
    @{ name = 'GuiWindow'; target = 'winexe' },
    @{ name = 'ExitEarly'; target = 'exe' },
    @{ name = 'NeverReady'; target = 'winexe' }
)) {
    $output = Join-Path $work "$($fixture.name).exe"
    & $compiler /nologo "/target:$($fixture.target)" "/out:$output" /reference:System.Windows.Forms.dll /reference:System.Drawing.dll $source
    if ($LASTEXITCODE -ne 0) { throw "Fixture compilation failed: $($fixture.name) ($LASTEXITCODE)." }
}

$results = [Collections.Generic.List[object]]::new()
function Assert-Fixture {
    param([bool]$Condition, [string]$Message)
    if (!$Condition) { throw $Message }
}
function Invoke-Fixture {
    param([string]$Name, [int]$Timeout = 12)
    $reportDirectory = Join-Path $work $Name
    $result = Invoke-DesktopProbe -Executable (Join-Path $work "$Name.exe") -ReportDirectory $reportDirectory -ExpectedTitle 'Cachella' -ReadyText 'Workspace ready' -StartupTimeoutSeconds $Timeout -CloseTimeoutSeconds 5
    foreach ($file in @('desktop.json', 'desktop.stdout.log', 'desktop.stderr.log')) {
        Assert-Fixture (Test-Path -LiteralPath (Join-Path $reportDirectory $file) -PathType Leaf) "$Name did not retain $file."
    }
    Assert-Fixture ($null -eq $result.cleanup_error) "$Name cleanup failed: $($result.cleanup_error)"
    Assert-Fixture ($null -eq (Get-Process -Id $result.process_id -ErrorAction SilentlyContinue)) "$Name left its owned process running."
    $results.Add([pscustomobject]@{ name = $Name; probe = $result })
    return $result
}

$summaryPath = Join-Path $work 'fixture-results.json'
$summary = [ordered]@{ passed = $false; tests = 4; old_input_idle_gate_regression = $null; fixtures = @(); failure = $null }
try {
    $console = Invoke-Fixture 'ConsoleWindow'
    Assert-Fixture $console.passed "Console GUI fixture failed: $($console.failure)"
    Assert-Fixture ($console.pe_subsystem.value -eq 3) 'Console fixture PE subsystem was not 3.'
    Assert-Fixture ($console.ready_observed -and $console.clean_close -and $console.exit_code -eq 0) 'Console fixture did not prove ready state and clean exit.'
    # This reproduces the previous acceptance rule against the actual native
    # observation, while the current rule requires window + marker + clean exit.
    $oldGateRejected = $console.input_idle.outcome -eq 'exception' -or $console.input_idle.result -ne $true
    $summary.old_input_idle_gate_regression = [ordered]@{
        input_idle = $console.input_idle
        old_gate_would_reject = $oldGateRejected
        current_probe_passed = $console.passed
        pe_subsystem = $console.pe_subsystem
    }
    Assert-Fixture $oldGateRejected 'Console fixture did not reproduce the old input-idle gate rejection.'
    Write-Output "PASS: console subsystem window accepted; old input-idle gate rejected ($($console.input_idle.outcome))."

    $gui = Invoke-Fixture 'GuiWindow'
    Assert-Fixture $gui.passed "GUI subsystem fixture failed: $($gui.failure)"
    Assert-Fixture ($gui.pe_subsystem.value -eq 2) 'GUI fixture PE subsystem was not 2.'
    Assert-Fixture ($gui.ready_observed -and $gui.clean_close -and $gui.exit_code -eq 0) 'GUI fixture did not prove ready state and clean exit.'
    Write-Output 'PASS: GUI subsystem window, ready marker, and clean exit.'

    $early = Invoke-Fixture 'ExitEarly' 4
    Assert-Fixture (!$early.passed -and $early.exit_code -eq 42 -and !$early.forced_cleanup) 'Early-exit fixture did not preserve exit code 42.'
    Assert-Fixture ($early.failure -like '*exited during startup*') 'Early-exit fixture did not record its startup failure.'
    Assert-Fixture ($early.elapsed_ms -lt 9000) "Early-exit fixture exceeded its bound ($($early.elapsed_ms) ms)."
    Assert-Fixture ((Get-Content -LiteralPath $early.stderr_log -Raw) -like '*intentional fixture exit 42*') 'Early-exit fixture lost stderr diagnostics.'
    Write-Output 'PASS: early exit rejected with code 42 and retained stderr.'

    $never = Invoke-Fixture 'NeverReady' 4
    Assert-Fixture (!$never.passed -and $null -ne $never.matched_window -and !$never.ready_observed -and $never.forced_cleanup) 'Never-ready fixture did not fail after observing its real window.'
    Assert-Fixture ($never.failure -like '*readiness text*') 'Never-ready fixture did not record missing readiness.'
    Assert-Fixture ($null -eq $never.readiness_error) "Never-ready fixture failed its UI Automation reader: $($never.readiness_error)"
    Assert-Fixture ($never.elapsed_ms -lt 10000) "Never-ready fixture exceeded its timeout/cleanup bound ($($never.elapsed_ms) ms)."
    Write-Output 'PASS: never-ready window rejected within startup deadline and owned process removed.'
    $summary.passed = $true
} catch {
    $summary.failure = $_.Exception.Message
    throw
} finally {
    $summary.fixtures = @($results.ToArray())
    $summary | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $summaryPath -Encoding utf8
}
Write-Output 'PASS: 4 native Windows desktop probe fixtures.'
