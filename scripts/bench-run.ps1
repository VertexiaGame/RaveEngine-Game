param(
    [string]$MapPath = "assets/maps/temp_playtest.vrtx",
    [int]$Frames = 600,
    [int]$WarmupFrames = 120,
    [ValidateSet("server", "bricks", "client", "studio", "all")]
    [string]$Scenario = "all"
)

$ErrorActionPreference = "Stop"
$repo = (git rev-parse --show-toplevel).Trim()
if ($LASTEXITCODE -ne 0 -or -not $repo) { throw "Not inside a Git repository" }
$commit = (git -C $repo rev-parse --short HEAD).Trim()

function Invoke-Native {
    param([scriptblock]$Command, [string]$Failure)
    & $Command
    if ($LASTEXITCODE -ne 0) { throw $Failure }
}

$exe = Join-Path $repo "target\release\RaveEngineServer.exe"
if (-not (Test-Path -LiteralPath $exe)) {
    Write-Host "Building bench binary !!!" -ForegroundColor Cyan
    Invoke-Native { cargo build --release --features bench --bin RaveEngineServer } "Release bench build failure"
}

function Run-Scenario {
    param([string]$BenchScenario)
    Write-Host "=== BENCHMARK: $BenchScenario ($commit) ===" -ForegroundColor Cyan
    $eap = $ErrorActionPreference
    $ErrorActionPreference = "SilentlyContinue"
    try {
        $result = & $exe --benchmark --bench-scenario $BenchScenario --bench-frames $Frames --bench-warmup $WarmupFrames --map $MapPath 2>&1
    } finally {
        $ErrorActionPreference = $eap
    }
    if ($LASTEXITCODE -ne 0) { throw "Benchmark failed for $BenchScenario" }
    $jsonLine = $result | Select-String '^\{"scenario"' | Select-Object -Last 1
    if (-not $jsonLine) { throw "No JSON output from benchmark for $BenchScenario" }
    $outFile = Join-Path $repo "bench_$BenchScenario.json"
    $jsonLine.Line | Out-File -Encoding utf8 $outFile
    Write-Host "  -> $outFile" -ForegroundColor Green
    return (Get-Content -LiteralPath $outFile | ConvertFrom-Json)
}

function Fps {
    param($FrameNs)
    if ($FrameNs -eq 0) { return 0 }
    return [math]::Round(1e9 / $FrameNs, 1)
}

$scenarios = if ($Scenario -eq "all") { @("server", "bricks", "client", "studio") } else { @($Scenario) }
$results = @{}
foreach ($s in $scenarios) {
    $results[$s] = Run-Scenario $s
}

Write-Host "`n===== SPEED & OPTIMIZATION SUMMARY ($commit) =====" -ForegroundColor Yellow
Write-Host ("{0,-10} {1,16} {2,10} {3,14} {4,14} {5,12} {6,12}" -f "Scenario", "avg_frame_ns", "FPS", "median_ns", "p95_ns", "meshes", "materials")
Write-Host ("{0,-10} {1,16} {2,10} {3,14} {4,14} {5,12} {6,12}" -f "--------", "-----------", "---", "---------", "-------", "------", "---------")
$culture = [System.Globalization.CultureInfo]::InvariantCulture
foreach ($s in $scenarios) {
    $r = $results[$s]
    Write-Host ("{0,-10} {1,16} {2,10} {3,14} {4,14} {5,12} {6,12}" -f $s, $r.avg_frame_ns.ToString("N0", $culture), (Fps $r.avg_frame_ns).ToString("N1", $culture), $r.median_frame_ns.ToString("N0", $culture), $r.p95_frame_ns.ToString("N0", $culture), $r.mesh_assets.ToString("N0", $culture), $r.material_assets.ToString("N0", $culture))
}
