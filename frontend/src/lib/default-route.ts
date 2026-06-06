/** First admin page the user may access after login. */
export function defaultAdminRoute(hasPermission: (slug: string) => boolean): string {
    const routes: [string, string][] = [
        ['view-dashboard', '/admin/dashboard'],
        ['view-attendance', '/admin/attendance'],
        ['view-users', '/admin/users'],
        ['manage-leave-requests', '/admin/leave-requests/manage'],
        ['view-payroll', '/admin/payroll'],
    ];
    for (const [perm, path] of routes) {
        if (hasPermission(perm)) return path;
    }
    return '/unauthorized';
}
