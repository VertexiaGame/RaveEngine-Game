param(
    [string]$BeforeCommit = "origin/main",
    [string]$AfterCommit = "HEAD",
    [string]$MapPath = "assets/maps/temp_playtest.vrtx",
    [int]$Frames = 500,
    [int]$WarmupFrames = 100,
    [ValidateSet("server", "client")]
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
    if (-not $hasBenchFeature) {
        Add-Content -LiteralPath (Join-Path $Worktree "Cargo.toml") -Value "`n[features]`nbench = []"
    }
}

function Run-Bench {
    param([string]$Label, [string]$Worktree, [string]$Commit)
    Write-Host "=== BENCHMARK: $Label ($Commit) ===" -ForegroundColor Cyan
    Add-BenchHarness $Worktree
    Push-Location $Worktree
    try {
        Invoke-Native { cargo build --release --features bench --bin RaveEngineServer } "Build failed for $Commit"
        $result = & .\target\release\RaveEngineServer.exe --benchmark --bench-scenario $Scenario --bench-frames $Frames --bench-warmup $WarmupFrames --map $MapPath 2>&1
        if ($LASTEXITCODE -ne 0) { throw "Benchmark failed for $Commit" }
        $jsonLine = $result | Select-String '^\{"scenario"' | Select-Object -Last 1
        if (-not $jsonLine) { throw "No JSON output from benchmark for $Commit" }
        $outFile = Join-Path $repo "bench_$($Label)_$Scenario.json"
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
    $beforeFile = Run-Bench "before" $beforeWorktree $beforeHash
    $afterFile = Run-Bench "after" $afterWorktree $afterHash
    $before = Get-Content -LiteralPath $beforeFile | ConvertFrom-Json
    $after = Get-Content -LiteralPath $afterFile | ConvertFrom-Json

    function Pct($old, $new) {
        if ($old -eq 0) { return "n/a" }
        $pct = [math]::Round(($new - $old) / $old * 100, 1)
        if ($pct -lt 0) { return "$pct% (faster)" }
        return "+$pct% (slower)"
    }

    Write-Host "`n=== COMPARISON ===" -ForegroundColor Yellow
    Write-Host ("{0,-24} {1,16} {2,16} {3}" -f "Metric", "Before", "After", "Delta")
    Write-Host ("{0,-24} {1,16:N0} {2,16:N0} {3}" -f "avg_frame_ns", $before.avg_frame_ns, $after.avg_frame_ns, (Pct $before.avg_frame_ns $after.avg_frame_ns))
    Write-Host ("{0,-24} {1,16:N0} {2,16:N0} {3}" -f "median_frame_ns", $before.median_frame_ns, $after.median_frame_ns, (Pct $before.median_frame_ns $after.median_frame_ns))
    Write-Host ("{0,-24} {1,16:N0} {2,16:N0} {3}" -f "p95_frame_ns", $before.p95_frame_ns, $after.p95_frame_ns, (Pct $before.p95_frame_ns $after.p95_frame_ns))
} finally {
    if (Test-Path -LiteralPath $beforeWorktree) { git -C $repo worktree remove --force $beforeWorktree | Out-Null }
    if (Test-Path -LiteralPath $afterWorktree) { git -C $repo worktree remove --force $afterWorktree | Out-Null }
    if (Test-Path -LiteralPath $tempRoot) { Remove-Item -LiteralPath $tempRoot }
    git -C $repo worktree prune
}
