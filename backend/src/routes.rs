use actix_web::web;
use crate::handlers;

/// M-CARD / BIO-PARK ADMS push protocol — device calls /pub/chat (no JWT). Bound on BIOMETRIC_PORT.
pub fn configure_adms(cfg: &mut web::ServiceConfig) {
    cfg
        .route("/pub/chat", web::get().to(handlers::biometric::adms_chat_ws))
        .route("/pub/chat", web::post().to(handlers::biometric::adms_chat_post))
        .route("/pub/getrequest", web::get().to(handlers::biometric::adms_getrequest));
}

/// iClock / ADMS — device pushes attendance here (no JWT). Bound on BIOMETRIC_PORT (7788).
pub fn configure_iclock(cfg: &mut web::ServiceConfig) {
    cfg
        .route("/iclock/cdata", web::get().to(handlers::biometric::iclock_handshake))
        .route("/iclock/cdata", web::post().to(handlers::biometric::iclock_receive))
        .route("/iclock/getrequest", web::get().to(handlers::biometric::iclock_getrequest))
        .route("/iclock/devicecmd", web::post().to(handlers::biometric::iclock_devicecmd));
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg
        // Health check
        .route("/api/health", web::get().to(|| async {
            actix_web::HttpResponse::Ok().json(serde_json::json!({"status": "ok", "service": "hrm-backend"}))
        }))

        // ── Auth ──
        .route("/api/auth/login", web::post().to(handlers::auth::login))
        .route("/api/auth/refresh", web::post().to(handlers::auth::refresh))
        .route("/api/auth/me", web::get().to(handlers::auth::me))
        .route("/api/auth/logout", web::post().to(handlers::auth::logout))
        .route("/api/onboarding/complete", web::post().to(handlers::settings::complete_onboarding))

        // ── Public Careers ──
        .route("/api/public/careers", web::get().to(handlers::careers::public_list))
        .route("/api/public/careers/apply", web::post().to(handlers::careers::public_apply))

        // ── Dashboard Analytics ──
        .route("/api/admin/dashboard/hr-data", web::get().to(handlers::analytics::hr_dashboard))

        // ── Users ──
        .route("/api/admin/users", web::get().to(handlers::users::index))
        .route("/api/admin/users/stats", web::get().to(handlers::users::stats))
        .route("/api/admin/users/list", web::get().to(handlers::users::list))
        .route("/api/admin/users", web::post().to(handlers::users::store))
        .route("/api/admin/users/{id}/salary-structure", web::get().to(handlers::salaries::user_salary_structure_show))
        .route("/api/admin/users/{id}/salary-structure", web::post().to(handlers::salaries::user_salary_structure_store))
        .route("/api/admin/users/{id}/ctc-profile", web::get().to(handlers::salaries::user_ctc_profile_show))
        .route("/api/admin/users/{id}/ctc-profile", web::post().to(handlers::salaries::user_ctc_profile_store))
        .route("/api/admin/users/{id}/ctc-profile", web::delete().to(handlers::salaries::user_ctc_profile_destroy))
        .route("/api/admin/users/{id}/advances", web::get().to(handlers::salaries::advances_list))
        .route("/api/admin/users/{id}/advances", web::post().to(handlers::salaries::advances_store))
        .route("/api/admin/users/{id}", web::get().to(handlers::users::show))
        .route("/api/admin/users/{id}", web::put().to(handlers::users::update))
        .route("/api/admin/users/{id}", web::post().to(handlers::users::update_form))
        .route("/api/admin/users/{id}", web::delete().to(handlers::users::destroy))

        // ── Departments ──
        .route("/api/admin/departments", web::get().to(handlers::departments::index))
        .route("/api/admin/departments/stats", web::get().to(handlers::departments::stats))
        .route("/api/admin/departments/list", web::get().to(handlers::departments::list))
        .route("/api/admin/departments", web::post().to(handlers::departments::store))
        .route("/api/admin/departments/{id}", web::get().to(handlers::departments::show))
        .route("/api/admin/departments/{id}", web::put().to(handlers::departments::update))
        .route("/api/admin/departments/{id}", web::delete().to(handlers::departments::destroy))

        // ── Designations ──
        .route("/api/admin/designations", web::get().to(handlers::designations::index))
        .route("/api/admin/designations/stats", web::get().to(handlers::designations::stats))
        .route("/api/admin/designations/list", web::get().to(handlers::designations::list))
        .route("/api/admin/designations", web::post().to(handlers::designations::store))
        .route("/api/admin/designations/{id}", web::get().to(handlers::designations::show))
        .route("/api/admin/designations/{id}", web::put().to(handlers::designations::update))
        .route("/api/admin/designations/{id}", web::delete().to(handlers::designations::destroy))

        // ── Roles ──
        .route("/api/admin/roles", web::get().to(handlers::roles::index))
        .route("/api/admin/roles/stats", web::get().to(handlers::roles::stats))
        .route("/api/admin/roles/list", web::get().to(handlers::roles::list))
        .route("/api/admin/roles", web::post().to(handlers::roles::store))
        .route("/api/admin/roles/{id}", web::get().to(handlers::roles::show))
        .route("/api/admin/roles/{id}", web::put().to(handlers::roles::update))
        .route("/api/admin/roles/{id}", web::delete().to(handlers::roles::destroy))

        // ── Permissions ──
        .route("/api/admin/permissions", web::get().to(handlers::permissions::index))
        .route("/api/admin/permissions/list", web::get().to(handlers::permissions::list))
        .route("/api/admin/permissions", web::post().to(handlers::permissions::store))
        .route("/api/admin/permissions/{id}", web::get().to(handlers::permissions::show))
        .route("/api/admin/permissions/{id}", web::put().to(handlers::permissions::update))
        .route("/api/admin/permissions/{id}", web::delete().to(handlers::permissions::destroy))

        // ── Attendance ──
        .route("/api/admin/attendance", web::get().to(handlers::attendance::index))
        .route("/api/admin/attendance/list", web::get().to(handlers::attendance::list))
        .route("/api/admin/attendance/users", web::get().to(handlers::attendance::users))
        .route("/api/admin/attendance/today", web::get().to(handlers::attendance::today))
        .route("/api/admin/attendance/stats", web::get().to(handlers::attendance::stats))
        .route("/api/admin/attendance/clock-in", web::post().to(handlers::attendance::clock_in))
        .route("/api/admin/attendance/clock-out", web::post().to(handlers::attendance::clock_out))

        // ── Shifts (Phase 1) ──
        .route("/api/admin/shifts", web::get().to(handlers::shifts::index))
        .route("/api/admin/shifts", web::post().to(handlers::shifts::store))
        .route("/api/admin/shifts/{id}", web::put().to(handlers::shifts::update))
        .route("/api/admin/shifts/{id}", web::delete().to(handlers::shifts::destroy))
        .route("/api/admin/shifts/assign-user", web::post().to(handlers::shifts::assign_user))
        .route("/api/admin/shifts/roster", web::get().to(handlers::shifts::roster))
        .route("/api/admin/shifts/daily-roster", web::get().to(handlers::shifts::daily_roster_show))
        .route("/api/admin/shifts/daily-roster", web::post().to(handlers::shifts::daily_roster_store))
        .route("/api/admin/shifts/user/{id}", web::get().to(handlers::shifts::user_assignment))

        // ── Leave Requests ──
        .route("/api/admin/leave-types", web::get().to(handlers::leave_types::index))
        .route("/api/admin/settings/leave-types", web::get().to(handlers::leave_types::settings_list))
        .route("/api/admin/settings/leave-types", web::post().to(handlers::leave_types::store))
        .route("/api/admin/settings/leave-types/{id}", web::put().to(handlers::leave_types::update))
        .route("/api/admin/leave-requests", web::get().to(handlers::leave_requests::index))
        .route("/api/admin/leave-requests/list", web::get().to(handlers::leave_requests::list))
        .route("/api/admin/leave-requests/stats", web::get().to(handlers::leave_requests::stats))
        .route("/api/admin/leave-requests", web::post().to(handlers::leave_requests::store))
        .route("/api/admin/leave-requests/{id}", web::delete().to(handlers::leave_requests::destroy))
        .route("/api/admin/leave-requests/manage", web::get().to(handlers::leave_requests::manage))
        .route("/api/admin/leave-requests/manage/list", web::get().to(handlers::leave_requests::list_all))
        .route("/api/admin/leave-requests/manage/stats", web::get().to(handlers::leave_requests::admin_stats))
        .route("/api/admin/leave-requests/{id}/approve", web::post().to(handlers::leave_requests::approve))
        .route("/api/admin/leave-requests/{id}/reject", web::post().to(handlers::leave_requests::reject))
        .route("/api/admin/leave-requests/{id}/remarks", web::put().to(handlers::leave_requests::update_remarks))

        // ── Holidays ──
        .route("/api/admin/holidays", web::get().to(handlers::holidays::index))
        .route("/api/admin/holidays/list", web::get().to(handlers::holidays::list))
        .route("/api/admin/holidays", web::post().to(handlers::holidays::store))
        .route("/api/admin/holidays/{id}", web::put().to(handlers::holidays::update))
        .route("/api/admin/holidays/{id}", web::delete().to(handlers::holidays::destroy))

        // ── Projects ──
        .route("/api/admin/projects", web::get().to(handlers::projects::index))
        .route("/api/admin/projects/list", web::get().to(handlers::projects::index))
        .route("/api/admin/projects", web::post().to(handlers::projects::store))
        .route("/api/admin/projects/{id}", web::get().to(handlers::projects::show))
        .route("/api/admin/projects/{id}", web::put().to(handlers::projects::update))
        .route("/api/admin/projects/{id}", web::delete().to(handlers::projects::destroy))

        // ── Tasks ──
        .route("/api/admin/tasks", web::get().to(handlers::tasks::index))
        .route("/api/admin/tasks/list", web::get().to(handlers::tasks::index))
        .route("/api/admin/tasks", web::post().to(handlers::tasks::store))
        .route("/api/admin/tasks/{id}", web::get().to(handlers::tasks::show))
        .route("/api/admin/tasks/{id}", web::put().to(handlers::tasks::update))
        .route("/api/admin/tasks/{id}", web::delete().to(handlers::tasks::destroy))
        .route("/api/admin/tasks/{id}/status", web::post().to(handlers::tasks::update_status))

        // ── Workflows ──
        .route("/api/admin/workflows", web::get().to(handlers::workflows::index))
        .route("/api/admin/workflows/list", web::get().to(handlers::workflows::index))
        .route("/api/admin/workflows", web::post().to(handlers::workflows::store))
        .route("/api/admin/workflows/{id}", web::get().to(handlers::workflows::show))
        .route("/api/admin/workflows/{id}", web::put().to(handlers::workflows::update))
        .route("/api/admin/workflows/{id}", web::delete().to(handlers::workflows::destroy))
        .route("/api/admin/workflows/{id}/toggle", web::post().to(handlers::workflows::toggle))
        .route("/api/admin/workflows/{id}/duplicate", web::post().to(handlers::workflows::duplicate))

        // ── Careers ──
        .route("/api/admin/careers", web::get().to(handlers::careers::index))
        .route("/api/admin/careers/stats", web::get().to(handlers::careers::stats))
        .route("/api/admin/careers/list", web::get().to(handlers::careers::list))
        .route("/api/admin/careers", web::post().to(handlers::careers::store))
        .route("/api/admin/careers/{id}", web::get().to(handlers::careers::show))
        .route("/api/admin/careers/{id}", web::put().to(handlers::careers::update))
        .route("/api/admin/careers/{id}", web::delete().to(handlers::careers::destroy))

        // ── Job Applications ──
        .route("/api/admin/job-applications", web::get().to(handlers::job_applications::index))
        .route("/api/admin/job-applications/stats", web::get().to(handlers::job_applications::stats))
        .route("/api/admin/job-applications/list", web::get().to(handlers::job_applications::list))
        .route("/api/admin/job-applications", web::post().to(handlers::job_applications::store))
        .route("/api/admin/job-applications/{id}", web::get().to(handlers::job_applications::show))
        .route("/api/admin/job-applications/{id}", web::delete().to(handlers::job_applications::destroy))
        .route("/api/admin/job-applications/{id}/update-status", web::post().to(handlers::job_applications::update_status))
        .route("/api/admin/job-applications/{id}/send-email", web::post().to(handlers::job_applications::send_email))
        
        // ── Webhooks ──
        .route("/api/webhooks/incoming-resume", web::post().to(handlers::job_applications::webhook_incoming_resume))

        // ── Reports ──
        .route("/api/admin/reports/attendance-summary", web::get().to(handlers::reports::attendance_summary))
        .route("/api/admin/reports/payroll-register", web::get().to(handlers::reports::payroll_register))
        .route("/api/admin/reports/payroll-split", web::get().to(handlers::reports::payroll_split))
        .route("/api/admin/reports/leave-balance", web::get().to(handlers::reports::leave_balance))

        // ── Payroll ──
        .route("/api/admin/payroll", web::get().to(handlers::payroll::index))
        .route("/api/admin/payroll/list", web::get().to(handlers::payroll::list))
        .route("/api/admin/payroll/stats", web::get().to(handlers::payroll::stats))
        .route("/api/admin/payroll/employees", web::get().to(handlers::payroll::employees))
        .route("/api/admin/payroll/preview", web::post().to(handlers::payroll::preview))
        .route("/api/admin/payroll/generate", web::post().to(handlers::payroll::generate))
        .route("/api/admin/payslips/{id}/unlock", web::post().to(handlers::payroll::unlock_payslip))


        // ── Salaries ──
        .route("/api/admin/salaries/components/list", web::get().to(handlers::salaries::components_list))
        .route("/api/admin/salaries/components", web::post().to(handlers::salaries::components_store))
        .route("/api/admin/salaries/components/{id}", web::put().to(handlers::salaries::components_update))
        .route("/api/admin/salaries/components/{id}", web::delete().to(handlers::salaries::components_destroy))
        .route("/api/admin/salaries/templates", web::get().to(handlers::salaries::templates_list))
        .route("/api/admin/salaries/ctc-preview", web::post().to(handlers::salaries::ctc_preview))
        .route("/api/admin/salaries/employees/list", web::get().to(handlers::salaries::employees_list))
        .route("/api/admin/salaries/employees/filter-options", web::get().to(handlers::salaries::employees_filter_options))
        .route("/api/admin/me/payslips", web::get().to(handlers::payslips::my_payslips_list))
        .route("/api/admin/salaries/employees/{id}/payslips/list", web::get().to(handlers::payslips::employee_payslips_list))
        .route("/api/admin/payslips/{id}/send-whatsapp", web::post().to(handlers::payslips::send_whatsapp))
        .route("/api/admin/payslips/{id}/pdf", web::get().to(handlers::payslips::payslip_pdf))

        // ── Centers (Settings) ──
        .route("/api/admin/settings/centers", web::get().to(handlers::centers::index))
        .route("/api/admin/api/settings/centers", web::get().to(handlers::centers::index))
        .route("/api/admin/api/settings/centers", web::post().to(handlers::centers::store))
        .route("/api/admin/api/settings/centers/{id}", web::put().to(handlers::centers::update))
        .route("/api/admin/api/settings/centers/{id}", web::delete().to(handlers::centers::destroy))
        .route("/api/admin/settings/centers", web::post().to(handlers::centers::store))
        .route("/api/admin/settings/centers/{id}", web::put().to(handlers::centers::update))
        .route("/api/admin/settings/centers/{id}", web::delete().to(handlers::centers::destroy))

        // ── Settings ──
        .route("/api/admin/settings/app", web::get().to(handlers::settings::index))
        .route("/api/admin/settings/app", web::post().to(handlers::settings::update))
        .route("/api/admin/settings/app/logo", web::post().to(handlers::settings::upload_logo))
        .route("/api/admin/settings/password", web::put().to(handlers::settings::update_password))
        .route("/api/admin/settings/profile", web::patch().to(handlers::settings::update_profile))
        .route("/api/admin/settings/profile", web::post().to(handlers::settings::update_profile))

        // ── iClock (also on BIOMETRIC_PORT listener) ──
        .configure(configure_iclock)

        // ── Biometric Admin API (Authenticated) ──
        .route("/api/admin/biometric/devices", web::get().to(handlers::biometric::devices_list))
        .route("/api/admin/biometric/devices", web::post().to(handlers::biometric::devices_store))
        .route("/api/admin/biometric/devices/{id}", web::delete().to(handlers::biometric::devices_destroy))
        .route("/api/admin/biometric/punches", web::get().to(handlers::biometric::punches_list))
        .route("/api/admin/biometric/mapping", web::get().to(handlers::biometric::mapping_list))
        .route("/api/admin/biometric/mapping", web::post().to(handlers::biometric::mapping_store))
        .route("/api/admin/biometric/mapping/{id}", web::delete().to(handlers::biometric::mapping_destroy))
        .route("/api/admin/biometric/stats", web::get().to(handlers::biometric::biometric_stats))
        .route("/api/admin/biometric/ws", web::get().to(handlers::biometric::biometric_live_ws));
}
