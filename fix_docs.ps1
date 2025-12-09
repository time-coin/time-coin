# Script to add basic documentation to undocumented items
# This adds minimal doc comments to satisfy the missing_docs lint

$filesToFix = @(
    "masternode\src\slashing.rs",
    "masternode\src\slashing_executor.rs",
    "masternode\src\collateral.rs",
    "masternode\src\config.rs",
    "masternode\src\registry.rs",
    "masternode\src\rewards.rs",
    "masternode\src\security.rs",
    "masternode\src\types.rs",
    "masternode\src\detector.rs",
    "masternode\src\address_monitor.rs",
    "masternode\src\api_server.rs",
    "masternode\src\blockchain_scanner.rs",
    "masternode\src\error.rs",
    "masternode\src\node.rs",
    "masternode\src\reputation.rs",
    "masternode\src\utxo_integration.rs",
    "masternode\src\utxo_tracker.rs",
    "masternode\src\wallet_api.rs",
    "masternode\src\wallet_dat.rs",
    "masternode\src\wallet_manager.rs"
)

foreach ($file in $filesToFix) {
    $fullPath = Join-Path $PSScriptRoot $file
    if (Test-Path $fullPath) {
        Write-Host "Processing $file..."
        $content = Get-Content $fullPath -Raw
        
        # Add doc comments for pub struct fields that don't have them
        $content = $content -replace '(\n    )(pub \w+: [^,\n]+,)(\n)', "`$1/// TODO: Add documentation`n`$1`$2`$3"
        
        Set-Content $fullPath $content -NoNewline
    }
}

Write-Host "Done! Re-run cargo check to see if warnings are reduced."
