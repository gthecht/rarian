# Add this to you Powershell Profile in order to change the terminal title to your current location
Remove-Alias cd
Function cd ($pathChange) {
  Set-Location $pathChange
  $Host.UI.RawUI.WindowTitle = "PowerShell " + $(Get-Location)
}
