param(
    [Parameter(Mandatory=$true)]
    [string]$Url,
    
    [Parameter(Mandatory=$true)]
    [string]$ApiToken,
    
    [Parameter(Mandatory=$true)]
    [string]$GroupName,
    
    [Parameter(Mandatory=$true)]
    [string]$ADAttribute,
    
    [Parameter(Mandatory=$false)]
    [string]$ADUsername,
    
    [Parameter(Mandatory=$false)]
    [string]$DomainController,
    
    [Parameter(Mandatory=$false)]
    [string]$EnrollmentTokenExpirationTime
)

# Function to make authenticated API calls to Defguard
function Invoke-AuthenticatedRestMethod {
    param(
        [string]$Method,
        [string]$Endpoint,
        [object]$Body = $null
    )
    
    $headers = @{
        "Authorization" = "Bearer $ApiToken"
        "Content-Type" = "application/json"
        "Accept" = "application/json"
    }
    
    $uri = "$Url/$Endpoint"
    
    try {
        if ($Body) {
            $jsonBody = $Body | ConvertTo-Json
            $response = Invoke-RestMethod -Uri $uri -Method $Method -Headers $headers -Body $jsonBody
        } else {
            $response = Invoke-RestMethod -Uri $uri -Method $Method -Headers $headers
        }
        return $response
    }
    catch {
        Write-Error "Defguard API call failed: $($_.Exception.Message)"
        return $null
    }
}

# Function to update Active Directory user attribute
function Set-ADUserEnrollmentToken {
    param(
        [string]$Username,
        [string]$EnrollmentToken,
        [string]$EnrollmentUrl,
        [string]$AttributeName,
        [System.Management.Automation.PSCredential]$Credential
    )
    
    try {
        # Build parameters for AD cmdlets
        $adParams = @{
            Identity = $Username
            Properties = $AttributeName
            ErrorAction = "Stop"
        }
        
        # Add credential if provided
        if ($Credential) {
            $adParams["Credential"] = $Credential
        }
        
        # Add domain controller if provided
        if ($DomainController) {
            $adParams["Server"] = $DomainController
        }
        
        # Verify user exists in Active Directory (result not stored, just checking for errors)
        Get-ADUser @adParams | Out-Null
        
        # Create JSON object to store in AD attribute
        $enrollmentData = @{
            enrollmentToken = $EnrollmentToken
            enrollmentUrl = $EnrollmentUrl
        }
        
        $jsonData = $enrollmentData | ConvertTo-Json -Compress
        
        # Update AD user attribute
        $setParams = @{
            Identity = $Username
            Replace = @{$AttributeName = $jsonData}
            ErrorAction = "Stop"
        }
        
        # Add credential if provided
        if ($Credential) {
            $setParams["Credential"] = $Credential
        }
        
        # Add domain controller if provided
        if ($DomainController) {
            $setParams["Server"] = $DomainController
        }
        
        Set-ADUser @setParams
        
        Write-Host "  Successfully updated AD attribute for $Username" -ForegroundColor Green
        return $true
    }
    catch {
        Write-Host "  Failed to update AD attribute for $Username : $($_.Exception.Message)" -ForegroundColor Red
        return $false
    }
}

# Main script execution
Write-Host "Fetching group members for group: $GroupName" -ForegroundColor Green

# Handle AD authentication
$ADCredential = $null
if ($ADUsername) {
    Write-Host "Using provided AD credentials for authentication" -ForegroundColor Yellow
    $ADPassword = Read-Host -Prompt "Enter AD password for $ADUsername" -AsSecureString
    $ADCredential = New-Object System.Management.Automation.PSCredential($ADUsername, $ADPassword)
} else {
    Write-Host "Using current user context for AD authentication" -ForegroundColor Yellow
}

# Get group members
Write-Host "Fetching group members from Defguard..." -ForegroundColor Yellow
$groupEndpoint = "api/v1/group/$GroupName"
$groupResponse = Invoke-AuthenticatedRestMethod -Method "GET" -Endpoint $groupEndpoint

if (-not $groupResponse) {
    Write-Error "Failed to fetch group members"
    exit 1
}

# Extract usernames from the response
$usernames = $groupResponse.members

if (-not $usernames -or $usernames.Count -eq 0) {
    Write-Host "No members found in group: $GroupName" -ForegroundColor Yellow
    exit 0
}

Write-Host "Found $($usernames.Count) members in the group" -ForegroundColor Green

# Import Active Directory module
try {
    Import-Module ActiveDirectory -ErrorAction Stop
    Write-Host "Active Directory module loaded successfully" -ForegroundColor Green
}
catch {
    Write-Error "Failed to load Active Directory module: $($_.Exception.Message)"
    exit 1
}

# Test AD connectivity
try {
    $testParams = @{ Filter = "Name -like '*'" }
    if ($ADCredential) { 
        $testParams["Credential"] = $ADCredential 
        Write-Host "Testing AD connectivity with provided credentials..." -ForegroundColor Yellow
    } else {
        Write-Host "Testing AD connectivity with current user context..." -ForegroundColor Yellow
    }
    if ($DomainController) { $testParams["Server"] = $DomainController }
    
    Get-ADUser @testParams -ResultSetSize 1 | Out-Null
    Write-Host "Active Directory connectivity test successful" -ForegroundColor Green
}
catch {
    Write-Error "Active Directory connectivity test failed: $($_.Exception.Message)"
    Write-Host "Please check your credentials, domain controller, and network connectivity" -ForegroundColor Red
    exit 1
}

# Array to store enrollment tokens
$enrollmentTokens = @()
$adUpdateResults = @()

# Loop through each user and generate enrollment token
foreach ($username in $usernames) {
    Write-Host "Processing user: $username" -ForegroundColor Cyan
    
    $enrollmentEndpoint = "api/v1/user/$username/start_enrollment"
    $requestBody = @{
        email = $null
        send_enrollment_notification = $false
    }
    
    # Add token expiration time if provided
    if ($EnrollmentTokenExpirationTime) {
        $requestBody["token_expiration_time"] = $EnrollmentTokenExpirationTime
    }
    
    $enrollmentResponse = Invoke-AuthenticatedRestMethod -Method "POST" -Endpoint $enrollmentEndpoint -Body $requestBody
    
    if ($enrollmentResponse) {
        $tokenInfo = @{
            username = $username
            enrollment_token = $enrollmentResponse.enrollment_token
            enrollment_url = $enrollmentResponse.enrollment_url
        }
        $enrollmentTokens += $tokenInfo
        
        Write-Host "  Enrollment token generated for $username" -ForegroundColor Green
        
        # Update Active Directory
        $adResult = Set-ADUserEnrollmentToken -Username $username -EnrollmentToken $enrollmentResponse.enrollment_token -EnrollmentUrl $enrollmentResponse.enrollment_url -AttributeName $ADAttribute -Credential $ADCredential
        
        $adUpdateResults += @{
            username = $username
            success = $adResult
            enrollment_token = $enrollmentResponse.enrollment_token
            enrollment_url = $enrollmentResponse.enrollment_url
        }
    }
    else {
        Write-Host "  Failed to generate enrollment token for $username" -ForegroundColor Red
        $adUpdateResults += @{
            username = $username
            success = $false
            enrollment_token = $null
            enrollment_url = $null
        }
    }
}

# Output summary
Write-Host "Enrollment token generation and AD update completed!" -ForegroundColor Green
$successfulADUpdates = ($adUpdateResults | Where-Object { $_.success }).Count
Write-Host "Successfully updated AD attributes: $successfulADUpdates/$($usernames.Count)" -ForegroundColor $(if ($successfulADUpdates -eq $usernames.Count) { "Green" } else { "Yellow" })
