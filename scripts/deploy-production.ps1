# Raintech HRM — production deploy from Windows dev machine
# Usage: powershell -NoProfile -File scripts/deploy-production.ps1

param(
    [string]$ServerIp = "13.232.29.223",
    [string]$SshUser = "ubuntu",
    [string]$KeyPath = "$env:USERPROFILE\Downloads\raintechHrm_key_pair.pem",
    [string]$TenantDomain = "hrm.13-232-29-223.sslip.io",
    [string]$PlatformDomain = "platform.13-232-29-223.sslip.io",
    [string]$ApiDomain = "api.13-232-29-223.sslip.io",
    [string]$GitRepo = "https://github.com/guru177/hrm-rust.git",
    [string]$GitBranch = "main",
    [switch]$SkipBootstrap,
    [switch]$SkipElectron,
    [switch]$UseGit
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
Set-Location $Root

function New-Secret([int]$Length = 48) {
    $chars = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
    -join ((1..$Length) | ForEach-Object { $chars[(Get-Random -Maximum $chars.Length)] })
}

function New-Password([int]$Length = 24) {
    $chars = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!@#%^*-_=+"
    -join ((1..$Length) | ForEach-Object { $chars[(Get-Random -Maximum $chars.Length)] })
}

$ssh = @("-i", $KeyPath, "-o", "StrictHostKeyChecking=no", "-o", "ConnectTimeout=30")
$scp = @("-i", $KeyPath, "-o", "StrictHostKeyChecking=no")

if (-not (Test-Path $KeyPath)) { throw "SSH key not found: $KeyPath" }

# Preserve production secrets on redeploy (keeps PostgreSQL volume password in sync).
$existingEnv = ""
try {
    $existingEnv = & ssh @ssh "${SshUser}@${ServerIp}" "cat /opt/hrm/deploy/.env 2>/dev/null || true"
} catch { }

function Get-EnvValue([string]$text, [string]$key) {
    if ($text -match "(?m)^$key=(.+)$") { return $Matches[1].Trim().Trim('"') }
    return $null
}

$jwt = Get-EnvValue $existingEnv "JWT_SECRET"
if (-not $jwt) { $jwt = New-Secret 48 }

$pgPass = Get-EnvValue $existingEnv "POSTGRES_PASSWORD"
if (-not $pgPass) { $pgPass = New-Secret 32 }

$platPass = Get-EnvValue $existingEnv "PLATFORM_ADMIN_PASSWORD"
if (-not $platPass) { $platPass = "Raintech$(New-Password 16)!" }

# Optional: merge SMTP/signup from local backend .env
$smtpBlock = @()
$localEnv = Join-Path $Root "backend\.env"
if (Test-Path $localEnv) {
    Get-Content $localEnv | ForEach-Object {
        if ($_ -match '^(SMTP_|MSG91_|ALLOW_PUBLIC_SIGNUP|TENANT_APP_URL|AWS_|GEMINI_|CLOUDFRONT_|RAZORPAY_)') {
            $smtpBlock += $_
        }
    }
}

$tenantUrl = "https://$TenantDomain"
$platformUrl = "https://$PlatformDomain"

$deployEnv = @"
TENANT_DOMAIN=$TenantDomain
PLATFORM_DOMAIN=$PlatformDomain
API_DOMAIN=$ApiDomain
ACME_EMAIL=info@raintechpos.com

POSTGRES_DB=hrm
POSTGRES_USER=hrm
POSTGRES_PASSWORD=$pgPass

JWT_SECRET=$jwt
PLATFORM_ADMIN_EMAIL=admin@retaildaddy.in
PLATFORM_ADMIN_PASSWORD=$platPass
PLATFORM_ADMIN_NAME="Platform Admin"

BIOMETRIC_STRICT_IP=1
CORS_ORIGINS=$tenantUrl,$platformUrl
TRUST_PROXY=1
ALLOW_PUBLIC_SIGNUP=1
TENANT_APP_URL=$tenantUrl

VITE_TENANT_APP_URL=$tenantUrl
VITE_PLATFORM_APP_URL=$platformUrl

SIGNUP_OTP_DEBUG=0
SIGNUP_OTP_BYPASS=0
RUST_LOG=info
$($smtpBlock -join "`n")
"@

function Write-Utf8NoBom([string]$Path, [string]$Content) {
    $utf8 = New-Object System.Text.UTF8Encoding $false
    [System.IO.File]::WriteAllText($Path, $Content, $utf8)
}

$deployDir = Join-Path $Root "deploy"
New-Item -ItemType Directory -Force -Path $deployDir | Out-Null
Write-Utf8NoBom (Join-Path $deployDir ".env") $deployEnv

# Electron production API (packaged desktop → production HTTPS)
$prodApi = @{
    apiBase = $tenantUrl
    tenantAppUrl = $tenantUrl
    platformAppUrl = $platformUrl
} | ConvertTo-Json
Write-Utf8NoBom (Join-Path $Root "frontend\electron\production-api.json") $prodApi

if ($UseGit) {
    Write-Host "==> Git deploy to $ServerIp (pull + build on server)..."
    & scp @scp (Join-Path $deployDir ".env") "${SshUser}@${ServerIp}:/tmp/hrm-deploy.env"
    & scp @scp (Join-Path $deployDir "remote-git-setup.sh") "${SshUser}@${ServerIp}:/tmp/"
    & scp @scp (Join-Path $deployDir "remote-git-deploy.sh") "${SshUser}@${ServerIp}:/tmp/"
    & scp @scp (Join-Path $deployDir "remote-deploy.sh") "${SshUser}@${ServerIp}:/tmp/"

    $gitRemote = @"
set -e
export HRM_GIT_REPO='$GitRepo'
export HRM_GIT_BRANCH='$GitBranch'
sed -i 's/\r$//' /tmp/remote-git-setup.sh /tmp/remote-git-deploy.sh /tmp/remote-deploy.sh 2>/dev/null || true
chmod +x /tmp/remote-git-setup.sh /tmp/remote-git-deploy.sh /tmp/remote-deploy.sh
if [ ! -d /opt/hrm/.git ]; then
  bash /tmp/remote-git-setup.sh
  cp /tmp/remote-git-deploy.sh /opt/hrm/deploy/remote-git-deploy.sh
  cp /tmp/remote-deploy.sh /opt/hrm/deploy/remote-deploy.sh
  chmod +x /opt/hrm/deploy/remote-git-deploy.sh /opt/hrm/deploy/remote-deploy.sh
fi
cp /tmp/hrm-deploy.env /opt/hrm/deploy/.env
bash /opt/hrm/deploy/remote-git-deploy.sh 2>&1 | tee /tmp/hrm-deploy.log
if ! grep -q 'Deploy finished' /tmp/hrm-deploy.log; then exit 1; fi
"@
    $gitRemote = ($gitRemote -replace "`r`n", "`n") -replace "`r", ""
    $gitRemote | & ssh @ssh "${SshUser}@${ServerIp}" "bash -s"

    Write-Host ""
    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host "PRODUCTION DEPLOYED (git)" -ForegroundColor Green
    Write-Host "Tenant:    $tenantUrl"
    Write-Host "Platform:  $platformUrl"
    Write-Host "Biometric: http://${ServerIp}:7788"
    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host "Future deploys on server: cd /opt/hrm && bash deploy/remote-git-deploy.sh" -ForegroundColor Yellow
    exit 0
}

Write-Host "==> Building tenant + platform frontends locally..."
Push-Location (Join-Path $Root "frontend")
$env:VITE_API_URL = "$tenantUrl/api"
$env:VITE_PLATFORM_APP_URL = $platformUrl
npm run build
if ($LASTEXITCODE -ne 0) { Pop-Location; throw "frontend build failed" }
Pop-Location
Push-Location (Join-Path $Root "platform")
$env:VITE_TENANT_APP_URL = $tenantUrl
npm run build
if ($LASTEXITCODE -ne 0) { Pop-Location; throw "platform build failed" }
Pop-Location

Write-Host "==> Packaging project (excluding node_modules, target, release)..."
$archive = Join-Path $env:TEMP "hrm-deploy.tgz"
if (Test-Path $archive) { Remove-Item $archive -Force }
& tar -czf $archive `
    --exclude=node_modules `
    --exclude=backend/target `
    --exclude=frontend/release `
    --exclude=frontend/node_modules `
    --exclude=platform/node_modules `
    --exclude=.git `
    -C $Root .

Write-Host "==> Uploading to $ServerIp..."
& scp @scp $archive "${SshUser}@${ServerIp}:/tmp/hrm-deploy.tgz"
& scp @scp (Join-Path $deployDir ".env") "${SshUser}@${ServerIp}:/tmp/hrm-deploy.env"
& scp @scp (Join-Path $deployDir "remote-bootstrap.sh") "${SshUser}@${ServerIp}:/tmp/"
& scp @scp (Join-Path $deployDir "remote-deploy.sh") "${SshUser}@${ServerIp}:/tmp/"

$remote = @"
set -e
sudo mkdir -p /opt/hrm && sudo chown ubuntu:ubuntu /opt/hrm
cd /opt/hrm
tar -xzf /tmp/hrm-deploy.tgz -C /opt/hrm
cp /tmp/hrm-deploy.env /opt/hrm/deploy/.env
chmod +x /tmp/remote-bootstrap.sh /tmp/remote-deploy.sh

"@

if (-not $SkipBootstrap) {
    $remote += "bash /tmp/remote-bootstrap.sh`n"
}

$remote += "sed -i 's/\r$//' /tmp/remote-deploy.sh /tmp/remote-bootstrap.sh 2>/dev/null || true`n"
$remote += "`nbash /tmp/remote-deploy.sh 2>&1 | tee /tmp/hrm-deploy.log`n"

$remote = ($remote -replace "`r`n", "`n") -replace "`r", ""
$remote | & ssh @ssh "${SshUser}@${ServerIp}" "bash -s"

Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "PRODUCTION DEPLOYED" -ForegroundColor Green
Write-Host "Tenant:    $tenantUrl"
Write-Host "Platform:  $platformUrl"
Write-Host "Biometric: http://${ServerIp}:7788"
Write-Host "Platform admin: admin@retaildaddy.in / (see deploy/.env on server)"
Write-Host "========================================" -ForegroundColor Cyan

if (-not $SkipElectron) {
    Write-Host "==> Building Electron installer for production..."
    Push-Location (Join-Path $Root "frontend")
    try {
        $env:VITE_API_URL = "$tenantUrl/api"
        $env:VITE_PLATFORM_APP_URL = $platformUrl
        npm run electron:build 2>&1
        if ($LASTEXITCODE -ne 0) { throw "electron:build failed" }
        node scripts/publish-desktop-update.cjs
        $releaseFiles = Get-ChildItem (Join-Path $Root "storage\desktop-updates") -File
        foreach ($f in $releaseFiles) {
            & scp @scp $f.FullName "${SshUser}@${ServerIp}:/tmp/desktop-updates-$($f.Name)"
        }
        $uploadDesktop = @"
sudo mkdir -p /var/lib/docker/volumes/deploy_hrm_data/_data/storage/desktop-updates 2>/dev/null || true
VOL=`$(docker volume inspect deploy_hrm_data --format '{{.Mountpoint}}' 2>/dev/null || echo '')
if [ -n "`$VOL" ]; then
  sudo mkdir -p "`$VOL/storage/desktop-updates"
  sudo cp /tmp/desktop-updates-* "`$VOL/storage/desktop-updates/" 2>/dev/null || true
  sudo chown -R 999:999 "`$VOL/storage" 2>/dev/null || true
fi
"@
        $uploadDesktop | & ssh @ssh "${SshUser}@${ServerIp}" "bash -s"
        Write-Host "Electron installer published to production update feed." -ForegroundColor Green
    } finally {
        Pop-Location
    }
}

Write-Host "Save deploy/.env locally - it contains production secrets." -ForegroundColor Yellow
