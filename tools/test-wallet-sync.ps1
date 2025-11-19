# Test wallet sync connectivity
Write-Host "=== Testing Wallet Sync Connectivity ===" -ForegroundColor Cyan

# Test 1: Can we reach the masternode?
Write-Host "`n[Test 1] Testing TCP connectivity to masternode..." -ForegroundColor Yellow
$masternodes = @("161.35.129.70:24100", "69.167.168.176:24100")
foreach ($mn in $masternodes) {
    $parts = $mn -split ":"
    $ip = $parts[0]
    $port = $parts[1]
    
    $result = Test-NetConnection -ComputerName $ip -Port $port -WarningAction SilentlyContinue
    if ($result.TcpTestSucceeded) {
        Write-Host "  ✓ $mn is reachable" -ForegroundColor Green
    } else {
        Write-Host "  ✗ $mn is NOT reachable" -ForegroundColor Red
    }
}

# Test 2: WebSocket port
Write-Host "`n[Test 2] Testing WebSocket connectivity (port 24002)..." -ForegroundColor Yellow
foreach ($mn in $masternodes) {
    $parts = $mn -split ":"
    $ip = $parts[0]
    
    $result = Test-NetConnection -ComputerName $ip -Port 24002 -WarningAction SilentlyContinue
    if ($result.TcpTestSucceeded) {
        Write-Host "  ✓ $ip:24002 WebSocket is reachable" -ForegroundColor Green
    } else {
        Write-Host "  ✗ $ip:24002 WebSocket is NOT reachable" -ForegroundColor Red
    }
}

Write-Host "`n=== Test Complete ===" -ForegroundColor Cyan
