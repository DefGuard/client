$ErrorActionPreference = 'Stop'
$toolsDir   = "$(Split-Path -parent $MyInvocation.MyCommand.Definition)"
$url        = 'https://github.com/DefGuard/client/releases/download/v1.6.5/Defguard_1.6.5_x64_en-US.msi'


$packageArgs = @{
  packageName   = $env:ChocolateyPackageName
  unzipLocation = $toolsDir
  fileType      = 'msi'
  url           = $url

  softwareName  = 'defguard*'

  checksum      = 'be99afe71ab88e0add4905721471d0d40935c33ae7cdb47084ba53a91d675cc7'
  checksumType  = 'sha256'


  silentArgs    = "/qn /norestart /l*v `"$($env:TEMP)\$($packageName).$($env:chocolateyPackageVersion).MsiInstall.log`""
  validExitCodes= @(0, 3010, 1641)
}

Install-ChocolateyPackage @packageArgs
Write-Warning "IMPORTANT: Reboot or Re-login Required: On initial install the user is added to the defguard group.A reboot or logging out and back in is required for group membership changes to take effect. This is not required on subsequent updates."