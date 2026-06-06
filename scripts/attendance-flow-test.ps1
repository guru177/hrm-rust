# Attendance module API flow test
$Base = "http://localhost:5174/api"
$Email = "admin@mashuptech.in"
$Password = "password"
$Results = @()

function Log($Step, $Status, $Detail = "") {
    $script:Results += [pscustomobject]@{ Step = $Step; Status = $Status; Detail = $Detail }
    $icon = if ($Status -eq "OK") { "[OK]" } elseif ($Status -eq "WARN") { "[!!]" } else { "[FAIL]" }
    Write-Host "$icon $Step $(if ($Detail) { "- $Detail" })"
}

function Api($Method, $Path, $Body, $Token) {
    $h = @{ "Content-Type" = "application/json" }
    if ($Token) { $h["Authorization"] = "Bearer $Token" }
    $uri = "$Base$Path"
    $p = @{ Uri = $uri; Method = $Method; Headers = $h; UseBasicParsing = $true; TimeoutSec = 30 }
    if ($null -ne $Body) { $p.Body = ($Body | ConvertTo-Json -Depth 10) }
    try {
        $r = Invoke-WebRequest @p
        return @{ Ok = $true; Code = $r.StatusCode; Json = ($r.Content | ConvertFrom-Json); Raw = $r.Content }
    } catch {
        $code = if ($_.Exception.Response) { [int]$_.Exception.Response.StatusCode } else { 0 }
        $body = ""
        try { $sr = [System.IO.StreamReader]::new($_.Exception.Response.GetResponseStream()); $body = $sr.ReadToEnd() } catch {}
        return @{ Ok = $false; Code = $code; Err = $_.Exception.Message; Body = $body }
    }
}

Write-Host "=== Attendance Module Test ===`n" -ForegroundColor Cyan

$login = Api POST "/auth/login" @{ email = $Email; password = $Password } $null
if (-not $login.Ok) { Log "Login" "FAIL" $login.Code; exit 1 }
$token = $login.Json.data.token
Log "Login" "OK"

$today = Api GET "/admin/attendance/today" $null $token
if ($today.Ok) {
    Log "GET /today" "OK" "sessions=$($today.Json.data.total_sessions)"
} else { Log "GET /today" "FAIL" $today.Code }

$stats = Api GET "/admin/attendance/stats" $null $token
if ($stats.Ok) {
    Log "GET /stats" "OK" "present=$($stats.Json.data.present_days) hours=$($stats.Json.data.total_hours)"
} else { Log "GET /stats" "FAIL" $stats.Code }

$list = Api GET "/admin/attendance/list?page=1&per_page=5" $null $token
if ($list.Ok) {
    $rows = $list.Json.data.data
    Log "GET /list" "OK" "total=$($list.Json.data.total) rows=$($rows.Count)"
} else { Log "GET /list" "FAIL" $list.Code }

$users = Api GET "/admin/attendance/users" $null $token
if ($users.Ok) { Log "GET /users" "OK" "count=$($users.Json.data.Count)" } else { Log "GET /users" "FAIL" $users.Code }

# Clock-in flow
$cin = Api POST "/admin/attendance/clock-in" @{ face_verified = $false; face_match_score = $null } $token
if ($cin.Ok) { Log "POST clock-in" "OK" $cin.Json.data.message } else { Log "POST clock-in" "FAIL" "$($cin.Code) $($cin.Body)" }

$today2 = Api GET "/admin/attendance/today" $null $token
$active = $today2.Json.data.active_clock_in
if ($active) { Log "Active session after clock-in" "OK" "id=$($active.id)" } else { Log "Active session after clock-in" "FAIL" "no active_clock_in" }

# Double clock-in (should auto-close previous or reject)
$cin2 = Api POST "/admin/attendance/clock-in" @{ face_verified = $false } $token
if ($cin2.Ok) { Log "POST second clock-in" "OK" "message=$($cin2.Json.data.message)" } else { Log "POST second clock-in" "WARN" "blocked or error: $($cin2.Code)" }

$openCount = 0
if ($today2.Json.data.attendances) {
    foreach ($s in (Api GET "/admin/attendance/today" $null $token).Json.data.attendances) {
        if (-not $s.clock_out) { $openCount++ }
    }
}
Log "Open sessions count" $(if ($openCount -le 1) { "OK" } else { "FAIL" }) "$openCount open (expect <=1)"

# Clock-out
$cout = Api POST "/admin/attendance/clock-out" @{} $token
if ($cout.Ok) { Log "POST clock-out" "OK" "duration=$($cout.Json.data.duration_minutes) min" } else { Log "POST clock-out" "FAIL" "$($cout.Code) $($cout.Body)" }

# Clock-out without active session
$cout2 = Api POST "/admin/attendance/clock-out" @{} $token
if (-not $cout2.Ok -and $cout2.Code -eq 400) { Log "Double clock-out rejected" "OK" } else { Log "Double clock-out rejected" "FAIL" "expected 400" }

Write-Host "`n--- Summary ---"
$Results | Format-Table -AutoSize
$fails = @($Results | Where-Object Status -eq "FAIL")
exit $(if ($fails.Count -gt 0) { 1 } else { 0 })
