import { ReactNode } from 'react';

import { usePermissions } from '@/hooks/use-permissions';

interface PermissionGuardProps {
    permission?: string;
    action?: string;
    resource?: string;
    children: ReactNode;
    fallback?: ReactNode;
    require?: boolean; // If true, throw error instead of hiding
}

/**
 * Component to conditionally render content based on permissions
 * 
 * Usage:
 *   <PermissionGuard permission="create-users">
 *     <button>Create User</button>
 *   </PermissionGuard>
 * 
 *   <PermissionGuard action="edit" resource="users">
 *     <button>Edit User</button>
 *   </PermissionGuard>
 * 
 *   <PermissionGuard permission="view-dashboard" fallback={<div>Access Denied</div>}>
 *     <Dashboard />
 *   </PermissionGuard>
 */
export function PermissionGuard({
    permission,
    action,
    resource,
    children,
    fallback,
    require: requirePermission = false,
}: PermissionGuardProps) {
    const { hasPermission, can, require: requirePerm } = usePermissions();

    // Determine if user has permission
    let hasAccess = true;

    if (permission) {
        hasAccess = hasPermission(permission);
    } else if (action && resource) {
        hasAccess = can(action, resource);
    }

    // If require is true, throw error instead of hiding
    if (requirePermission && !hasAccess) {
        try {
            requirePerm(permission || `${action}-${resource}`);
        } catch (error) {
            throw error;
        }
    }

    // If user doesn't have access, show fallback or nothing
    if (!hasAccess) {
        return fallback || null;
    }

    return <>{children}</>;
}
