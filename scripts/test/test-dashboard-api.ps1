#!/usr/bin/env pwsh
# Test Dashboard API Connectivity
# This script tests if the dashboard can connect to the API properly

param(
    [string]$ApiUrl = "http://localhost:3031"
)

Write-Host ""
Write-Host "╔═══════════════════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "║                                                       ║" -ForegroundColor Cyan
Write-Host "║     Dashboard API Connectivity Test                  ║" -ForegroundColor Cyan
Write-Host "║                                                       ║" -ForegroundColor Cyan
Write-Host "╚═══════════════════════════════════════════════════════╝" -ForegroundColor Cyan
Write-Host ""
Write-Host "Testing API: $ApiUrl" -ForegroundColor Yellow
Write-Host ""

$allPassed = $true

# Test 1: Health Check
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Gray
Write-Host "Test 1: Health Check" -ForegroundColor Cyan
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Gray
Write-Host "GET $ApiUrl/health"
try {
    $health = Invoke-RestMethod -Uri "$ApiUrl/health" -Method Get -TimeoutSec 5
    Write-Host "✓ PASS" -ForegroundColor Green
    Write-Host "  Status: $($health.status)" -ForegroundColor Gray
} catch {
    Write-Host "✗ FAIL" -ForegroundColor Red
    Write-Host "  Error: $($_.Exception.Message)" -ForegroundColor Red
    $allPassed = $false
}
Write-Host ""

# Test 2: Blockchain Info
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Gray
Write-Host "Test 2: Blockchain Info" -ForegroundColor Cyan
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Gray
Write-Host "GET $ApiUrl/blockchain/info"
try {
    $info = Invoke-RestMethod -Uri "$ApiUrl/blockchain/info" -Method Get -TimeoutSec 5
    Write-Host "✓ PASS" -ForegroundColor Green
    Write-Host "  Height: $($info.height)" -ForegroundColor Gray
    Write-Host "  Hash: $($info.best_block_hash.Substring(0, 16))..." -ForegroundColor Gray
    Write-Host "  Wallet: $($info.wallet_address)" -ForegroundColor Gray
    $walletAddress = $info.wallet_address
} catch {
    Write-Host "✗ FAIL" -ForegroundColor Red
    Write-Host "  Error: $($_.Exception.Message)" -ForegroundColor Red
    $allPassed = $false
    $walletAddress = $null
}
Write-Host ""

# Test 3: Balance Endpoint (if we have wallet address)
if ($walletAddress) {
    Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Gray
    Write-Host "Test 3: Balance Endpoint" -ForegroundColor Cyan
    Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Gray
    Write-Host "GET $ApiUrl/blockchain/balance/$walletAddress"
    try {
        $balance = Invoke-RestMethod -Uri "$ApiUrl/blockchain/balance/$walletAddress" -Method Get -TimeoutSec 5
        Write-Host "✓ PASS" -ForegroundColor Green
        Write-Host "  Address: $($balance.address)" -ForegroundColor Gray
        Write-Host "  Confirmed: $($balance.balance) satoshis" -ForegroundColor Gray
        Write-Host "  Unconfirmed: $($balance.unconfirmed_balance) satoshis" -ForegroundColor Gray
        
        $confirmedTIME = $balance.balance / 100000000.0
        $unconfirmedTIME = $balance.unconfirmed_balance / 100000000.0
        Write-Host "  Confirmed: $confirmedTIME TIME" -ForegroundColor Yellow
        if ($unconfirmedTIME -gt 0) {
            Write-Host "  Unconfirmed: $unconfirmedTIME TIME" -ForegroundColor Cyan
        }
    } catch {
        Write-Host "✗ FAIL" -ForegroundColor Red
        Write-Host "  Error: $($_.Exception.Message)" -ForegroundColor Red
        if ($_.Exception.Response) {
            Write-Host "  Status Code: $($_.Exception.Response.StatusCode.value__)" -ForegroundColor Red
            try {
                $reader = [System.IO.StreamReader]::new($_.Exception.Response.GetResponseStream())
                $body = $reader.ReadToEnd()
                Write-Host "  Response: $body" -ForegroundColor Red
            } catch {}
        }
        $allPassed = $false
    }
    Write-Host ""
} else {
    Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Gray
    Write-Host "Test 3: Balance Endpoint" -ForegroundColor Cyan
    Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Gray
    Write-Host "⊘ SKIP - No wallet address from blockchain info" -ForegroundColor Yellow
    Write-Host ""
}

# Test 4: Peers Endpoint
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Gray
Write-Host "Test 4: Peers Endpoint" -ForegroundColor Cyan
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Gray
Write-Host "GET $ApiUrl/peers"
try {
    $peers = Invoke-RestMethod -Uri "$ApiUrl/peers" -Method Get -TimeoutSec 5
    Write-Host "✓ PASS" -ForegroundColor Green
    Write-Host "  Peer count: $($peers.peers.Count)" -ForegroundColor Gray
} catch {
    Write-Host "✗ FAIL" -ForegroundColor Red
    Write-Host "  Error: $($_.Exception.Message)" -ForegroundColor Red
    $allPassed = $false
}
Write-Host ""

# Test 5: Mempool Endpoint
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Gray
Write-Host "Test 5: Mempool Endpoint" -ForegroundColor Cyan
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Gray
Write-Host "GET $ApiUrl/mempool"
try {
    $mempool = Invoke-RestMethod -Uri "$ApiUrl/mempool" -Method Get -TimeoutSec 5
    Write-Host "✓ PASS" -ForegroundColor Green
    Write-Host "  Pending: $($mempool.pending)" -ForegroundColor Gray
} catch {
    Write-Host "✗ FAIL" -ForegroundColor Red
    Write-Host "  Error: $($_.Exception.Message)" -ForegroundColor Red
    $allPassed = $false
}
Write-Host ""

# Summary
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Gray
Write-Host "Summary" -ForegroundColor Cyan
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Gray
if ($allPassed) {
    Write-Host "✓ ALL TESTS PASSED" -ForegroundColor Green
    Write-Host ""
    Write-Host "The dashboard should work correctly with this API endpoint." -ForegroundColor Green
    Write-Host "Run: time-dashboard --api-url $ApiUrl" -ForegroundColor Yellow
} else {
    Write-Host "✗ SOME TESTS FAILED" -ForegroundColor Red
    Write-Host ""
    Write-Host "Troubleshooting:" -ForegroundColor Yellow
    Write-Host "  1. Check if timed is running: ps aux | grep timed" -ForegroundColor Gray
    Write-Host "  2. Check API port: netstat -an | grep 3031" -ForegroundColor Gray
    Write-Host "  3. Check firewall: iptables -L | grep 3031" -ForegroundColor Gray
    Write-Host "  4. Check logs: journalctl -u timed -n 50" -ForegroundColor Gray
}
Write-Host ""
