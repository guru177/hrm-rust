# API input flow test — same payloads the UI sends
$Base = "http://localhost:5174/api"
$Email = "admin@mashuptech.in"
$Password = "password"
$Ts = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
$Results = @()
$Created = @{ departments = @(); designations = @(); holidays = @(); tasks = @(); workflows = @(); projects = @(); centers = @(); leave = @() }

function Log($Module, $Step, $Status, $Detail = "") {
    $script:Results += [pscustomobject]@{ Module = $Module; Step = $Step; Status = $Status; Detail = $Detail }
    Write-Host "[$Status] $Module — $Step $(if ($Detail) { "($Detail)" })"
}

function Api($Method, $Path, $Body, $Token) {
    $h = @{ "Content-Type" = "application/json"; "Authorization" = "Bearer $Token" }
    $uri = "$Base$Path"
    $p = @{ Uri = $uri; Method = $Method; Headers = $h; UseBasicParsing = $true; TimeoutSec = 30 }
    if ($null -ne $Body) { $p.Body = ($Body | ConvertTo-Json -Depth 10) }
    try {
        $r = Invoke-WebRequest @p
        return @{ Ok = $true; Code = $r.StatusCode; Json = ($r.Content | ConvertFrom-Json) }
    } catch {
        $code = if ($_.Exception.Response) { [int]$_.Exception.Response.StatusCode } else { 0 }
        return @{ Ok = $false; Code = $code; Err = $_.Exception.Message }
    }
}

Write-Host "=== API Input Flow Test ===`n"
$login = Api POST "/auth/login" @{ email = $Email; password = $Password } $null
if (-not $login.Ok) { Write-Host "Login failed"; exit 1 }
$token = $login.Json.data.token
Log "Auth" "Login" "OK"

# Attendance: clock in → today → clock out
$cin = Api POST "/admin/attendance/clock-in" @{ face_verified = $false; face_match_score = $null } $token
Log "Attendance" "Clock in (no face)" $(if ($cin.Ok) { "OK" } else { "FAIL" }) $cin.Code
$today = Api GET "/admin/attendance/today" $null $token
$active = $today.Json.data.active_clock_in
Log "Attendance" "Today shows active session" $(if ($active) { "OK" } else { "WARN" }) ""
$cout = Api POST "/admin/attendance/clock-out" @{} $token
Log "Attendance" "Clock out" $(if ($cout.Ok) { "OK" } else { "FAIL" }) ""

# Department CRUD
$deptName = "API Dept $Ts"
$d = Api POST "/admin/departments" @{ name = $deptName; description = "test"; is_active = $true } $token
if ($d.Ok -and $d.Json.data.id) { $Created.departments += $d.Json.data.id; Log "Departments" "Create" "OK" $deptName }
else { Log "Departments" "Create" "FAIL" $d.Code }

# Designation
$desName = "API Desig $Ts"
$des = Api POST "/admin/designations" @{ name = $desName; description = "test"; is_active = $true } $token
if ($des.Ok) { Log "Designations" "Create" "OK" $desName } else { Log "Designations" "Create" "FAIL" $des.Code }

# Holiday
$hol = Api POST "/admin/holidays" @{ name = "API Hol $Ts"; date = "2099-07-04"; is_paid = $true; description = "test" } $token
if ($hol.Ok) { Log "Holidays" "Create" "OK" } else { Log "Holidays" "Create" "FAIL" $hol.Code }

# Leave request
$lv = Api POST "/admin/leave-requests" @{
    leave_type = "annual"; start_date = "2099-08-01"; end_date = "2099-08-03"; reason = "API flow test"
} $token
if ($lv.Ok) { Log "Leave" "Submit request" "OK" } else { Log "Leave" "Submit" "FAIL" $lv.Code }

# Task
$task = Api POST "/admin/tasks" @{
    title = "API Task $Ts"; description = "d"; status = "todo"; priority = "medium"; type = "other"
    assigned_to = "unassigned"; project_id = "none"; related_type = "none"
} $token
if ($task.Ok) { Log "Tasks" "Create" "OK" } else { Log "Tasks" "Create" "FAIL" $task.Code }

# Workflow
$wf = Api POST "/admin/workflows" @{
    name = "API WF $Ts"; description = "d"; trigger_type = "leave_request_submitted"
    actions = @(@{ type = "send_notification"; config = @{} }); is_active = $true
} $token
if ($wf.Ok) { Log "Workflows" "Create" "OK" } else { Log "Workflows" "Create" "FAIL" $wf.Code }

# Project
$pr = Api POST "/admin/projects" @{
    name = "API Proj $Ts"; description = "d"; status = "active"; priority = "medium"
} $token
if ($pr.Ok) { Log "Projects" "Create" "OK" } else { Log "Projects" "Create" "FAIL" $pr.Code }

# Center
$ct = Api POST "/admin/api/settings/centers" @{
    name = "API Center $Ts"; address_line1 = "1 St"; place = "P"; city = "C"; state = "S"; pincode = "1"
} $token
if ($ct.Ok) { Log "Centers" "Create" "OK" } else { Log "Centers" "Create" "FAIL" $ct.Code }

# Settings patch
$prof = Api PATCH "/admin/settings/profile" @{ phone = "9999999999" } $token
Log "Settings" "Profile patch (phone)" $(if ($prof.Ok) { "OK" } else { "FAIL" }) ""

# Payroll preview
$now = Get-Date
$empPath = '/admin/payroll/employees?month=' + $now.Month + '&year=' + $now.Year
$emps = Api GET $empPath $null $token
$ids = @(1)
if ($emps.Ok -and $emps.Json.data -and $emps.Json.data.Count -gt 0) { $ids = @($emps.Json.data[0].id) }
$prev = Api POST "/admin/payroll/preview" @{ month = $now.Month; year = $now.Year; employee_ids = $ids } $token
Log "Payroll" "Preview with employee_ids" $(if ($prev.Ok) { "OK" } else { "FAIL" }) ""

Write-Host "`n--- Done ---"
$Results | Format-Table -AutoSize
$fails = @($Results | Where-Object Status -eq "FAIL")
exit $(if ($fails.Count -gt 0) { 1 } else { 0 })
