import { ReactNode } from 'react';

import { Button, type ButtonProps } from '@/components/ui/button';
import {
    Tooltip,
    TooltipContent,
    TooltipProvider,
    TooltipTrigger,
} from '@/components/ui/tooltip';
import { usePermissions } from '@/hooks/use-permissions';

interface PermissionButtonProps extends Omit<ButtonProps, 'children'> {
    permission?: string;
    action?: string;
    resource?: string;
    children: ReactNode;
    showTooltip?: boolean;
    tooltipMessage?: string;
}

/**
 * A button component that checks permissions and disables/hides based on access
 * 
 * Usage:
 *   <PermissionButton permission="create-users">
 *     Create Contact
 *   </PermissionButton>
 * 
 *   <PermissionButton action="delete" resource="users" variant="destructive">
 *     Delete User
 *   </PermissionButton>
 */
export function PermissionButton({
    permission,
    action,
    resource,
    children,
    showTooltip = true,
    tooltipMessage,
    ...buttonProps
}: PermissionButtonProps) {
    const { hasPermission, can } = usePermissions();

    // Determine if user has permission
    let hasAccess = true;

    if (permission) {
        hasAccess = hasPermission(permission);
    } else if (action && resource) {
        hasAccess = can(action, resource);
    }

    // If user doesn't have access, return disabled button or nothing
    if (!hasAccess) {
        const message = tooltipMessage || `You don't have permission to perform this action`;

        if (showTooltip) {
            return (
                <TooltipProvider>
                    <Tooltip>
                        <TooltipTrigger asChild>
                            <div className="inline-block">
                                <Button {...buttonProps} disabled>
                                    {children}
                                </Button>
                            </div>
                        </TooltipTrigger>
                        <TooltipContent>
                            <p>{message}</p>
                        </TooltipContent>
                    </Tooltip>
                </TooltipProvider>
            );
        }

        return (
            <Button {...buttonProps} disabled>
                {children}
            </Button>
        );
    }

    return <Button {...buttonProps}>{children}</Button>;
}
