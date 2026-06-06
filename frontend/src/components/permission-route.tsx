import { Navigate, type ReactNode } from 'react-router-dom';
import { useAuth } from '@/contexts/AuthContext';

function PageLoader() {
    return (
        <div className="flex min-h-[200px] items-center justify-center">
            <div className="h-8 w-8 animate-spin rounded-full border-4 border-primary border-t-transparent" />
        </div>
    );
}

/** Redirects to /unauthorized when the user lacks the required permission slug. */
export function PermissionRoute({
    permission,
    children,
}: {
    permission?: string;
    children: ReactNode;
}) {
    const { user, loading, hasPermission } = useAuth();
    if (loading) return <PageLoader />;
    if (!user) return <Navigate to="/login" replace />;
    if (permission && !hasPermission(permission)) {
        return <Navigate to="/unauthorized" replace />;
    }
    return <>{children}</>;
}
