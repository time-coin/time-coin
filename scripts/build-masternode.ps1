# Build masternode binary for production deployment
# This script builds only what's needed for a masternode

Write-Host "🔨 Building TIME Coin Masternode (optimized)..." -ForegroundColor Cyan
Write-Host "   ⚡ Skipping documentation"
Write-Host ""

# Build only masternode and its dependencies (using correct package name)
cargo build --release -p time-masternode

if ($LASTEXITCODE -eq 0) {
    Write-Host ""
    Write-Host "✅ Masternode built successfully!" -ForegroundColor Green
    Write-Host ""
    Write-Host "📦 Binary location:" -ForegroundColor Yellow
    Write-Host "   target\release\time-masternode.exe"
    Write-Host ""
    Write-Host "📊 Binary size:" -ForegroundColor Yellow
    $size = (Get-Item target\release\time-masternode.exe).Length / 1MB
    Write-Host "   $([math]::Round($size, 2)) MB"
    Write-Host ""
    Write-Host "🚀 To run:" -ForegroundColor Yellow
    Write-Host "   .\target\release\time-masternode.exe"
    Write-Host ""
} else {
    Write-Host "❌ Build failed!" -ForegroundColor Red
    exit 1
}
