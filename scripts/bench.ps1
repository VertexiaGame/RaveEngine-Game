param(
    [string]$BeforeCommit = "origin/main",
    [string]$AfterCommit = "HEAD",
    [string]$MapPath = "assets/maps/temp_playtest.vrtx",
    [int]$Frames = 500,
    [int]$WarmupFrames = 100,
    [ValidateSet("server", "client", "studio", "all")]
    [string]$Scenario = "server"
)

$ErrorActionPreference = "Stop"
$repo = (git rev-parse --show-toplevel).Trim()
if ($LASTEXITCODE -ne 0 -or -not $repo) { throw "Not inside a Git repository" }
$beforeHash = (git rev-parse --verify "$BeforeCommit`^{commit}").Trim()
if ($LASTEXITCODE -ne 0 -or -not $beforeHash) { throw "Invalid before revision: $BeforeCommit" }
$afterHash = (git rev-parse --verify "$AfterCommit`^{commit}").Trim()
if ($LASTEXITCODE -ne 0 -or -not $afterHash) { throw "Invalid after revision: $AfterCommit" }
$tempRoot = Join-Path ([IO.Path]::GetTempPath()) ("rave-bench-" + [Guid]::NewGuid().ToString("N"))
$beforeWorktree = Join-Path $tempRoot "before"
$afterWorktree = Join-Path $tempRoot "after"

function Invoke-Native {
    param([scriptblock]$Command, [string]$Failure)
    & $Command
    if ($LASTEXITCODE -ne 0) { throw $Failure }
}

function Add-BenchHarness {
    param([string]$Worktree)
    $benchFile = Join-Path $Worktree "src/common/core/bench.rs"
    if ((Test-Path -LiteralPath $benchFile) -and (Select-String -Path $benchFile -Pattern 'median_frame_ns' -Quiet)) {
        return
    }
    $hasBenchFeature = Select-String -Path (Join-Path $Worktree "Cargo.toml") -Pattern '^bench\s*=\s*\[\]' -Quiet
    $coreDir = Join-Path $Worktree "src/common/core"
    Copy-Item -LiteralPath (Join-Path $repo "src/bin/server.rs") -Destination (Join-Path $Worktree "src/bin/server.rs")
    Copy-Item -LiteralPath (Join-Path $repo "src/common/core/mod.rs") -Destination (Join-Path $coreDir "mod.rs")
    Copy-Item -LiteralPath (Join-Path $repo "src/common/core/bench.rs") -Destination (Join-Path $coreDir "bench.rs")
    $clientSource = Get-Content -LiteralPath (Join-Path $repo "src/client/mod.rs")
    $clientMarker = [Array]::IndexOf($clientSource, 'fn spawn_client_benchmark(mut commands: Commands) {')
    if ($clientMarker -lt 1) { throw "Client benchmark harness marker not found" }
    Add-Content -LiteralPath (Join-Path $Worktree "src/client/mod.rs") -Value ("`n" + ($clientSource[($clientMarker - 1)..($clientSource.Count - 1)] -join "`n"))

    $studioUiPath = Join-Path $Worktree "src/studio/ui.rs"
    $studioUi = Get-Content -Raw -LiteralPath $studioUiPath
    if ($studioUi.Contains('fn line_numbers(')) {
        $studioUi = $studioUi.Replace('fn line_numbers(', 'pub(crate) fn line_numbers(')
        Set-Content -LiteralPath $studioUiPath -Value $studioUi
    } else {
        Add-Content -LiteralPath $studioUiPath -Value @'

pub(crate) fn line_numbers(cache: &mut Option<(usize, String)>, total_lines: usize) -> &str {
    let max_digit_width = total_lines.to_string().len();
    let mut text = String::with_capacity(total_lines * (max_digit_width + 1));
    for line in 1..=total_lines {
        text.push_str(&format!("{:>width$}\n", line, width = max_digit_width));
    }
    *cache = Some((total_lines, text));
    cache.as_ref().unwrap().1.as_str()
}
'@
    }
    $studioSource = Get-Content -LiteralPath (Join-Path $repo "src/studio/mod.rs")
    $studioMarker = [Array]::IndexOf($studioSource, 'fn spawn_studio_benchmark(mut commands: Commands) {')
    if ($studioMarker -lt 1) { throw "Studio benchmark harness marker not found" }
    Add-Content -LiteralPath (Join-Path $Worktree "src/studio/mod.rs") -Value ("`n" + ($studioSource[($studioMarker - 1)..($studioSource.Count - 1)] -join "`n"))
    if (-not $hasBenchFeature) {
        Add-Content -LiteralPath (Join-Path $Worktree "Cargo.toml") -Value "`n[features]`nbench = []"
    }
}

function Run-Bench {
    param([string]$Label, [string]$Worktree, [string]$Commit, [string]$BenchScenario)
    Write-Host "=== BENCHMARK: $Label $BenchScenario ($Commit) ===" -ForegroundColor Cyan
    Add-BenchHarness $Worktree
    Push-Location $Worktree
    try {
        Invoke-Native { cargo build --release --features bench --bin RaveEngineServer } "Build failed for $Commit"
        $result = & .\target\release\RaveEngineServer.exe --benchmark --bench-scenario $BenchScenario --bench-frames $Frames --bench-warmup $WarmupFrames --map $MapPath 2>&1
        if ($LASTEXITCODE -ne 0) { throw "Benchmark failed for $Commit" }
        $jsonLine = $result | Select-String '^\{"scenario"' | Select-Object -Last 1
        if (-not $jsonLine) { throw "No JSON output from benchmark for $Commit" }
        $outFile = Join-Path $repo "bench_$($Label)_$BenchScenario.json"
        $jsonLine.Line | Out-File -Encoding utf8 $outFile
        Write-Host "  -> $outFile" -ForegroundColor Green
        return $outFile
    } finally {
        Pop-Location
    }
}

New-Item -ItemType Directory -Path $tempRoot | Out-Null
try {
    Invoke-Native { git -C $repo worktree add --detach $beforeWorktree $beforeHash } "Failed to create before worktree"
    Invoke-Native { git -C $repo worktree add --detach $afterWorktree $afterHash } "Failed to create after worktree"
    function Pct($old, $new) {
        if ($old -eq 0) { return "n/a" }
        $pct = [math]::Round(($new - $old) / $old * 100, 1)
        if ($pct -lt 0) { return "$pct% (faster)" }
        return "+$pct% (slower)"
    }

    $scenarios = if ($Scenario -eq "all") { @("server", "client", "studio") } else { @($Scenario) }
    foreach ($benchScenario in $scenarios) {
        $beforeFile = Run-Bench "before" $beforeWorktree $beforeHash $benchScenario
        $afterFile = Run-Bench "after" $afterWorktree $afterHash $benchScenario
        $before = Get-Content -LiteralPath $beforeFile | ConvertFrom-Json
        $after = Get-Content -LiteralPath $afterFile | ConvertFrom-Json

        Write-Host "`n=== $($benchScenario.ToUpper()) COMPARISON ===" -ForegroundColor Yellow
        Write-Host ("{0,-24} {1,16} {2,16} {3}" -f "Metric", "Before", "After", "Delta")
        Write-Host ("{0,-24} {1,16:N0} {2,16:N0} {3}" -f "avg_frame_ns", $before.avg_frame_ns, $after.avg_frame_ns, (Pct $before.avg_frame_ns $after.avg_frame_ns))
        Write-Host ("{0,-24} {1,16:N0} {2,16:N0} {3}" -f "median_frame_ns", $before.median_frame_ns, $after.median_frame_ns, (Pct $before.median_frame_ns $after.median_frame_ns))
        Write-Host ("{0,-24} {1,16:N0} {2,16:N0} {3}" -f "p95_frame_ns", $before.p95_frame_ns, $after.p95_frame_ns, (Pct $before.p95_frame_ns $after.p95_frame_ns))
        if ($benchScenario -eq "studio") {
            Write-Host ("{0,-24} {1,16:N0} {2,16:N0}" -f "mesh_assets", $before.mesh_assets, $after.mesh_assets)
            Write-Host ("{0,-24} {1,16:N0} {2,16:N0}" -f "material_assets", $before.material_assets, $after.material_assets)
        }
    }
} finally {
    if (Test-Path -LiteralPath $beforeWorktree) { git -C $repo worktree remove --force $beforeWorktree | Out-Null }
    if (Test-Path -LiteralPath $afterWorktree) { git -C $repo worktree remove --force $afterWorktree | Out-Null }
    if (Test-Path -LiteralPath $tempRoot) { Remove-Item -LiteralPath $tempRoot }
    git -C $repo worktree prune
}
