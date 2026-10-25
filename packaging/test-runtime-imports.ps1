<#
.SYNOPSIS
  Reject executable imports that require a separately installed VC runtime.
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory)]
    [string]$Path
)

$ErrorActionPreference = 'Stop'
$stream = [System.IO.File]::OpenRead((Resolve-Path -LiteralPath $Path).Path)
$reader = [System.IO.BinaryReader]::new($stream)
try {
    $stream.Position = 0x3c
    $peOffset = $reader.ReadUInt32()
    $stream.Position = $peOffset
    if ($reader.ReadUInt32() -ne 0x4550) { throw 'Not a PE executable' }
    $stream.Position = $peOffset + 6
    $sectionCount = $reader.ReadUInt16()
    $stream.Position = $peOffset + 20
    $optionalSize = $reader.ReadUInt16()
    $optionalOffset = $peOffset + 24
    $stream.Position = $optionalOffset
    $magic = $reader.ReadUInt16()
    $directoryOffset = switch ($magic) {
        0x20b { $optionalOffset + 112 }
        0x10b { $optionalOffset + 96 }
        default { throw 'Unsupported PE optional header' }
    }
    $sections = for ($i = 0; $i -lt $sectionCount; $i++) {
        $stream.Position = $optionalOffset + $optionalSize + 40 * $i + 8
        $virtualSize = $reader.ReadUInt32()
        $address = $reader.ReadUInt32()
        $rawSize = $reader.ReadUInt32()
        $rawOffset = $reader.ReadUInt32()
        [pscustomobject]@{
            Address = $address
            Size = [Math]::Max($virtualSize, $rawSize)
            Offset = $rawOffset
        }
    }
    function Get-FileOffset([uint32]$Rva) {
        foreach ($section in $sections) {
            if ($Rva -ge $section.Address -and $Rva -lt $section.Address + $section.Size) {
                return [long]($section.Offset + $Rva - $section.Address)
            }
        }
        throw "PE address is outside the sections: $Rva"
    }
    function Get-ImportName([uint32]$Rva) {
        $stream.Position = Get-FileOffset $Rva
        $name = [System.Text.StringBuilder]::new()
        while (($value = $reader.ReadByte()) -ne 0) {
            [void]$name.Append([char]$value)
        }
        return $name.ToString()
    }

    $imports = @(
        # Standard imports and delay-load imports both need checking.
        foreach ($table in @(
            @{ Index = 1; EntrySize = 20; NameOffset = 12 },
            @{ Index = 13; EntrySize = 32; NameOffset = 4 }
        )) {
            $stream.Position = $directoryOffset + 8 * $table.Index
            $tableRva = $reader.ReadUInt32()
            $tableSize = $reader.ReadUInt32()
            if ($tableRva -eq 0) { continue }
            $tableOffset = Get-FileOffset $tableRva
            for ($offset = 0; $offset + $table.EntrySize -le $tableSize; $offset += $table.EntrySize) {
                $stream.Position = $tableOffset + $offset
                $entry = $reader.ReadBytes($table.EntrySize)
                if (-not ($entry | Where-Object { $_ -ne 0 })) { break }
                if ($table.Index -eq 13 -and ([BitConverter]::ToUInt32($entry, 0) -band 1) -eq 0) {
                    throw 'Unsupported delay import using virtual addresses'
                }
                Get-ImportName ([BitConverter]::ToUInt32($entry, $table.NameOffset))
            }
        }
    ) | Sort-Object -Unique
    Write-Host "DLL imports: $($imports -join ', ')"
    $externalRuntime = @($imports | Where-Object { $_ -match '^(vcruntime|msvcp|msvcr|concrt|vcomp)\d.*\.dll$' })
    if ($externalRuntime.Count -gt 0) {
        throw "Executable requires an external Visual C++ runtime: $($externalRuntime -join ', '). Rebuild with crt-static."
    }
    Write-Host 'PASS: No external Visual C++ runtime imports.'
}
finally {
    $reader.Dispose()
    $stream.Dispose()
}
