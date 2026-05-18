$ErrorActionPreference = 'Stop'
$toolsDir   = "$(Split-Path -parent $MyInvocation.MyCommand.Definition)"
$url        = 'https://github.com/DefGuard/client/releases/download/v1.6.8/Defguard_1.6.8_x64_en-US.msi'


$packageArgs = @{
  packageName   = $env:ChocolateyPackageName
  unzipLocation = $toolsDir
  fileType      = 'msi'
  url           = $url

  softwareName  = 'defguard*'

  checksum      = 'f7291e9d74cc270445bc1adc2624c2b74289f2276221f1c355f96d1db021871b'
  checksumType  = 'sha256'


  silentArgs    = "/qn /norestart /l*v `"$($env:TEMP)\$($packageName).$($env:chocolateyPackageVersion).MsiInstall.log`""
  validExitCodes= @(0, 3010, 1641)
}

Install-ChocolateyPackage @packageArgs
Write-Warning "IMPORTANT: Reboot or Re-login Required: On initial install the user is added to the defguard group.A reboot or logging out and back in is required for group membership changes to take effect. This is not required on subsequent updates."
