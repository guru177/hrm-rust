use crate::db::DbPool;

fn expand_salary_component_calculation_types(conn: &rusqlite::Connection) {
    let ddl: String = conn
        .query_row(
            "SELECT sql FROM sqlite_master WHERE name='salary_components'",
            [],
            |r| r.get(0),
        )
        .unwrap_or_default();
    if ddl.contains("percentage_of_gross") {
        return;
    }
    let _ = conn.execute_batch(
        "
        PRAGMA foreign_keys=OFF;
        CREATE TABLE salary_components_new (
            id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
            name VARCHAR NOT NULL,
            type VARCHAR CHECK (type IN ('earning', 'deduction', 'reimbursement')) NOT NULL,
            description VARCHAR,
            is_active TINYINT(1) NOT NULL DEFAULT 1,
            created_at DATETIME,
            updated_at DATETIME,
            earning_type VARCHAR,
            name_in_payslip VARCHAR,
            calculation_type VARCHAR CHECK (calculation_type IN ('flat_amount', 'percentage_of_basic', 'percentage_of_ctc', 'percentage_of_gross')),
            amount NUMERIC,
            deduction_type VARCHAR,
            deduction_frequency VARCHAR CHECK (deduction_frequency IN ('recurring', 'one_time')),
            is_pre_tax TINYINT(1) NOT NULL DEFAULT 0,
            reimbursement_type VARCHAR,
            max_amount_per_month NUMERIC,
            slug TEXT,
            component_type TEXT,
            default_value REAL,
            is_taxable INTEGER
        );
        INSERT INTO salary_components_new SELECT * FROM salary_components;
        DROP TABLE salary_components;
        ALTER TABLE salary_components_new RENAME TO salary_components;
        PRAGMA foreign_keys=ON;
        ",
    );
    log::info!("Expanded salary_components.calculation_type (ctc + gross percentages)");
}

/// Run all migrations. The existing SQLite database from Laravel is used directly,
/// so we only need to ensure our extra tables (e.g. jwt_refresh_tokens) exist.
pub fn run_migrations(pool: &DbPool) {
    let conn = pool.get().expect("Failed to get connection for migrations");

    // The existing Laravel database already has all tables.
    // We only add tables that Rust-specific features need.
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS jwt_refresh_tokens (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            token TEXT NOT NULL UNIQUE,
            expires_at TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            revoked INTEGER NOT NULL DEFAULT 0
        );

        CREATE INDEX IF NOT EXISTS idx_jwt_refresh_tokens_user_id ON jwt_refresh_tokens(user_id);
        CREATE INDEX IF NOT EXISTS idx_jwt_refresh_tokens_token ON jwt_refresh_tokens(token);

        -- Centers (settings)
        CREATE TABLE IF NOT EXISTS centers (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            code TEXT,
            address TEXT,
            city TEXT,
            state TEXT,
            country TEXT,
            is_active INTEGER NOT NULL DEFAULT 1,
            created_at TEXT DEFAULT (datetime('now')),
            updated_at TEXT DEFAULT (datetime('now'))
        );

        -- App Settings (key-value config)
        CREATE TABLE IF NOT EXISTS app_settings (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            key TEXT NOT NULL UNIQUE,
            value TEXT,
            type TEXT NOT NULL DEFAULT 'text',
            description TEXT,
            created_at TEXT DEFAULT (datetime('now')),
            updated_at TEXT DEFAULT (datetime('now'))
        );

        -- Shift templates (Phase 1)
        CREATE TABLE IF NOT EXISTS shift_templates (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            start_time TEXT NOT NULL DEFAULT '09:00:00',
            end_time TEXT NOT NULL DEFAULT '18:00:00',
            grace_in_minutes INTEGER NOT NULL DEFAULT 0,
            grace_out_minutes INTEGER NOT NULL DEFAULT 0,
            is_active INTEGER NOT NULL DEFAULT 1,
            created_at TEXT DEFAULT (datetime('now')),
            updated_at TEXT DEFAULT (datetime('now'))
        );

        -- User-level shift assignment (Phase 1)
        CREATE TABLE IF NOT EXISTS user_shift_assignments (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            shift_template_id INTEGER NOT NULL REFERENCES shift_templates(id) ON DELETE CASCADE,
            effective_from TEXT NOT NULL,
            effective_to TEXT,
            created_at TEXT DEFAULT (datetime('now')),
            updated_at TEXT DEFAULT (datetime('now'))
        );

        CREATE INDEX IF NOT EXISTS idx_user_shift_user_date
            ON user_shift_assignments(user_id, effective_from, effective_to);
        ",
    )
    .expect("Failed to run migrations");

    // Add new ATS columns to job_applications (ignore errors if they already exist)
    let extra_columns = [
        "ALTER TABLE job_applications ADD COLUMN applied_position TEXT;",
        "ALTER TABLE job_applications ADD COLUMN experience_years INTEGER;",
        "ALTER TABLE job_applications ADD COLUMN expected_salary TEXT;",
        "ALTER TABLE job_applications ADD COLUMN dob TEXT;",
        "ALTER TABLE job_applications ADD COLUMN ats_score INTEGER;",
        "ALTER TABLE job_applications ADD COLUMN ats_feedback TEXT;",
    ];

    for col_sql in extra_columns.iter() {
        let _ = conn.execute(col_sql, []);
    }

    // ── Biometric Integration Tables ──
    conn.execute_batch(
        "
        -- Registered biometric devices
        CREATE TABLE IF NOT EXISTS biometric_devices (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            serial_number TEXT NOT NULL UNIQUE,
            name TEXT DEFAULT 'BIO-PARK D01',
            model TEXT DEFAULT 'D01',
            ip_address TEXT,
            location TEXT,
            is_active INTEGER NOT NULL DEFAULT 1,
            last_heartbeat TEXT,
            firmware_version TEXT,
            created_at TEXT DEFAULT (datetime('now')),
            updated_at TEXT DEFAULT (datetime('now'))
        );

        -- Raw punch logs from biometric device (immutable audit trail)
        CREATE TABLE IF NOT EXISTS biometric_punches (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            device_serial TEXT NOT NULL,
            device_pin TEXT NOT NULL,
            punch_time TEXT NOT NULL,
            punch_type INTEGER NOT NULL DEFAULT 0,
            verify_method INTEGER DEFAULT 0,
            user_id INTEGER REFERENCES users(id),
            is_processed INTEGER NOT NULL DEFAULT 0,
            created_at TEXT DEFAULT (datetime('now'))
        );

        CREATE INDEX IF NOT EXISTS idx_bio_punches_device ON biometric_punches(device_serial);
        CREATE INDEX IF NOT EXISTS idx_bio_punches_time ON biometric_punches(punch_time);
        CREATE INDEX IF NOT EXISTS idx_bio_punches_user ON biometric_punches(user_id);

        -- Mapping: device PIN → HRM user
        CREATE TABLE IF NOT EXISTS biometric_user_map (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            device_serial TEXT NOT NULL,
            device_pin TEXT NOT NULL,
            user_id INTEGER NOT NULL REFERENCES users(id),
            created_at TEXT DEFAULT (datetime('now')),
            UNIQUE(device_serial, device_pin)
        );

        -- Command queue for sending instructions to devices
        CREATE TABLE IF NOT EXISTS biometric_commands (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            device_serial TEXT NOT NULL,
            command TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'pending',
            result TEXT,
            created_at TEXT DEFAULT (datetime('now')),
            executed_at TEXT
        );
        ",
    )
    .expect("Failed to run biometric migrations");

    // Add source column to attendance table (ignore if already exists)
    let _ = conn.execute("ALTER TABLE attendance ADD COLUMN source TEXT DEFAULT 'manual'", []);

    // Empty employee_id strings violate UNIQUE when multiple users have no ID
    let _ = conn.execute("UPDATE users SET employee_id = NULL WHERE employee_id = ''", []);

    // Default shift flag (auto-assign unassigned employees to this template)
    let _ = conn.execute(
        "ALTER TABLE shift_templates ADD COLUMN is_default INTEGER NOT NULL DEFAULT 0",
        [],
    );
    let default_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM shift_templates WHERE is_default = 1",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);
    if default_count == 0 {
        let _ = conn.execute(
            "UPDATE shift_templates SET is_default = 1
             WHERE id = (
                 SELECT id FROM shift_templates
                 WHERE LOWER(name) = 'general'
                 ORDER BY id ASC
                 LIMIT 1
             )",
            [],
        );
    }

    // Merge duplicate general/General templates, then assign unassigned employees
    crate::shift_logic::consolidate_duplicate_general_shifts(&conn);
    crate::shift_logic::backfill_general_shift_assignments(&conn);

    let _ = conn.execute(
        "ALTER TABLE shift_templates ADD COLUMN working_days_mask INTEGER NOT NULL DEFAULT 31",
        [],
    );

    let _ = conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS shift_daily_roster (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            roster_date TEXT NOT NULL,
            shift_template_id INTEGER REFERENCES shift_templates(id) ON DELETE SET NULL,
            is_day_off INTEGER NOT NULL DEFAULT 0,
            created_at TEXT DEFAULT (datetime('now')),
            updated_at TEXT DEFAULT (datetime('now')),
            UNIQUE(user_id, roster_date)
        );
        CREATE INDEX IF NOT EXISTS idx_shift_daily_roster_date
            ON shift_daily_roster(roster_date);
        CREATE INDEX IF NOT EXISTS idx_shift_daily_roster_user_date
            ON shift_daily_roster(user_id, roster_date);
        ",
    );

    let payslip_cols = [
        "ALTER TABLE payslips ADD COLUMN adjustments TEXT;",
        "ALTER TABLE payslips ADD COLUMN generated_at TEXT;",
        "ALTER TABLE payslips ADD COLUMN shift_penalty REAL DEFAULT 0;",
    ];
    for sql in payslip_cols {
        let _ = conn.execute(sql, []);
    }

    let _ = conn.execute(
        "INSERT OR IGNORE INTO app_settings (key, value, type, description, created_at, updated_at)
         VALUES ('annual_leave_quota', '12', 'number', 'Annual leave days per employee', datetime('now'), datetime('now'))",
        [],
    );

    let _ = conn.execute(
        "INSERT OR IGNORE INTO app_settings (key, value, type, description, created_at, updated_at)
         VALUES ('msg91_whatsapp_sender', '', 'text', 'MSG91 WhatsApp integrated number/sender ID', datetime('now'), datetime('now'))",
        [],
    );

    let _ = conn.execute(
        "INSERT OR IGNORE INTO permissions (name, slug, description, \"group\", created_at, updated_at)
         VALUES ('Manage Payroll', 'manage-payroll', 'Generate, preview, and unlock payslips', 'Payroll', datetime('now'), datetime('now'))",
        [],
    );

    // Laravel salary_components uses `type`; Rust code expects `component_type`, `slug`, etc.
    let salary_component_cols = [
        "ALTER TABLE salary_components ADD COLUMN slug TEXT;",
        "ALTER TABLE salary_components ADD COLUMN component_type TEXT;",
        "ALTER TABLE salary_components ADD COLUMN default_value REAL;",
        "ALTER TABLE salary_components ADD COLUMN is_taxable INTEGER;",
    ];
    for sql in salary_component_cols {
        let _ = conn.execute(sql, []);
    }
    let _ = conn.execute(
        "UPDATE salary_components SET component_type = type WHERE component_type IS NULL AND type IS NOT NULL",
        [],
    );
    let _ = conn.execute(
        "UPDATE salary_components SET slug = LOWER(REPLACE(name, ' ', '_')) WHERE slug IS NULL OR slug = ''",
        [],
    );
    let _ = conn.execute(
        "UPDATE salary_components SET default_value = amount WHERE default_value IS NULL AND amount IS NOT NULL",
        [],
    );
    let _ = conn.execute(
        "UPDATE salary_components SET is_taxable = COALESCE(is_pre_tax, 0) WHERE is_taxable IS NULL",
        [],
    );
    // Reimbursements used calculation_type='reimbursement' as a marker; store real calc separately.
    let _ = conn.execute(
        "UPDATE salary_components SET
            type = 'reimbursement',
            component_type = 'reimbursement',
            calculation_type = 'flat_amount'
         WHERE calculation_type = 'reimbursement'",
        [],
    );
    let _ = conn.execute(
        "UPDATE salary_components SET
            type = 'reimbursement',
            component_type = 'reimbursement'
         WHERE reimbursement_type IS NOT NULL
           AND COALESCE(component_type, type) NOT IN ('reimbursement')",
        [],
    );
    let _ = conn.execute(
        "UPDATE salary_components SET earning_type = name
         WHERE COALESCE(component_type, type) = 'earning'
           AND (earning_type IS NULL OR earning_type IN ('flat_amount', 'percentage_of_basic'))",
        [],
    );

    // ── Phase 1–4: CTC templates, profiles, advances, extended payslip ──
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS salary_templates (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            basic_pct REAL NOT NULL DEFAULT 50,
            hra_pct REAL NOT NULL DEFAULT 35,
            conv_pct REAL NOT NULL DEFAULT 15,
            special_pct REAL NOT NULL DEFAULT 0,
            is_default INTEGER NOT NULL DEFAULT 0,
            created_at TEXT DEFAULT (datetime('now')),
            updated_at TEXT DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS employee_salary_profiles (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            yearly_ctc REAL NOT NULL DEFAULT 0,
            template_id INTEGER REFERENCES salary_templates(id),
            pf_applicable INTEGER NOT NULL DEFAULT 1,
            esi_applicable INTEGER NOT NULL DEFAULT 1,
            effective_from TEXT NOT NULL,
            created_at TEXT DEFAULT (datetime('now')),
            updated_at TEXT DEFAULT (datetime('now'))
        );

        CREATE INDEX IF NOT EXISTS idx_emp_salary_profile_user
            ON employee_salary_profiles(user_id, effective_from);

        CREATE TABLE IF NOT EXISTS employee_advances (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            amount REAL NOT NULL,
            balance REAL NOT NULL,
            monthly_emi REAL NOT NULL DEFAULT 0,
            description TEXT,
            is_active INTEGER NOT NULL DEFAULT 1,
            created_at TEXT DEFAULT (datetime('now')),
            updated_at TEXT DEFAULT (datetime('now'))
        );

        CREATE INDEX IF NOT EXISTS idx_employee_advances_user ON employee_advances(user_id);
        ",
    )
    .expect("salary template migrations");

    let _ = conn.execute(
        "INSERT OR IGNORE INTO salary_templates (name, basic_pct, hra_pct, conv_pct, special_pct, is_default)
         VALUES ('Standard (50/35/15)', 50, 35, 15, 0, 1)",
        [],
    );

    expand_salary_component_calculation_types(&conn);

    let _ = conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS leave_types (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            slug TEXT NOT NULL UNIQUE,
            name TEXT NOT NULL,
            payment_type TEXT NOT NULL DEFAULT 'paid' CHECK (payment_type IN ('paid', 'unpaid', 'half_day')),
            counts_toward_quota INTEGER NOT NULL DEFAULT 0,
            is_active INTEGER NOT NULL DEFAULT 1,
            created_at TEXT,
            updated_at TEXT
        );",
    );

    let default_leave_types = [
        ("sick", "Sick Leave", "paid", 0),
        ("annual", "Annual Leave", "paid", 1),
        ("personal", "Personal Leave", "paid", 0),
        ("unpaid", "Unpaid Leave", "unpaid", 0),
        ("emergency", "Emergency Leave", "paid", 0),
    ];
    for (slug, name, payment, quota) in default_leave_types {
        let _ = conn.execute(
            "INSERT OR IGNORE INTO leave_types (slug, name, payment_type, counts_toward_quota, is_active, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, 1, datetime('now'), datetime('now'))",
            rusqlite::params![slug, name, payment, quota],
        );
    }
    let _ = conn.execute(
        "UPDATE leave_types SET payment_type='unpaid' WHERE slug='unpaid'",
        [],
    );

    // Statutory settings
    let statutory_settings = [
        ("pf_wage_ceiling", "15000", "PF wage ceiling"),
        ("pf_employee_rate", "0.12", "PF employee rate"),
        ("pf_employer_rate", "0.12", "PF employer rate"),
        ("esi_gross_ceiling", "21000", "ESI gross ceiling"),
        ("esi_employee_rate", "0.0075", "ESI employee rate"),
        ("esi_employer_rate", "0.0325", "ESI employer rate"),
        ("esi_admin_rate", "0", "ESI admin rate"),
        ("prof_tax_default", "200", "Default professional tax"),
        ("lw_employee", "50", "Labour welfare employee"),
        ("lw_employer", "50", "Labour welfare employer"),
    ];
    for (key, value, desc) in statutory_settings {
        let _ = conn.execute(
            "INSERT OR IGNORE INTO app_settings (key, value, type, description, created_at, updated_at)
             VALUES (?1, ?2, 'number', ?3, datetime('now'), datetime('now'))",
            rusqlite::params![key, value, desc],
        );
    }

    let payslip_ext_cols = [
        "ALTER TABLE payslips ADD COLUMN lop_basic REAL DEFAULT 0;",
        "ALTER TABLE payslips ADD COLUMN lop_hra REAL DEFAULT 0;",
        "ALTER TABLE payslips ADD COLUMN lop_transport REAL DEFAULT 0;",
        "ALTER TABLE payslips ADD COLUMN lop_other REAL DEFAULT 0;",
        "ALTER TABLE payslips ADD COLUMN prof_tax REAL DEFAULT 0;",
        "ALTER TABLE payslips ADD COLUMN advance_deduction REAL DEFAULT 0;",
        "ALTER TABLE payslips ADD COLUMN lw_employee REAL DEFAULT 0;",
        "ALTER TABLE payslips ADD COLUMN payroll_detail TEXT;",
    ];
    for sql in payslip_ext_cols {
        let _ = conn.execute(sql, []);
    }

    let _ = conn.execute(
        "INSERT OR IGNORE INTO permission_role (permission_id, role_id, created_at, updated_at)
         SELECT p_manage.id, pr.role_id, datetime('now'), datetime('now')
         FROM permissions p_manage
         JOIN permission_role pr ON 1=1
         JOIN permissions p_view ON p_view.id = pr.permission_id AND p_view.slug = 'view-payroll'
         WHERE p_manage.slug = 'manage-payroll'
           AND NOT EXISTS (
               SELECT 1 FROM permission_role x
               WHERE x.permission_id = p_manage.id AND x.role_id = pr.role_id
           )",
        [],
    );

    log::info!("✅ Database migrations completed");
}
