$ErrorActionPreference = 'Stop'

$toolsDir = "$(Split-Path -parent $MyInvocation.MyCommand.Definition)"
$exePath  = Join-Path $toolsDir 'service-bus-explorer-tui.exe'

if (Test-Path $exePath) {
  Remove-Item $exePath -Force
}
