param(
    [Parameter(Mandatory = $true)][string]$WorkDirectory
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
if (!$IsWindows) { throw 'Windows qualification requires Windows.' }
New-Item -ItemType Directory -Force -Path $WorkDirectory | Out-Null
$work = (Resolve-Path $WorkDirectory).Path
$reports = Join-Path $work 'reports'
New-Item -ItemType Directory -Force -Path $reports | Out-Null

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

$installers = @(Get-ChildItem src-tauri/target/debug/bundle/nsis/*.exe)
if ($installers.Count -ne 1) { throw 'Expected exactly one NSIS installer.' }
$installed = Join-Path $work 'installed'
$process = Start-Process -FilePath $installers[0].FullName -ArgumentList "/S /D=$installed" -PassThru -Wait
if ($process.ExitCode -ne 0) { throw "Installer exited with $($process.ExitCode)" }
$app = Join-Path $installed 'cachella.exe'
$engine = Join-Path $installed 'engine/llama-server.exe'
$decoder = Join-Path $installed 'decoder/ffmpeg.exe'
foreach ($path in @($app, $engine, $decoder)) {
    if (!(Test-Path -LiteralPath $path -PathType Leaf)) { throw "Missing installed file: $path" }
}

# Validate the installed payload using the same pinned verification as preparation.
& python3 -c 'import pathlib,sys; sys.path.insert(0,"scripts"); import prepare_engine as p; assert p.verify_output(pathlib.Path(sys.argv[1]),p.ARTIFACTS["windows-x86_64"]), "Installed engine failed pinned verification"' (Join-Path $installed 'engine')
if ($LASTEXITCODE -ne 0) { throw 'Installed engine verification failed.' }

$desktop = Start-Process -FilePath $app -PassThru
try {
    if (!$desktop.WaitForInputIdle(30000)) { throw 'Desktop did not become input-idle.' }
    $deadline = [DateTime]::UtcNow.AddSeconds(30)
    do {
        $desktop.Refresh()
        if ($desktop.HasExited) { throw 'Desktop exited during launch.' }
        if ($desktop.MainWindowHandle -ne 0) { break }
        Start-Sleep -Milliseconds 250
    } while ([DateTime]::UtcNow -lt $deadline)
    if ($desktop.MainWindowHandle -eq 0) { throw 'Desktop did not create a main window.' }
    $windowTitle = $desktop.MainWindowTitle
    if (!$desktop.CloseMainWindow()) { throw 'Desktop rejected the window-close request.' }
    if (!$desktop.WaitForExit(30000)) { throw 'Desktop did not exit after window close.' }
    if ($desktop.ExitCode -ne 0) { throw "Desktop exited with $($desktop.ExitCode)" }
} finally {
    if (!$desktop.HasExited) { Stop-Process -Id $desktop.Id -Force }
}
@{
    installer_sha256 = (Get-FileHash $installers[0].FullName -Algorithm SHA256).Hash
    installed_engine_verified = $true
    window_title = $windowTitle
    window_opened_and_closed = $true
    scope = 'Hosted Windows GUI launch and close; interactive import UX and consumer-device performance remain separate.'
} | ConvertTo-Json | Set-Content (Join-Path $reports 'installer.json')
Write-Output 'PASS: NSIS installation, pinned engine payload, desktop window launch and close.'

$models = Join-Path $work 'models'
Invoke-CheckedExample 'install_models_smoke' @($models, 'qwen3-1.7b-q4', (Join-Path $reports 'qwen-install.json'))
Invoke-CheckedExample 'managed_runtime_smoke' @($engine, (Find-Model 'Qwen3-1.7B-Q4_K_M.gguf'))
Invoke-CheckedExample 'install_models_smoke' @($models, 'embeddinggemma2-q8', (Join-Path $reports 'embedding-install.json'))
Invoke-CheckedExample 'multimodal_smoke' @(
    $engine,
    (Find-Model 'embeddinggemma-2-Q8_0.gguf'),
    (Find-Model 'mmproj-embeddinggemma-2-Q8_0.gguf'),
    $decoder,
    'evaluation/multimodal/fixtures',
    (Join-Path $reports 'multimodal.json')
)

$remaining = @(Get-Process -Name 'llama-server', 'cachella' -ErrorAction SilentlyContinue)
if ($remaining.Count -ne 0) { throw 'Managed desktop or engine processes remained after qualification.' }
@{
    completed = $true
    remaining_desktop_or_engine_processes = $remaining.Count
    physical_memory_bytes = (Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory
    cpu = @((Get-CimInstance Win32_Processor).Name)
    scope = 'Hosted Windows CPU execution with Qwen3 and EmbeddingGemma 2; synthetic retrieval fixtures.'
} | ConvertTo-Json | Set-Content (Join-Path $reports 'qualification.json')
Write-Output 'PASS: Windows model installation, managed inference, multimodal retrieval and process cleanup.'
