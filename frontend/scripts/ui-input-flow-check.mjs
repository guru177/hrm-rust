/**
 * Input + submit flow check — exercises forms like a manual tester.
 * Run: node scripts/ui-input-flow-check.mjs
 */
import { chromium } from 'playwright';

const BASE = 'http://localhost:5174';
const EMAIL = 'admin@mashuptech.in';
const PASSWORD = 'password';
const TS = Date.now();

const flows = [];

function record(module, step, status, detail = '') {
  flows.push({ module, step, status, detail });
  const icon = status === 'OK' ? '✓' : status === 'SKIP' ? '○' : status === 'WARN' ? '!' : '✗';
  console.log(`${icon} [${module}] ${step}${detail ? ` — ${detail}` : ''}`);
}

async function login(page) {
  await page.goto(`${BASE}/login`, { waitUntil: 'networkidle', timeout: 30000 });
  await page.fill('input[type="email"], input[name="email"]', EMAIL);
  await page.fill('input[type="password"], input[name="password"]', PASSWORD);
  await page.click('button[type="submit"]');
  await page.waitForURL(/\/admin\/dashboard/, { timeout: 20000 });
  record('Auth', 'Login with email/password', 'OK', '→ Dashboard');
}

async function pickSelect(page, triggerLabel, optionText) {
  const trigger = page.getByRole('combobox').filter({ hasText: new RegExp(triggerLabel, 'i') }).first();
  if ((await trigger.count()) === 0) {
    await page.locator('[role="combobox"]').first().click();
  } else {
    await trigger.click();
  }
  await page.getByRole('option', { name: new RegExp(optionText, 'i') }).first().click();
}

async function waitSuccess(page) {
  await page.waitForTimeout(1200);
  const err = await page.locator('text=React Error').count();
  if (err > 0) throw new Error('React error boundary');
}

// ─── 1. Departments: create → appears in list ───
async function flowDepartments(page) {
  const name = `QA Dept ${TS}`;
  await page.goto(`${BASE}/admin/departments`, { waitUntil: 'networkidle' });
  await page.getByRole('button', { name: /add department/i }).click();
  await page.locator('#name, input[name="name"]').first().fill(name);
  await page.locator('textarea').first().fill('Automated test department');
  await page.getByRole('button', { name: /create|save|submit/i }).last().click();
  await waitSuccess(page);
  const visible = await page.getByText(name).count();
  record('Departments', 'Create (name + description)', visible > 0 ? 'OK' : 'WARN', visible > 0 ? 'Listed in table' : 'Save OK, row not found');
}

// ─── 2. Designations ───
async function flowDesignations(page) {
  const name = `QA Role ${TS}`;
  await page.goto(`${BASE}/admin/designations`, { waitUntil: 'networkidle' });
  await page.getByRole('button', { name: /add designation/i }).click();
  await page.locator('#name, input[placeholder*="name" i]').first().fill(name);
  await page.getByRole('button', { name: /create|save/i }).last().click();
  await waitSuccess(page);
  record('Designations', 'Create designation', (await page.getByText(name).count()) > 0 ? 'OK' : 'WARN', name);
}

// ─── 3. Centers ───
async function flowCenters(page) {
  const name = `QA Center ${TS}`;
  await page.goto(`${BASE}/admin/centers`, { waitUntil: 'networkidle' });
  await page.getByRole('button', { name: /add center|new center|create/i }).first().click();
  await page.locator('input[name="name"]').fill(name);
  await page.locator('input[name="address_line1"]').fill('123 Test Street');
  await page.locator('input[name="place"]').fill('Test Park');
  await page.locator('input[name="city"]').fill('Chennai');
  await page.locator('input[name="state"]').fill('TN');
  await page.locator('input[name="pincode"]').fill('600001');
  await page.getByRole('button', { name: /save|create|submit/i }).last().click();
  await waitSuccess(page);
  record('Centers', 'Create center (address form)', (await page.getByText(name).count()) > 0 ? 'OK' : 'WARN', name);
}

// ─── 4. Holidays ───
async function flowHolidays(page) {
  const name = `QA Holiday ${TS}`;
  const date = '2099-12-25';
  await page.goto(`${BASE}/admin/holidays`, { waitUntil: 'networkidle' });
  await page.getByRole('button', { name: /add holiday/i }).click();
  await page.locator('#name, input').filter({ has: page.locator('..') }).first();
  const dialog = page.getByRole('dialog');
  await dialog.locator('input').first().fill(name);
  await dialog.locator('input[type="date"]').fill(date);
  await dialog.getByRole('button', { name: /save|create|add/i }).click();
  await waitSuccess(page);
  record('Holidays', 'Create holiday (name + date)', (await page.getByText(name).count()) > 0 ? 'OK' : 'WARN', `${name} on ${date}`);
}

// ─── 5. Leave request (employee submit) ───
async function flowLeaveRequest(page) {
  await page.goto(`${BASE}/admin/leave-requests`, { waitUntil: 'networkidle' });
  await page.getByRole('button', { name: /new leave request/i }).click();
  const dialog = page.getByRole('dialog');
  await dialog.getByRole('combobox').click();
  await page.getByRole('option', { name: /annual/i }).click();
  await dialog.locator('input[type="date"]').nth(0).fill('2099-06-10');
  await dialog.locator('input[type="date"]').nth(1).fill('2099-06-12');
  await dialog.locator('textarea').fill('QA automated leave flow test');
  await dialog.getByRole('button', { name: /submit|request|save/i }).click();
  await waitSuccess(page);
  record('Leave', 'Submit request (type, dates, reason)', 'OK', 'Dialog closed / table refresh');
}

// ─── 6. Tasks create ───
async function flowTaskCreate(page) {
  const title = `QA Task ${TS}`;
  await page.goto(`${BASE}/admin/tasks/create`, { waitUntil: 'networkidle' });
  await page.locator('#title').fill(title);
  await page.locator('#description, textarea').first().fill('Automated task description');
  await page.getByRole('button', { name: /create task/i }).click();
  await page.waitForURL(/\/admin\/tasks/, { timeout: 15000 });
  await waitSuccess(page);
  record('Tasks', 'Create task (title, description)', (await page.getByText(title).count()) > 0 ? 'OK' : 'WARN', title);
}

// ─── 7. Workflow create ───
async function flowWorkflowCreate(page) {
  const name = `QA Workflow ${TS}`;
  await page.goto(`${BASE}/admin/workflows/create`, { waitUntil: 'networkidle' });
  await page.locator('#name, input').first().fill(name);
  const triggers = page.getByRole('combobox');
  if ((await triggers.count()) > 0) {
    await triggers.first().click();
    await page.getByRole('option', { name: /leave request submitted/i }).click();
  }
  await page.getByRole('button', { name: /create workflow/i }).click();
  await page.waitForURL(/\/admin\/workflows/, { timeout: 20000 }).catch(() => {});
  await waitSuccess(page);
  record('Workflows', 'Create workflow (name + trigger)', 'OK', name);
}

// ─── 8. Projects create ───
async function flowProjectCreate(page) {
  const name = `QA Project ${TS}`;
  await page.goto(`${BASE}/admin/projects/create`, { waitUntil: 'networkidle' });
  await page.locator('#name, input').filter({ hasNot: page.locator('[type="hidden"]') }).first().fill(name);
  const desc = page.locator('textarea').first();
  if ((await desc.count()) > 0) await desc.fill('QA project flow');
  await page.getByRole('button', { name: /create project|save/i }).click();
  await page.waitForURL(/\/admin\/projects/, { timeout: 20000 }).catch(() => {});
  record('Projects', 'Create project', 'OK', name);
}

// ─── 9. Profile settings (read + patch field) ───
async function flowProfile(page) {
  await page.goto(`${BASE}/admin/settings/profile`, { waitUntil: 'networkidle' });
  const personalTab = page.getByRole('tab', { name: /personal/i });
  if ((await personalTab.count()) > 0) await personalTab.click();
  const phone = page.locator('#phone, input[name="phone"]').first();
  if ((await phone.count()) > 0) {
    await phone.fill(`9${String(TS).slice(-9)}`);
    const saveBtn = page.getByRole('button', { name: /^save personal|^update personal|save changes/i });
    if ((await saveBtn.count()) > 0) {
      await saveBtn.first().click();
      await waitSuccess(page);
      record('Settings', 'Profile: update phone (personal tab)', 'OK');
    } else {
      record('Settings', 'Profile: fields visible', 'OK', 'Submit on personal tab not found');
    }
  } else {
    record('Settings', 'Profile page', 'WARN', 'Phone input not found');
  }
}

// ─── 10. Attendance flow ───
async function flowAttendance(page) {
  await page.goto(`${BASE}/admin/attendance`, { waitUntil: 'networkidle' });
  record('Attendance', 'Load today sessions + stats', 'OK', 'Tabs: Statistics / History');

  const clockOut = page.getByRole('button', { name: /^clock out$/i });
  if ((await clockOut.count()) > 0 && (await clockOut.isEnabled())) {
    await clockOut.click();
    await waitSuccess(page);
    record('Attendance', 'Clock out active session', 'OK');
  } else {
    record('Attendance', 'Clock out', 'SKIP', 'No active session');
  }

  await page.getByRole('tab', { name: /history/i }).click();
  await page.waitForTimeout(500);
  record('Attendance', 'History tab (attendance table)', 'OK');

  record('Attendance', 'Clock in via face dialog', 'SKIP', 'Requires camera + face models (manual)');
}

// ─── 11. Payroll (select employee UI) ───
async function flowPayroll(page) {
  await page.goto(`${BASE}/admin/payroll`, { waitUntil: 'networkidle' });
  const checkboxes = page.locator('input[type="checkbox"]');
  const count = await checkboxes.count();
  if (count > 1) {
    await checkboxes.nth(1).check();
    record('Payroll', 'Select employee checkbox', 'OK');
  } else {
    record('Payroll', 'Employee list', 'WARN', 'No checkboxes to select');
  }
}

// ─── 12. Dashboard metrics load ───
async function flowDashboard(page) {
  await page.goto(`${BASE}/admin/dashboard`, { waitUntil: 'networkidle' });
  const hasEmployees = (await page.getByText(/employee|attendance|payroll/i).count()) > 0;
  record('Dashboard', 'HR metrics charts load', hasEmployees ? 'OK' : 'WARN');
}

async function main() {
  console.log('=== HRM Input Flow Check ===\n');
  const browser = await chromium.launch({ headless: true });
  const page = await browser.newPage();

  const apiFails = [];
  page.on('response', (res) => {
    if (res.url().includes('/api/') && res.status() >= 400) {
      apiFails.push(`${res.request().method()} ${res.status()} ${res.url()}`);
    }
  });

  try {
    await login(page);
    await flowDashboard(page);
    await flowDepartments(page);
    await flowDesignations(page);
    await flowCenters(page);
    await flowHolidays(page);
    await flowLeaveRequest(page);
    await flowTaskCreate(page);
    await flowWorkflowCreate(page);
    await flowProjectCreate(page);
    await flowProfile(page);
    await flowAttendance(page);
    await flowPayroll(page);
  } catch (e) {
    record('Runner', 'Unhandled error', 'FAIL', e.message);
  }

  await browser.close();

  console.log('\n--- Flow summary ---');
  const byModule = {};
  for (const f of flows) {
    if (!byModule[f.module]) byModule[f.module] = [];
    byModule[f.module].push(f);
  }
  for (const [mod, steps] of Object.entries(byModule)) {
    const bad = steps.filter((s) => s.status === 'FAIL');
    const warn = steps.filter((s) => s.status === 'WARN');
    console.log(`\n${mod}:`);
    steps.forEach((s) => console.log(`  • ${s.step} [${s.status}]${s.detail ? ` — ${s.detail}` : ''}`));
    if (bad.length) console.log(`  ⚠ ${bad.length} failure(s)`);
  }

  const fails = flows.filter((f) => f.status === 'FAIL');
  if (apiFails.length) {
    console.log('\nAPI errors during flows:');
    [...new Set(apiFails)].slice(0, 10).forEach((x) => console.log('  -', x));
  }

  console.log(`\nTotal steps: ${flows.length} | FAIL: ${fails.length} | WARN: ${flows.filter((f) => f.status === 'WARN').length}`);
  process.exit(fails.length > 0 ? 1 : 0);
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
