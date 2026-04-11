$root = 'C:\Users\x\dev\ts-native\crates\tsn-runtime\src\modules'
Get-ChildItem $root -Filter '*.rs' | ForEach-Object {
    $file = $_.FullName
    $lines = Get-Content $file
    $output = New-Object System.Collections.Generic.List[string]
    foreach ($line in $lines) {
        if ($line -eq 'use tsn_op_macros::op;') {
            continue
        }
        if ($line -match '^#\[op\("[^"]+" ?\)\]$') {
            continue
        }
        if ($line -match '^fn ') {
            $line = 'pub ' + $line
        }
        $output.Add($line)
    }
    Set-Content -Path $file -Value $output
}

$table = 'C:\Users\x\dev\ts-native\crates\tsn-vm\src\intrinsic\table.rs'
$text = Get-Content -Raw $table
$text = [regex]::Replace($text, '([A-Za-z0-9_]+)::([A-Za-z0-9_]+)_OP', 'HostOp { name: "$2", func: $1::$2 }')
if ($text -notmatch 'use std::sync::Arc;') {
    $text = $text -replace '(use tsn_types::Value;\r?\n)', "use std::sync::Arc;`r`n`$1"
}
Set-Content -Path $table -Value $text
