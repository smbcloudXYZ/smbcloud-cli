# Fails if a Windows binary depends on the Visual C++ runtime DLLs.
#
# smb is linked with +crt-static (see .cargo/config.toml) so it runs on a
# clean Windows install. If RUSTFLAGS overrides that, or a native dependency
# drags in the DLL runtime, the exe still builds and passes tests on a runner
# that has the redistributable installed, then fails to load on users'
# machines. This check catches it before release.
param(
    [Parameter(Mandatory = $true)]
    [string]$Path
)

$ErrorActionPreference = 'Stop'

$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
$dumpbin = & $vswhere -latest -products * -find 'VC\Tools\MSVC\*\bin\Hostx64\x64\dumpbin.exe' |
    Select-Object -First 1
if (-not $dumpbin) {
    throw 'dumpbin.exe not found; is the MSVC toolset installed?'
}

$output = & $dumpbin /nologo /dependents $Path
if ($LASTEXITCODE -ne 0) {
    throw "dumpbin failed on $Path"
}
$output | Write-Host

$runtime = $output |
    Where-Object { $_ -match '^\s+(vcruntime|msvcp|concrt|ucrtbase|api-ms-win-crt-)\S*\.dll\s*$' } |
    ForEach-Object { $_.Trim() }
if ($runtime) {
    Write-Host "::error::$Path depends on the Visual C++ runtime: $($runtime -join ', ')"
    exit 1
}
Write-Host "$Path does not depend on the Visual C++ runtime."
