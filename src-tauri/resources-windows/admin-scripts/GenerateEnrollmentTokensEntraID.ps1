param(
    [Parameter(Mandatory=$true)]
    [string]$Url,
    
    [Parameter(Mandatory=$true)]
    [string]$ApiToken,
    
    [Parameter(Mandatory=$true)]
    [string]$GroupName,
    
    [Parameter(Mandatory=$false)]
    [string]$AttributeSetName = "Defguard"
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


# Function to find user in Entra ID by username using Microsoft.Graph module
function Get-EntraIDUser {
    param(
        [string]$Username
    )
    
    # Try different user properties to find the user
    try {
        # Try by UserPrincipalName first
        $user = Get-MgUser -Filter "userPrincipalName eq '$Username'" -ErrorAction Stop
        if ($user) { return $user }
    } catch { }
    
    try {
        # Try by mail
        $user = Get-MgUser -Filter "mail eq '$Username'" -ErrorAction Stop
        if ($user) { return $user }
    } catch { }
    
    try {
        # Try by display name
        $user = Get-MgUser -Filter "displayName eq '$Username'" -ErrorAction Stop
        if ($user) { return $user }
    } catch { }
    
    try {
        # Try starts with userPrincipalName
        $user = Get-MgUser -Filter "startswith(userPrincipalName,'$Username@')" -ErrorAction Stop
        if ($user) { return $user }
    } catch { }
    
    return $null
}

# Function to update Entra ID user custom security attributes using Microsoft.Graph module
function Set-EntraIDUserEnrollmentToken {
    param(
        [string]$UserId,
        [string]$EnrollmentToken,
        [string]$EnrollmentUrl,
        [string]$AttributeSetName
    )
    
    try {
        # Build the custom security attributes payload
        $attributes = @{
            "$AttributeSetName" = @{
                "@odata.type" = "#microsoft.graph.customSecurityAttributeValue"
                "enrollmentToken" = $EnrollmentToken
                "enrollmentUrl" = $EnrollmentUrl
            }
        }
        
        # Update user with custom security attributes
        Update-MgUser -UserId $UserId -CustomSecurityAttributes $attributes -ErrorAction Stop
        
        Write-Host "  Successfully updated Entra ID custom security attributes for user ID: $UserId" -ForegroundColor Green
        return $true
    }
    catch {
        Write-Host "  Failed to update Entra ID custom security attributes for user ID: $UserId" -ForegroundColor Red
        Write-Host "  Error: $($_.Exception.Message)" -ForegroundColor Red
        
        # Check if it's a permissions error
        if ($_.Exception.Message -like "*403*" -or $_.Exception.Message -like "*Forbidden*") {
            Write-Host "  This is likely a permissions issue. Ensure the current user has:" -ForegroundColor Yellow
            Write-Host "  - CustomSecurityAttributes.ReadWrite.All permission" -ForegroundColor Yellow
            Write-Host "  - User.ReadWrite.All permission" -ForegroundColor Yellow
        }
        return $false
    }
}

# Main script execution
Write-Host "Starting enrollment token generation for Entra ID users in group: $GroupName" -ForegroundColor Green

# Check for Microsoft.Graph module and install/import if needed
$requiredModules = @(
    "Microsoft.Graph.Authentication",
    "Microsoft.Graph.Users",
    "Microsoft.Graph.Users.Actions"
)

foreach ($module in $requiredModules) {
    if (-not (Get-Module -ListAvailable -Name $module)) {
        Write-Host "$module module is required but not installed. Attempting to install..." -ForegroundColor Yellow
        try {
            Install-Module $module -Scope CurrentUser -Force -ErrorAction Stop
            Write-Host "$module module installed successfully" -ForegroundColor Green
        }
        catch {
            Write-Error "Failed to install $module module: $($_.Exception.Message)"
            Write-Host "Please install it manually using: Install-Module $module -Scope CurrentUser" -ForegroundColor Red
            exit 1
        }
    }
    
    # Import the module
    try {
        Import-Module $module -Force -ErrorAction Stop
        Write-Host "$module module imported successfully" -ForegroundColor Green
    }
    catch {
        Write-Error "Failed to import $module module: $($_.Exception.Message)"
        exit 1
    }
}

# Connect to Microsoft Graph
Write-Host "Connecting to Microsoft Graph..." -ForegroundColor Yellow
try {                                                                                                                                       
    # Check if we're already connected                                                                                                      
    $context = Get-MgContext -ErrorAction SilentlyContinue                                                                                  
    if ($context -and $context.Scopes -contains "CustomSecAttributeAssignment.ReadWrite.All" -and $context.Scopes -contains                     
"User.ReadWrite.All") {                                                                                                                     
        Write-Host "Already connected to Microsoft Graph with required permissions" -ForegroundColor Green                                  
    } else {                                                                                                                                
        # Disconnect from any existing Graph sessions first                                                                                 
        Disconnect-MgGraph -ErrorAction SilentlyContinue                                                                                    
                                                                                                                                            
        # Try authentication methods in order of preference                                                                                 
        Write-Host "Attempting authentication methods..." -ForegroundColor Cyan                                                             
                                                                                                                                            
        # Method 1: Try integrated Windows authentication (best for domain-joined)                                                          
        try {                                                                                                                               
            Write-Host "Trying Integrated Windows Authentication..." -ForegroundColor Yellow                                                
            Connect-MgGraph -Scopes @('CustomSecAttributeAssignment.ReadWrite.All', 'User.ReadWrite.All') -UseDeviceCode -ErrorAction Stop      
            Write-Host "Successfully connected using device code flow" -ForegroundColor Green                                               
        }                                                                                                                                   
        catch {                                                                                                                             
            # Method 2: Fall back to interactive browser                                                                                    
            Write-Host "Device code flow failed, trying interactive browser..." -ForegroundColor Yellow                                     
            Connect-MgGraph -Scopes @('CustomSecAttributeAssignment.ReadWrite.All', 'User.ReadWrite.All') -ErrorAction Stop                     
            Write-Host "Successfully connected using interactive authentication" -ForegroundColor Green                                     
        }                                                                                                                                   
    }                                                                                                                                       
}                                                                                                                                           
catch {                                                                                                                                     
    Write-Error "Failed to connect to Microsoft Graph: $($_.Exception.Message)"                                                             
    Write-Host "Authentication troubleshooting:" -ForegroundColor Red                                                                       
    Write-Host "  - Ensure you're on a domain-joined machine and connected to corporate network" -ForegroundColor Yellow                    
    Write-Host "  - Verify you have the required Entra ID permissions" -ForegroundColor Yellow                                              
    Write-Host "  - Check that custom security attributes are enabled in your tenant" -ForegroundColor Yellow                               
    Write-Host "  - Complete any MFA prompts if required" -ForegroundColor Yellow                                                           
    Write-Host "Required permissions: CustomSecAttributeAssignment.ReadWrite.All, User.ReadWrite.All" -ForegroundColor Cyan                     
    exit 1                                                                                                                                  
}                       

# Get group members from Defguard
Write-Host "Fetching group members from Defguard..." -ForegroundColor Yellow
$groupEndpoint = "api/v1/group/$GroupName"
$groupResponse = Invoke-AuthenticatedRestMethod -Method "GET" -Endpoint $groupEndpoint

if (-not $groupResponse) {
    Write-Error "Failed to fetch group members from Defguard"
    exit 1
}

# Extract usernames from the response
$usernames = $groupResponse.members

if (-not $usernames -or $usernames.Count -eq 0) {
    Write-Host "No members found in group: $GroupName" -ForegroundColor Yellow
    exit 0
}

Write-Host "Found $($usernames.Count) members in the group" -ForegroundColor Green

# Arrays to store results
$enrollmentTokens = @()
$entraUpdateResults = @()

# Loop through each user and generate enrollment token, then update Entra ID
foreach ($username in $usernames) {
    Write-Host "Processing user: $username" -ForegroundColor Cyan
    
    # Generate enrollment token from Defguard
    $enrollmentEndpoint = "api/v1/user/$username/start_enrollment"
    $requestBody = @{
        email = $null
        send_enrollment_notification = $false
    }
    
    $enrollmentResponse = Invoke-AuthenticatedRestMethod -Method "POST" -Endpoint $enrollmentEndpoint -Body $requestBody
    
    if ($enrollmentResponse) {
        Write-Host "  Enrollment token generated for $username" -ForegroundColor Green
        
        # Find user in Entra ID
        Write-Host "  Searching for user in Entra ID..." -ForegroundColor Yellow
        $entraUser = Get-EntraIDUser -Username $username
        
        if ($entraUser) {
            Write-Host "  Found user in Entra ID: $($entraUser.UserPrincipalName) (ID: $($entraUser.Id))" -ForegroundColor Green
            
            # Update Entra ID custom security attributes
            $updateResult = Set-EntraIDUserEnrollmentToken -UserId $entraUser.Id -EnrollmentToken $enrollmentResponse.enrollment_token -EnrollmentUrl $enrollmentResponse.enrollment_url -AttributeSetName $AttributeSetName
            
            $entraUpdateResults += @{
                username = $username
                entra_id = $entraUser.Id
                user_principal_name = $entraUser.UserPrincipalName
                success = $updateResult
                enrollment_token = $enrollmentResponse.enrollment_token
                enrollment_url = $enrollmentResponse.enrollment_url
            }
        }
        else {
            Write-Host "  User $username not found in Entra ID" -ForegroundColor Red
            $entraUpdateResults += @{
                username = $username
                entra_id = $null
                user_principal_name = $null
                success = $false
                enrollment_token = $enrollmentResponse.enrollment_token
                enrollment_url = $enrollmentResponse.enrollment_url
            }
        }
    }
    else {
        Write-Host "  Failed to generate enrollment token for $username" -ForegroundColor Red
        $entraUpdateResults += @{
            username = $username
            entra_id = $null
            user_principal_name = $null
            success = $false
            enrollment_token = $null
            enrollment_url = $null
        }
    }
}

# Output summary
Write-Host "`nEnrollment token generation and Entra ID update completed!" -ForegroundColor Green
$successfulEntraUpdates = ($entraUpdateResults | Where-Object { $_.success }).Count
Write-Host "Successfully updated Entra ID custom security attributes: $successfulEntraUpdates/$($usernames.Count)" -ForegroundColor $(if ($successfulEntraUpdates -eq $usernames.Count) { "Green" } else { "Yellow" })

# Disconnect from Microsoft Graph
Write-Host "`nDisconnecting from Microsoft Graph..." -ForegroundColor Yellow
Disconnect-MgGraph -ErrorAction SilentlyContinue
Write-Host "Disconnected successfully" -ForegroundColor Green
