/**
 * Browser navigation check — simulates manual walkthrough of HRM frontend.
 * Run from frontend/: node scripts/ui-nav-check.mjs
 */
import { chromium } from 'playwright';

const BASE = 'http://localhost:5174';
const EMAIL = 'admin@mashuptech.in';
const PASSWORD = 'password';

const routes = [
  { path: '/login', name: 'Login', guest: true },
  { path: '/admin/dashboard', name: 'Dashboard' },
  { path: '/admin/users', name: 'Users' },
  { path: '/admin/departments', name: 'Departments' },
  { path: '/admin/designations', name: 'Designations' },
  { path: '/admin/centers', name: 'Centers' },
  { path: '/admin/careers', name: 'Careers' },
  { path: '/admin/job-applications', name: 'Job Applications' },
  { path: '/admin/attendance', name: 'Attendance' },
  { path: '/admin/biometric', name: 'Biometric' },
  { path: '/admin/leave-requests', name: 'Leave Requests' },
  { path: '/admin/leave-requests/manage', name: 'Manage Leave' },
  { path: '/admin/holidays', name: 'Holidays' },
  { path: '/admin/salaries/components', name: 'Salary Components' },
  { path: '/admin/salaries/employees', name: 'Salary Employees' },
  { path: '/admin/payroll', name: 'Payroll' },
  { path: '/admin/workflows', name: 'Workflows' },
  { path: '/admin/tasks', name: 'Tasks' },
  { path: '/admin/projects', name: 'Projects' },
  { path: '/admin/settings/profile', name: 'Profile Settings' },
  { path: '/admin/settings/password', name: 'Password Settings' },
  { path: '/admin/settings/appearance', name: 'Appearance' },
  { path: '/admin/settings/app', name: 'App Settings' },
];

const results = [];

function log(row) {
  results.push(row);
  const icon = row.status === 'OK' ? '✓' : row.status === 'WARN' ? '!' : '✗';
  console.log(`${icon} ${row.page.padEnd(22)} ${row.status.padEnd(6)} ${row.detail || ''}`);
}

async function main() {
  let browser;
  try {
    browser = await chromium.launch({ headless: true });
  } catch (e) {
    console.error('Playwright browser not installed. Run: npx playwright install chromium');
    process.exit(2);
  }

  const context = await browser.newContext();
  const page = await context.newPage();

  const consoleErrors = [];
  const failedRequests = [];

  page.on('console', (msg) => {
    if (msg.type() === 'error') {
      const t = msg.text();
      if (!t.includes('favicon') && !t.includes('DevTools')) consoleErrors.push(t);
    }
  });

  page.on('response', (res) => {
    const url = res.url();
    if (url.includes('/api/') && res.status() >= 400) {
      failedRequests.push(`${res.status()} ${url}`);
    }
  });

  page.on('pageerror', (err) => consoleErrors.push(`PAGE ERROR: ${err.message}`));

  try {
    await page.goto(`${BASE}/login`, { waitUntil: 'networkidle', timeout: 30000 });
    const hasEmail = await page.locator('input[type="email"], input[name="email"]').count();
    log({
      page: 'Login page',
      status: hasEmail > 0 ? 'OK' : 'FAIL',
      detail: hasEmail > 0 ? 'Form visible' : 'Email input missing',
    });
  } catch (e) {
    log({ page: 'Login page', status: 'FAIL', detail: e.message });
    await browser.close();
    process.exit(1);
  }

  try {
    await page.fill('input[type="email"], input[name="email"]', EMAIL);
    await page.fill('input[type="password"], input[name="password"]', PASSWORD);
    await page.click('button[type="submit"]');
    await page.waitForURL(/\/admin\/dashboard/, { timeout: 20000 });
    log({ page: 'Login submit', status: 'OK', detail: 'Redirected to dashboard' });
  } catch (e) {
    log({ page: 'Login submit', status: 'FAIL', detail: e.message });
    await browser.close();
    process.exit(1);
  }

  for (const route of routes) {
    if (route.guest) continue;
    const errorsBefore = consoleErrors.length;
    const reqsBefore = failedRequests.length;

    try {
      await page.goto(`${BASE}${route.path}`, { waitUntil: 'networkidle', timeout: 45000 });
      await page.waitForTimeout(800);

      const url = page.url();
      const onLogin = url.includes('/login');
      const bodyText = await page.locator('body').innerText();
      const hasReactError = bodyText.includes('React Error') || bodyText.includes('⚠️ React Error');
      const hasLoadingOnly =
        bodyText.trim() === 'Loading...' || (bodyText.includes('Loading...') && bodyText.length < 80);

      const newApiFails = failedRequests.slice(reqsBefore);
      const newConsole = consoleErrors.slice(errorsBefore);

      let status = 'OK';
      let detail = '';

      if (onLogin) {
        status = 'FAIL';
        detail = 'Redirected to login (auth lost)';
      } else if (hasReactError) {
        status = 'FAIL';
        detail = 'Error boundary shown';
      } else if (hasLoadingOnly) {
        status = 'WARN';
        detail = 'Stuck on loading';
      } else if (newApiFails.length > 0) {
        status = 'WARN';
        detail = newApiFails[0].slice(0, 120);
      } else if (newConsole.length > 0) {
        status = 'WARN';
        detail = newConsole[0].slice(0, 120);
      } else {
        detail = url.replace(BASE, '') || '/';
      }

      log({ page: route.name, status, detail });
    } catch (e) {
      log({ page: route.name, status: 'FAIL', detail: e.message.slice(0, 120) });
    }
  }

  try {
    await page.goto(`${BASE}/admin/attendance`, { waitUntil: 'networkidle', timeout: 30000 });
    const clockBtn = page.getByRole('button', { name: /clock in/i });
    if ((await clockBtn.count()) > 0) {
      log({ page: 'Attendance UI', status: 'OK', detail: 'Clock-in button present' });
    } else {
      const clockOut = page.getByRole('button', { name: /clock out/i });
      log({
        page: 'Attendance UI',
        status: (await clockOut.count()) > 0 ? 'OK' : 'WARN',
        detail: (await clockOut.count()) > 0 ? 'Clock-out button present' : 'No clock buttons found',
      });
    }
  } catch (e) {
    log({ page: 'Attendance UI', status: 'WARN', detail: e.message.slice(0, 80) });
  }

  await browser.close();

  console.log('\n--- Summary ---');
  const fails = results.filter((r) => r.status === 'FAIL');
  const warns = results.filter((r) => r.status === 'WARN');
  console.log(`Total: ${results.length}  OK: ${results.filter((r) => r.status === 'OK').length}  WARN: ${warns.length}  FAIL: ${fails.length}`);

  if (consoleErrors.length) {
    console.log('\nConsole errors (sample):');
    [...new Set(consoleErrors)].slice(0, 8).forEach((e) => console.log('  -', e.slice(0, 150)));
  }
  if (failedRequests.length) {
    console.log('\nFailed API calls (sample):');
    [...new Set(failedRequests)].slice(0, 8).forEach((e) => console.log('  -', e));
  }

  process.exit(fails.length > 0 ? 1 : 0);
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
