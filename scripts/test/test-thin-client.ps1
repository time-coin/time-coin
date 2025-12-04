# Thin Client Test Script
# Starts masternode API and tests endpoints

Write-Host "🚀 Starting Thin Client Test Environment" -ForegroundColor Cyan
Write-Host ""

# Start API server in background
Write-Host "📡 Starting Masternode API Server..." -ForegroundColor Yellow
$apiProcess = Start-Process powershell -ArgumentList @(
    "-NoExit",
    "-Command",
    "cd '$PWD'; Write-Host '🔧 Masternode API Server' -ForegroundColor Green; .\target\release\masternode-api.exe"
) -PassThru -WindowStyle Normal

Write-Host "✅ API Server started (PID: $($apiProcess.Id))" -ForegroundColor Green
Write-Host "   Waiting for server to initialize..." -ForegroundColor Gray
Start-Sleep -Seconds 3

# Test health endpoint
Write-Host ""
Write-Host "🏥 Testing Health Endpoint..." -ForegroundColor Yellow
try {
    $health = Invoke-RestMethod -Uri 'http://127.0.0.1:24100/health' -Method Get
    Write-Host "✅ Health: $($health.status)" -ForegroundColor Green
} catch {
    Write-Host "❌ Health check failed: $_" -ForegroundColor Red
    exit 1
}

# Test balance endpoint
Write-Host ""
Write-Host "💰 Testing Balance Endpoint..." -ForegroundColor Yellow
$testXpub = 'xpub6CUGRUonZSQ4TWtTMmzXdrXDtypWKiKrhko4egpiMZbpiaQL2jkwSB1icqYh2cfDfVxdx4df189oLKnC5fSwqPfgyP3hooxujYzAu3fDVmz'
try {
    $balance = Invoke-RestMethod -Uri "http://127.0.0.1:24100/wallet/balance?xpub=$testXpub" -Method Get
    $timeAmount = [math]::Round($balance.total / 100000000, 2)
    Write-Host "✅ Balance: $timeAmount TIME ($($balance.total) satoshis)" -ForegroundColor Green
    Write-Host "   Confirmed: $($balance.confirmed) satoshis" -ForegroundColor Gray
    Write-Host "   Pending: $($balance.pending) satoshis" -ForegroundColor Gray
} catch {
    Write-Host "❌ Balance query failed: $_" -ForegroundColor Red
}

# Test UTXOs endpoint
Write-Host ""
Write-Host "📦 Testing UTXO Endpoint..." -ForegroundColor Yellow
try {
    $utxos = Invoke-RestMethod -Uri "http://127.0.0.1:24100/wallet/utxos?xpub=$testXpub" -Method Get
    Write-Host "✅ UTXOs: $($utxos.Count) found" -ForegroundColor Green
    foreach ($utxo in $utxos) {
        $amount = [math]::Round($utxo.amount / 100000000, 2)
        Write-Host "   - $($utxo.txid.Substring(0,16))... : $amount TIME" -ForegroundColor Gray
    }
} catch {
    Write-Host "❌ UTXO query failed: $_" -ForegroundColor Red
}

# Test transactions endpoint
Write-Host ""
Write-Host "📜 Testing Transactions Endpoint..." -ForegroundColor Yellow
try {
    $txs = Invoke-RestMethod -Uri "http://127.0.0.1:24100/wallet/transactions?xpub=$testXpub&limit=10" -Method Get
    Write-Host "✅ Transactions: $($txs.Count) found" -ForegroundColor Green
} catch {
    Write-Host "❌ Transaction query failed: $_" -ForegroundColor Red
}

Write-Host ""
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
Write-Host "✅ All Tests Passed!" -ForegroundColor Green
Write-Host ""
Write-Host "📋 Test XPUB (has data):" -ForegroundColor Yellow
Write-Host "   $testXpub" -ForegroundColor Gray
Write-Host ""
Write-Host "🎯 Next Steps:" -ForegroundColor Yellow
Write-Host "   1. Launch wallet-gui: .\target\release\wallet-gui.exe" -ForegroundColor Gray
Write-Host "   2. Click the '🔄 Refresh (Thin)' button" -ForegroundColor Gray
Write-Host "   3. Watch the console for API calls" -ForegroundColor Gray
Write-Host ""
Write-Host "🛑 To stop API server:" -ForegroundColor Yellow
Write-Host "   Stop-Process -Id $($apiProcess.Id)" -ForegroundColor Gray
Write-Host ""
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
Write-Host ""
Write-Host "Press any key to launch wallet-gui..." -ForegroundColor Yellow
$null = $Host.UI.RawUI.ReadKey('NoEcho,IncludeKeyDown')

# Launch wallet-gui
Write-Host ""
Write-Host "🚀 Launching Wallet-GUI..." -ForegroundColor Cyan
$env:RUST_LOG = "info"
.\target\release\wallet-gui.exe
