$root = 'C:\Users\x\dev\ts-native\crates\tsn-runtime\src\modules'
Get-ChildItem $root -Filter '*.rs' | ForEach-Object {
    $file = $_.FullName
    $lines = Get-Content $file
    $output = New-Object System.Collections.Generic.List[string]
    $seenOpImport = $false
    $previousLine = $null

    foreach ($line in $lines) {
        if ($line -eq 'use tsn_op_macros::op;') {
            if ($seenOpImport) {
                continue
            }
            $seenOpImport = $true
        }

        if ($line -match '^#\[op\("([^"]+)" \)\]$' -and $previousLine -eq $line) {
            continue
        }

        $output.Add($line)
        $previousLine = $line
    }

    Set-Content -Path $file -Value $output
}
