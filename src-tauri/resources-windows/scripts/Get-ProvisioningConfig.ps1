#Requires -Version 5.1

<#
.SYNOPSIS
    Retrieves Defguard client provisioning configuration for the currently logged-in user from AD or Entra ID.

.DESCRIPTION
    This script detects whether the computer is joined to on-premises Active Directory 
    or Entra ID (Azure AD), then fetches Defguard provisioning data (URL and enrollment token) from the appropriate source.
    - On-premises AD: Reads from extensionAttribute1 and extensionAttribute2
    - Entra ID: Reads from custom security attributes under the 'Defguard' set
    - Workgroup: Exits gracefully
    The retrieved enrollment data is saved to a JSON file for the Defguard client to use.

.PARAMETER ADAttribute
    Specifies which Active Directory attribute to read from (default: extensionAttribute1)
#>

param(
    [string]$ADAttribute = "extensionAttribute1"
)

# Check device join status
function Get-DomainJoinStatus {
    try {
        $computerSystem = Get-WmiObject -Class Win32_ComputerSystem -ErrorAction Stop
        
        # Check for traditional domain join
        if ($computerSystem.PartOfDomain -eq $true) {
            return @{
                JoinType = "OnPremisesAD"
                Domain = $computerSystem.Domain
            }
        }
        
        # Check for Entra ID (Azure AD) join
        $dsregStatus = dsregcmd /status
        if ($dsregStatus -match "AzureAdJoined\s*:\s*YES") {
            $tenantName = ($dsregStatus | Select-String "TenantName\s*:\s*(.+)").Matches.Groups[1].Value.Trim()
            return @{
                JoinType = "EntraID"
                Domain = $tenantName
            }
        }
        
        # Check for Hybrid join
        if ($dsregStatus -match "DomainJoined\s*:\s*YES" -and $dsregStatus -match "AzureAdJoined\s*:\s*YES") {
            return @{
                JoinType = "Hybrid"
                Domain = $computerSystem.Domain
            }
        }
        
        # Not joined to any directory
        return @{
            JoinType = "Workgroup"
            Domain = $null
        }
        
    } catch {
        Write-Host "Unable to determine domain status: $_" -ForegroundColor Yellow
        return @{
            JoinType = "Unknown"
            Domain = $null
        }
    }
}

# Save Defguard enrollment data to JSON
function Save-DefguardEnrollmentData {
    param(
        [string]$EnrollmentUrl,
        [string]$EnrollmentToken,
        [string]$UserPrincipalName,
        [string]$DisplayName
    )
    
    # Create Defguard directory in AppData\Roaming
    $defguardDir = Join-Path $env:APPDATA "net.defguard"
    $jsonOutputPath = Join-Path $defguardDir "provisioning.json"
    
    try {
        # Create directory if it doesn't exist
        if (-not (Test-Path -Path $defguardDir)) {
            New-Item -ItemType Directory -Path $defguardDir -Force | Out-Null
            Write-Host "`nCreated directory: $defguardDir" -ForegroundColor Gray
        }
        
        $jsonData = @{
            enrollmentUrl = $EnrollmentUrl
            enrollmentToken = $EnrollmentToken
            userPrincipalName = $UserPrincipalName
            displayName = $DisplayName
            retrievedAt = (Get-Date).ToString("o")
        }
        
        $jsonData | ConvertTo-Json -Depth 10 | Out-File -FilePath $jsonOutputPath -Encoding UTF8 -Force
        Write-Host "`nDefguard enrollment data saved to: $jsonOutputPath" -ForegroundColor Green
        return $true
    } catch {
        Write-Host "`nFailed to save JSON file: $_" -ForegroundColor Red
        return $false
    }
}

# Get Defguard client provisioning config from on-premises AD
function Get-OnPremisesADProvisioningConfig {
    param(
        [string]$Username,
        [string]$ADAttribute
    )
    
    # Check if Active Directory module is available
    if (-not (Get-Module -ListAvailable -Name ActiveDirectory)) {
        Write-Host "Active Directory module is not installed. Please install RSAT tools." -ForegroundColor Red
        return
    }

    # Import the Active Directory module
    try {
        Import-Module ActiveDirectory -ErrorAction Stop
    } catch {
        Write-Host "Failed to import Active Directory module: $_" -ForegroundColor Red
        return
    }

    # Fetch AD user information
    try {
        $adUser = Get-ADUser -Identity $Username -Properties * -ErrorAction Stop
        
        # Display user information
        Write-Host "`n=== On-Premises Active Directory User Information ===" -ForegroundColor Cyan
        Write-Host "Display Name:        $($adUser.DisplayName)"
        Write-Host "Username (SAM):      $($adUser.SamAccountName)"
        Write-Host "User Principal Name: $($adUser.UserPrincipalName)"
        Write-Host "Email:               $($adUser.EmailAddress)"
        Write-Host "Enabled:             $($adUser.Enabled)"
        Write-Host "Created:             $($adUser.Created)"
        Write-Host "Distinguished Name:  $($adUser.DistinguishedName)"
        Write-Host "======================================================`n" -ForegroundColor Cyan

        # Check for Defguard enrollment data in the specified AD attribute
        Write-Host "`n--- Active Directory Attribute ---" -ForegroundColor Yellow
        
        # Read JSON data from the specified AD attribute
        $jsonData = $adUser.$ADAttribute
        
        Write-Host "Defguard Enrollment JSON ($ADAttribute): $jsonData"
        
        if ($jsonData) {
            try {
                # Parse the JSON data
                $enrollmentConfig = $jsonData | ConvertFrom-Json -ErrorAction Stop
                
                # Extract URL and token from the parsed JSON
                $enrollmentUrl = $enrollmentConfig.enrollmentUrl
                $enrollmentToken = $enrollmentConfig.enrollmentToken
                
                Write-Host "Defguard Enrollment URL:   $enrollmentUrl"
                Write-Host "Defguard Enrollment Token: $enrollmentToken"
                
                # Save enrollment data to JSON file only if both URL and token exist
                if ($enrollmentUrl -and $enrollmentToken) {
                    Save-DefguardEnrollmentData -EnrollmentUrl $enrollmentUrl `
                                                 -EnrollmentToken $enrollmentToken `
                                                 -UserPrincipalName $adUser.UserPrincipalName `
                                                 -DisplayName $adUser.DisplayName
                } else {
                    Write-Host "`nWarning: Incomplete Defguard enrollment data in JSON. Both URL and token are required." -ForegroundColor Yellow
                }
            } catch {
                Write-Host "Failed to parse JSON from AD attribute '$ADAttribute': $_" -ForegroundColor Red
                Write-Host "JSON data should be in format: {`"enrollmentUrl`":`"https://...`",`"enrollmentToken`":`"token-value`"}" -ForegroundColor Yellow
            }
        } else {
            Write-Host "No Defguard enrollment data found in the specified AD attribute." -ForegroundColor Yellow
        }
        
        Write-Host "======================================================`n" -ForegroundColor Cyan
        
        
        return
        
    } catch {
        Write-Host "Failed to retrieve AD user information for '$Username': $_" -ForegroundColor Red
        return
    }
}

# Get Defguard client provisioning config from Entra ID
function Get-EntraIDProvisioningConfig {
    # Check if Microsoft.Graph module is available
    if (-not (Get-Module -ListAvailable -Name Microsoft.Graph.Users)) {
        Write-Host "Microsoft.Graph.Users module is not installed." -ForegroundColor Yellow
        Write-Host "Install it with: Install-Module Microsoft.Graph.Users -Scope CurrentUser" -ForegroundColor Yellow
        return
    }

    # Import the module
    try {
        Import-Module Microsoft.Graph.Users -ErrorAction Stop
    } catch {
        Write-Host "Failed to import Microsoft.Graph.Users module: $_" -ForegroundColor Red
        return
    }

    # Connect to Microsoft Graph
    try {
        $context = Get-MgContext -ErrorAction SilentlyContinue
        
        if (-not $context) {
            Write-Host "Connecting to Microsoft Graph (authentication required)..." -ForegroundColor Yellow
            Write-Host "Note: Requesting additional permissions for custom security attributes..." -ForegroundColor Gray
            Connect-MgGraph -Scopes "User.Read", "CustomSecAttributeAssignment.Read.All" -ErrorAction Stop
        } else {
            # Check if we have the required scope for custom attributes
            $hasCustomAttrScope = $context.Scopes -contains "CustomSecAttributeAssignment.Read.All"
            if (-not $hasCustomAttrScope) {
                Write-Host "Warning: Missing 'CustomSecAttributeAssignment.Read.All' permission." -ForegroundColor Yellow
                Write-Host "Custom security attributes will not be available. Reconnect with:" -ForegroundColor Yellow
                Write-Host "  Connect-MgGraph -Scopes 'User.Read', 'CustomSecAttributeAssignment.Read.All'" -ForegroundColor Gray
                return
            }
        }
        
        # Get current user info including custom security attributes
        $properties = @(
            "DisplayName",
            "UserPrincipalName",
            "Mail",
            "AccountEnabled",
            "CreatedDateTime",
            "Id",
            "CustomSecurityAttributes"
        )
        
        $mgUser = Get-MgUser -UserId (Get-MgContext).Account -Property $properties -ErrorAction Stop
        
        # Display user information
        Write-Host "`n=== Entra ID (Azure AD) User Information ===" -ForegroundColor Cyan
        Write-Host "Display Name:        $($mgUser.DisplayName)"
        Write-Host "User Principal Name: $($mgUser.UserPrincipalName)"
        Write-Host "Email:               $($mgUser.Mail)"
        Write-Host "Account Enabled:     $($mgUser.AccountEnabled)"
        Write-Host "Created:             $($mgUser.CreatedDateTime)"
        Write-Host "User ID:             $($mgUser.Id)"
        
        # Try to get custom security attributes
        if ($mgUser.CustomSecurityAttributes) {
            Write-Host "`n--- Custom Security Attributes ---" -ForegroundColor Yellow
            
            # Access Defguard attributes
            if ($mgUser.CustomSecurityAttributes.AdditionalProperties) {
                $defguardAttrs = $mgUser.CustomSecurityAttributes.AdditionalProperties["Defguard"]
                
                if ($defguardAttrs) {
                    $enrollmentUrl = $defguardAttrs["EnrollmentUrl"]
                    $enrollmentToken = $defguardAttrs["EnrollmentToken"]
                    
                    Write-Host "Defguard Enrollment URL:   $enrollmentUrl"
                    Write-Host "Defguard Enrollment Token: $enrollmentToken"

                    # Save enrollment data to JSON file only if both URL and token exist
                    if ($enrollmentUrl -and $enrollmentToken) {
                        Save-DefguardEnrollmentData -EnrollmentUrl $enrollmentUrl `
                                                     -EnrollmentToken $enrollmentToken `
                                                     -UserPrincipalName $mgUser.UserPrincipalName `
                                                     -DisplayName $mgUser.DisplayName
                    } else {
                        Write-Host "`nWarning: Incomplete Defguard enrollment data. Both URL and token are required." -ForegroundColor Yellow
                    }
                } else {
                    Write-Host "No Defguard attributes found for this user." -ForegroundColor Gray
                }
            } else {
                Write-Host "No custom security attributes found." -ForegroundColor Gray
            }
        } else {
            Write-Host "`nCustom security attributes not available." -ForegroundColor Gray
            Write-Host "(May require additional permissions or attributes not set)" -ForegroundColor Gray
        }
        
        Write-Host "=============================================`n" -ForegroundColor Cyan
                
    } catch {
        Write-Host "Failed to retrieve Entra ID user information: $_" -ForegroundColor Red
        Write-Host "Error details: $($_.Exception.Message)" -ForegroundColor Red
    }
}

# Log all script output to file
$defguardDir = Join-Path $env:APPDATA "net.defguard"
$logFilePath = Join-Path $defguardDir "provisioning_log.txt"
Start-Transcript -Path $logFilePath

# Main script execution
Write-Host "Detecting domain join status..." -ForegroundColor Gray

$joinStatus = Get-DomainJoinStatus
$joinType = $joinStatus.JoinType

Write-Host "Join Type = '$joinType'" -ForegroundColor Magenta

if ($joinType -eq "OnPremisesAD") {
        Write-Host "Connected to on-premises Active Directory: $($joinStatus.Domain)" -ForegroundColor Green
        $currentUser = $env:USERNAME
        Get-OnPremisesADProvisioningConfig -Username $currentUser -ADAttribute $ADAttribute
        exit 0
    
    
} elseif ($joinType -eq "Hybrid") {
        Write-Host "Hybrid join detected (both on-premises AD and Entra ID): $($joinStatus.Domain)" -ForegroundColor Green
        Write-Host "Querying on-premises Active Directory..." -ForegroundColor Gray
        $currentUser = $env:USERNAME
        Get-OnPremisesADProvisioningConfig -Username $currentUser -ADAttribute $ADAttribute
        exit 0
    
    
} elseif ($joinType -eq "EntraID") {
        Write-Host "Connected to Entra ID (Azure AD)" -ForegroundColor Green
        if ($joinStatus.Domain) {
            Write-Host "  Tenant: $($joinStatus.Domain)" -ForegroundColor Gray
        }
        Get-EntraIDProvisioningConfig
        exit 0
    
    
} elseif ($joinType -eq "Workgroup") {
        Write-Host "This computer is not connected to a domain (Workgroup). Exiting." -ForegroundColor Yellow
        exit 0
    
    
} else {
        Write-Host "Unable to determine domain connection status. Exiting." -ForegroundColor Yellow
        exit 0
    
}
