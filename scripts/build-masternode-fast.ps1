# Optimized masternode build - skips docs and unnecessary features
# This is the fastest way to build for production deployment

Write-Host "🔨 Building TIME Coin Masternode (ultra-optimized)..." -ForegroundColor Cyan
Write-Host "   ⚡ Skipping documentation"
Write-Host "   ⚡ Release mode with optimizations"
Write-Host ""

# Set environment variable to skip building docs
$env:CARGO_PROFILE_RELEASE_BUILD_OVERRIDE_DEBUG = "false"

# Build masternode without docs, with minimal features (using correct package name)
cargo build --release -p time-masternode --no-default-features

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
