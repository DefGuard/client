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


# Function to find user in Entra ID by email address using Microsoft.Graph module
function Get-EntraIDUser {
    param(
        [string]$Email
    )
    
    Write-Host "    Searching for user with email: $Email" -ForegroundColor Cyan
    
    # Try to find user by mail (primary email)
    try {
        Write-Host "      Trying search by 'mail' property..." -ForegroundColor Gray
        $user = Get-MgUser -Filter "mail eq '$Email'" -ErrorAction Stop
        if ($user) { 
            Write-Host "    Found user by mail: $($user.UserPrincipalName)" -ForegroundColor Green
            return $user 
        }
    } 
    catch { 
        Write-Host "      Error searching by mail: $($_.Exception.Message)" -ForegroundColor Red
        Write-Host "      Full error details:" -ForegroundColor Red
        Write-Host "        Exception Type: $($_.Exception.GetType().FullName)" -ForegroundColor Red
        Write-Host "        Inner Exception: $($_.Exception.InnerException.Message)" -ForegroundColor Red
    }
    
    # Try to find user by userPrincipalName (often matches email)
    try {
        Write-Host "      Trying search by 'userPrincipalName' property..." -ForegroundColor Gray
        $user = Get-MgUser -Filter "userPrincipalName eq '$Email'" -ErrorAction Stop
        if ($user) { 
            Write-Host "    Found user by userPrincipalName: $($user.UserPrincipalName)" -ForegroundColor Green
            return $user 
        }
    } 
    catch { 
        Write-Host "      Error searching by userPrincipalName: $($_.Exception.Message)" -ForegroundColor Red
        Write-Host "      Full error details:" -ForegroundColor Red
        Write-Host "        Exception Type: $($_.Exception.GetType().FullName)" -ForegroundColor Red
        Write-Host "        Inner Exception: $($_.Exception.InnerException.Message)" -ForegroundColor Red
    }
    
    # Try other mail properties if the above don't work
    try {
        Write-Host "      Trying search by 'otherMails' property..." -ForegroundColor Gray
        $user = Get-MgUser -Filter "otherMails/any(m:m eq '$Email')" -ErrorAction Stop
        if ($user) { 
            Write-Host "    Found user by otherMails: $($user.UserPrincipalName)" -ForegroundColor Green
            return $user 
        }
    } 
    catch { 
        Write-Host "      Error searching by otherMails: $($_.Exception.Message)" -ForegroundColor Red
        Write-Host "      Full error details:" -ForegroundColor Red
        Write-Host "        Exception Type: $($_.Exception.GetType().FullName)" -ForegroundColor Red
        Write-Host "        Inner Exception: $($_.Exception.InnerException.Message)" -ForegroundColor Red
    }
    
    # Try a broader search to see if we can find any users at all
    try {
        Write-Host "      Testing basic user query..." -ForegroundColor Gray
        $testUser = Get-MgUser -Top 1 -ErrorAction Stop
        Write-Host "      Basic user query successful - permissions appear valid" -ForegroundColor Green
    }
    catch {
        Write-Host "      Basic user query failed: $($_.Exception.Message)" -ForegroundColor Red
        Write-Host "      This suggests a permissions issue with User.ReadWrite.All" -ForegroundColor Red
    }
    
    Write-Host "    User not found in Entra ID for email: $Email" -ForegroundColor Yellow
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
                "EnrollmentToken" = $EnrollmentToken
                "EnrollmentUrl" = $EnrollmentUrl
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
        
        return $false
    }
}

# Main script execution
Write-Host "Starting enrollment token generation for Entra ID users in group: $GroupName" -ForegroundColor Green

# Check for Microsoft.Graph module and install/import if needed
Write-Host "Setting up Microsoft.Graph modules..." -ForegroundColor Yellow

# Install only the specific modules we need to avoid function capacity issues
$requiredModules = @(
    "Microsoft.Graph.Authentication",
    "Microsoft.Graph.Users"
)

foreach ($module in $requiredModules) {
    try {
        # Check if module is installed
        if (-not (Get-Module -ListAvailable -Name $module)) {
            Write-Host "$module module is required but not installed. Attempting to install..." -ForegroundColor Yellow
            Install-Module $module -Scope CurrentUser -Force -ErrorAction Stop
            Write-Host "$module module installed successfully" -ForegroundColor Green
        } else {
            # Update to latest version to avoid dependency issues
            Write-Host "Updating $module module to latest version..." -ForegroundColor Yellow
            Update-Module $module -Force -ErrorAction SilentlyContinue
            Write-Host "$module module is up to date" -ForegroundColor Green
        }
        
        # Import the module
        Import-Module $module -Force -ErrorAction Stop
        Write-Host "$module module imported successfully" -ForegroundColor Green
    }
    catch {
        Write-Error "Failed to setup $module module: $($_.Exception.Message)"
        Write-Host "Please try installing manually: Install-Module $module -Scope CurrentUser -Force" -ForegroundColor Red
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
                                                                                                                                            
        # Method 1: Try interactive browser authentication                                                                                    
        try {                                                                                                                               
            Write-Host "Trying interactive browser authentication" -ForegroundColor Yellow                                     
            Connect-MgGraph -Scopes @('CustomSecAttributeAssignment.ReadWrite.All', 'User.ReadWrite.All') -ErrorAction Stop                     
            Write-Host "Successfully connected using interactive authentication" -ForegroundColor Green                                     
        }                                                                                                                                   
        catch {                                                                                                                             
            # Method 2: Fall back to device code authentication                                                                                    
            Write-Host "Interactive browser authentication failed, trying device code flow..." -ForegroundColor Yellow                                     
            Connect-MgGraph -Scopes @('CustomSecAttributeAssignment.ReadWrite.All', 'User.ReadWrite.All') -UseDeviceCode -ErrorAction Stop      
            Write-Host "Successfully connected using device code flow" -ForegroundColor Green                                               
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

# Loop through each username, fetch user details to get email, then generate enrollment token and update Entra ID
foreach ($username in $usernames) {
    Write-Host "Processing user: $username" -ForegroundColor Cyan
    
    # Get user details from Defguard to fetch email
    $userDetailsEndpoint = "api/v1/user/$username"
    $userDetailsResponse = Invoke-AuthenticatedRestMethod -Method "GET" -Endpoint $userDetailsEndpoint
    
    if (-not $userDetailsResponse -or -not $userDetailsResponse.user -or -not $userDetailsResponse.user.email) {
        Write-Host "  Failed to fetch user details or email not found for user: $username" -ForegroundColor Red
        $entraUpdateResults += @{
            username = $username
            email = $null
            entra_id = $null
            user_principal_name = $null
            success = $false
            enrollment_token = $null
            enrollment_url = $null
        }
        continue
    }
    
    $userEmail = $userDetailsResponse.user.email
    Write-Host "  Found email: $userEmail" -ForegroundColor Green
    
    # Generate enrollment token from Defguard using username
    $enrollmentEndpoint = "api/v1/user/$username/start_enrollment"
    $requestBody = @{
        email = $null
        send_enrollment_notification = $false
    }
    
    $enrollmentResponse = Invoke-AuthenticatedRestMethod -Method "POST" -Endpoint $enrollmentEndpoint -Body $requestBody
    
    if ($enrollmentResponse) {
        Write-Host "  Enrollment token generated for $username" -ForegroundColor Green
        
        # Find user in Entra ID by email
        Write-Host "  Searching for user in Entra ID by email..." -ForegroundColor Yellow
        $entraUser = Get-EntraIDUser -Email $userEmail
        
        if ($entraUser) {
            Write-Host "  Found user in Entra ID: $($entraUser.UserPrincipalName) (ID: $($entraUser.Id))" -ForegroundColor Green
            
            # Update Entra ID custom security attributes
            $updateResult = Set-EntraIDUserEnrollmentToken -UserId $entraUser.Id -EnrollmentToken $enrollmentResponse.enrollment_token -EnrollmentUrl $enrollmentResponse.enrollment_url -AttributeSetName $AttributeSetName
            
            $entraUpdateResults += @{
                username = $username
                email = $userEmail
                entra_id = $entraUser.Id
                user_principal_name = $entraUser.UserPrincipalName
                success = $updateResult
                enrollment_token = $enrollmentResponse.enrollment_token
                enrollment_url = $enrollmentResponse.enrollment_url
            }
        }
        else {
            Write-Host "  User with email $userEmail not found in Entra ID" -ForegroundColor Red
            $entraUpdateResults += @{
                username = $username
                email = $userEmail
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
            email = $userEmail
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
