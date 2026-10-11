param(
    [Parameter(Mandatory = $true)][string]$WorkDirectory,
    [Parameter(Mandatory = $true)][string]$InstallerPath
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
if (!$IsWindows) { throw 'Windows qualification requires Windows.' }
New-Item -ItemType Directory -Force -Path $WorkDirectory | Out-Null
$work = (Resolve-Path $WorkDirectory).Path
$reports = Join-Path $work 'reports'
New-Item -ItemType Directory -Force -Path $reports | Out-Null
. (Join-Path $PSScriptRoot 'windows_gui_probe.ps1')
$stages = [ordered]@{}

function Invoke-Stage {
    param([string]$Name, [scriptblock]$Action)
    $record = [ordered]@{ passed = $false; started = [DateTime]::UtcNow.ToString('o'); error = $null }
    $timer = [Diagnostics.Stopwatch]::StartNew()
    try {
        & $Action | Out-Host
        $record.passed = $true
        Write-Host "PASS: $Name"
    } catch {
        $record.error = $_.ToString()
        Write-Host "FAIL: ${Name}: $_"
    } finally {
        $record.elapsed_seconds = $timer.Elapsed.TotalSeconds
        $stages[$Name] = $record
        $record | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $reports "$Name-stage.json")
    }
}

function Invoke-CheckedExample {
    param([string]$Name, [string[]]$ExampleArguments)
    & cargo run --manifest-path src-tauri/Cargo.toml --features desktop --example $Name -- @ExampleArguments 2>&1 |
        Tee-Object -FilePath (Join-Path $reports "$Name.log")
    if ($LASTEXITCODE -ne 0) { throw "$Name failed with exit code $LASTEXITCODE" }
}

function Find-Model {
    param([string]$Name)
    $matches = @(Get-ChildItem -LiteralPath (Join-Path $work 'models') -Recurse -File |
        Where-Object { $_.Name -ceq $Name })
    if ($matches.Count -ne 1) { throw "Expected exactly one verified model named $Name" }
    return $matches[0].FullName
}

$installed = Join-Path $work 'installed'
$app = Join-Path $installed 'cachella.exe'
$engine = Join-Path $installed 'engine/llama-server.exe'
$decoder = Join-Path $installed 'decoder/ffmpeg.exe'
$models = Join-Path $work 'models'
Invoke-Stage 'installer' {
    $installer = (Resolve-Path -LiteralPath $InstallerPath).Path
    $process = Start-Process -FilePath $installer -ArgumentList "/S /D=$installed" -PassThru
    try {
        if (!$process.WaitForExit(120000)) { throw 'Installer exceeded 120 seconds.' }
        if ($process.ExitCode -ne 0) { throw "Installer exited with $($process.ExitCode)" }
    } finally {
        if (!$process.HasExited) {
            $process.Kill($true)
            if (!$process.WaitForExit(5000)) { throw 'Installer survived forced cleanup.' }
        }
        $process.Dispose()
    }
    foreach ($path in @($app, $engine, $decoder)) {
        if (!(Test-Path -LiteralPath $path -PathType Leaf)) { throw "Missing installed file: $path" }
    }
    & python3 -c 'import pathlib,sys; sys.path.insert(0,"scripts"); import prepare_engine as p; assert p.verify_output(pathlib.Path(sys.argv[1]),p.ARTIFACTS["windows-x86_64"]), "Installed engine failed pinned verification"' (Join-Path $installed 'engine')
    if ($LASTEXITCODE -ne 0) { throw 'Installed engine verification failed.' }
    $subsystem = (Get-DesktopProbeSubsystem $app).value
    @{
        installer_sha256 = (Get-FileHash $installer -Algorithm SHA256).Hash
        app_sha256 = (Get-FileHash $app -Algorithm SHA256).Hash
        app_pe_subsystem = $subsystem
        installed_engine_verified = $true
        files = @(Get-ChildItem -LiteralPath $installed -Recurse -File | ForEach-Object { @{path=$_.FullName; bytes=$_.Length} })
    } | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $reports 'installer.json')
    if ($subsystem -ne 2) { throw "Expected release Windows GUI subsystem 2; found $subsystem" }
}

if ($stages['installer'].passed) {
    Invoke-Stage 'desktop' {
        $probe = Invoke-DesktopProbe -Executable $app -ReportDirectory $reports -ExpectedTitle 'Cachella' -ReadyText 'Workspace ready'
        if (!$probe.passed) { throw "Desktop probe failed; see desktop.json: $($probe.failure)" }
    }
    Invoke-Stage 'installed-helpers' {
        & python3 scripts/windows_installed_helpers.py --app $app --decoder $decoder --pdf-fixture evaluation/pdf/fixtures/project-brief.pdf --report-directory $reports
        if ($LASTEXITCODE -ne 0) { throw 'Installed helper qualification failed.' }
    }
    Invoke-Stage 'generation' {
        Invoke-CheckedExample 'install_models_smoke' @($models, 'qwen3-1.7b-q4', (Join-Path $reports 'qwen-install.json'))
        Invoke-CheckedExample 'managed_runtime_smoke' @($engine, (Find-Model 'Qwen3-1.7B-Q4_K_M.gguf'))
    }
    Invoke-Stage 'multimodal' {
        Invoke-CheckedExample 'install_models_smoke' @($models, 'embeddinggemma2-q8', (Join-Path $reports 'embedding-install.json'))
        Invoke-CheckedExample 'multimodal_smoke' @(
            $engine, (Find-Model 'embeddinggemma-2-Q8_0.gguf'),
            (Find-Model 'mmproj-embeddinggemma-2-Q8_0.gguf'), $decoder,
            'evaluation/multimodal/fixtures', (Join-Path $reports 'multimodal.json')
        )
    }
}
Invoke-Stage 'cleanup' {
    $remaining = @(Get-Process -Name 'llama-server', 'cachella' -ErrorAction SilentlyContinue)
    $remaining | Select-Object Id, ProcessName | ConvertTo-Json | Set-Content (Join-Path $reports 'remaining-processes.json')
    if ($remaining.Count -ne 0) { throw "$($remaining.Count) desktop or engine processes remained." }
}
$passed = $stages.Count -eq 6 -and @($stages.Values | Where-Object { !$_.passed }).Count -eq 0
@{
    passed = $passed
    stages = $stages
    physical_memory_bytes = (Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory
    cpu = @((Get-CimInstance Win32_Processor).Name)
    os = (Get-CimInstance Win32_OperatingSystem).Caption
    os_version = [Environment]::OSVersion.VersionString
    scope = 'Hosted Windows release installation, GUI readiness/close, installed helper modes, CPU inference and synthetic retrieval. Consumer Windows 11 hardware, interactive imports and GPU performance require separate qualification.'
} | ConvertTo-Json -Depth 10 | Set-Content (Join-Path $reports 'qualification.json')
if (!$passed) { throw 'Windows qualification failed; retained reports identify independent stage results.' }
Write-Output 'PASS: All six Windows qualification stages.'
