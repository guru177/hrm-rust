# HRM end-to-end API flow test (via Vite proxy)
$Base = "http://localhost:5174/api"
$Email = "admin@mashuptech.in"
$Password = "password"
$Results = @()

function Test-Endpoint {
    param(
        [string]$Name,
        [string]$Method,
        [string]$Path,
        [object]$Body = $null,
        [int[]]$ExpectStatus = @(200),
        [string]$Token = $null
    )
    $headers = @{ "Content-Type" = "application/json" }
    if ($Token) { $headers["Authorization"] = "Bearer $Token" }
    $uri = "$Base$Path"
    try {
        $params = @{ Uri = $uri; Method = $Method; Headers = $headers; UseBasicParsing = $true; TimeoutSec = 30 }
        if ($Body -ne $null) { $params["Body"] = ($Body | ConvertTo-Json -Depth 10) }
        $r = Invoke-WebRequest @params
        $ok = $ExpectStatus -contains $r.StatusCode
        $script:Results += [pscustomobject]@{ Test = $Name; Status = if ($ok) { "PASS" } else { "FAIL" }; Code = $r.StatusCode; Detail = $r.StatusCode }
        return $r
    } catch {
        $code = $null
        if ($_.Exception.Response) { $code = [int]$_.Exception.Response.StatusCode }
        $ok = $code -and ($ExpectStatus -contains $code)
        $script:Results += [pscustomobject]@{ Test = $Name; Status = if ($ok) { "PASS" } else { "FAIL" }; Code = $code; Detail = $_.Exception.Message }
        return $null
    }
}

Write-Host "=== HRM Flow Test ===" -ForegroundColor Cyan

# Frontend shell
try {
    $fe = Invoke-WebRequest -Uri "http://localhost:5174/" -UseBasicParsing -TimeoutSec 10
    $Results += [pscustomobject]@{ Test = "Frontend (Vite)"; Status = if ($fe.StatusCode -eq 200) { "PASS" } else { "FAIL" }; Code = $fe.StatusCode; Detail = "index.html" }
} catch {
    $Results += [pscustomobject]@{ Test = "Frontend (Vite)"; Status = "FAIL"; Code = ""; Detail = "Not running on 5174" }
}

Test-Endpoint "Health" GET "/health"

$login = Test-Endpoint "Login" POST "/auth/login" @{ email = $Email; password = $Password }
if (-not $login) { $Results | Format-Table -AutoSize; exit 1 }
$json = $login.Content | ConvertFrom-Json
$token = $json.data.token
if (-not $token) { $token = $json.token }
Write-Host "Logged in as $($json.data.user.email)" -ForegroundColor Green

Test-Endpoint "Auth /me" GET "/auth/me" -Token $token

# Core modules (GET list/stats used by UI)
$endpoints = @(
    @("Dashboard HR data", "GET", "/admin/dashboard/hr-data"),
    @("Users list", "GET", "/admin/users/list"),
    @("Users stats", "GET", "/admin/users/stats"),
    @("Departments list", "GET", "/admin/departments/list"),
    @("Departments stats", "GET", "/admin/departments/stats"),
    @("Designations list", "GET", "/admin/designations/list"),
    @("Designations stats", "GET", "/admin/designations/stats"),
    @("Roles list", "GET", "/admin/roles/list"),
    @("Permissions list", "GET", "/admin/permissions/list"),
    @("Attendance today", "GET", "/admin/attendance/today"),
    @("Attendance stats", "GET", "/admin/attendance/stats"),
    @("Attendance list", "GET", "/admin/attendance/list?page=1&per_page=10"),
    @("Leave stats", "GET", "/admin/leave-requests/stats"),
    @("Leave list", "GET", "/admin/leave-requests/list"),
    @("Leave manage list", "GET", "/admin/leave-requests/manage/list"),
    @("Leave manage stats", "GET", "/admin/leave-requests/manage/stats"),
    @("Holidays list", "GET", "/admin/holidays/list"),
    @("Payroll stats", "GET", "/admin/payroll/stats"),
    @("Payroll list", "GET", "/admin/payroll/list"),
    @("Payroll employees", "GET", "/admin/payroll/employees"),
    @("Salary components", "GET", "/admin/salaries/components/list"),
    @("Salary employees", "GET", "/admin/salaries/employees/list"),
    @("Salary filter options", "GET", "/admin/salaries/employees/filter-options"),
    @("Job applications list", "GET", "/admin/job-applications/list"),
    @("Job applications stats", "GET", "/admin/job-applications/stats"),
    @("Careers list", "GET", "/admin/careers/list"),
    @("Reports attendance", "GET", "/admin/reports/attendance-summary"),
    @("Reports payroll", "GET", "/admin/reports/payroll-register"),
    @("Public careers", "GET", "/public/careers"),
    @("Tasks list", "GET", "/admin/tasks/list"),
    @("Projects list", "GET", "/admin/projects/list"),
    @("Workflows list", "GET", "/admin/workflows/list"),
    @("Settings app", "GET", "/admin/settings/app"),
    @("Centers", "GET", "/admin/api/settings/centers"),
    @("Biometric devices", "GET", "/admin/biometric/devices"),
    @("Biometric stats", "GET", "/admin/biometric/stats"),
    @("Biometric punches", "GET", "/admin/biometric/punches"),
    @("Biometric mapping", "GET", "/admin/biometric/mapping")
)

foreach ($e in $endpoints) {
    Test-Endpoint $e[0] $e[1] $e[2] -Token $token
}

# Known missing routes (expect 404 - document gaps)
Test-Endpoint "Workflow duplicate" POST "/admin/workflows/1/duplicate" -Token $token -ExpectStatus @(200, 201, 404)
Test-Endpoint "Work locations (missing?)" GET "/admin/api/settings/work-locations" -Token $token -ExpectStatus @(404, 405, 200)
Test-Endpoint "Interview centers (missing?)" GET "/admin/api/settings/interview-centers" -Token $token -ExpectStatus @(404, 405, 200)
Test-Endpoint "Settings save (app)" POST "/admin/settings/app" @{ smtp_host = "smtp.test.local" } -Token $token

# Attendance flow: clock in then out
$cin = Test-Endpoint "Clock in" POST "/admin/attendance/clock-in" @{ method = "manual" } -Token $token
Test-Endpoint "Clock out" POST "/admin/attendance/clock-out" @{} -Token $token

# Payroll preview (needs employee_ids array)
$now = Get-Date
$empResp = Test-Endpoint "Payroll employees for preview" GET "/admin/payroll/employees?month=$($now.Month)&year=$($now.Year)" -Token $token
$empIds = @(1)
if ($empResp) {
    $empJson = $empResp.Content | ConvertFrom-Json
    if ($empJson.data -and $empJson.data.Count -gt 0) {
        $empIds = @($empJson.data[0].id)
    }
}
Test-Endpoint "Payroll preview" POST "/admin/payroll/preview" @{ month = $now.Month; year = $now.Year; employee_ids = $empIds } -Token $token

Write-Host ""
$Results | Format-Table -AutoSize
$failed = @($Results | Where-Object { $_.Status -eq "FAIL" })
$passed = @($Results | Where-Object { $_.Status -eq "PASS" })
Write-Host "Passed: $($passed.Count)  Failed: $($failed.Count)" -ForegroundColor $(if ($failed.Count -eq 0) { "Green" } else { "Yellow" })
if ($failed.Count -gt 0) {
    Write-Host "Failures:" -ForegroundColor Red
    $failed | ForEach-Object { Write-Host "  - $($_.Test): $($_.Detail) (HTTP $($_.Code))" }
}
exit $(if ($failed.Count -gt 0) { 1 } else { 0 })
