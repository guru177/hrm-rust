import { Link, useLocation } from 'react-router-dom';
import {
    Award, BarChart3, Briefcase, Building, CalendarCheck, ClipboardList, Clock,
    DollarSign, FileText, Fingerprint, Folder, IndianRupee, LayoutGrid, MapPin, Users, UsersRound,
    Wallet, Workflow, Calendar, FileCheck, Settings,
} from 'lucide-react';

import { NavFooter } from '@/components/nav-footer';
import { NavMain } from '@/components/nav-main';
import { NavUser } from '@/components/nav-user';
import {
    Sidebar, SidebarContent, SidebarFooter, SidebarHeader,
    SidebarMenu, SidebarMenuButton, SidebarMenuItem,
} from '@/components/ui/sidebar';
import { type NavGroup, type NavItem } from '@/types';
import { useAuth } from '@/contexts/AuthContext';
import AppLogo from './app-logo';

type NavItemWithPerm = NavItem & { permission?: string };
type NavGroupWithPerm = Omit<NavGroup, 'items'> & { items: NavItemWithPerm[] };
type NavEntry = NavItemWithPerm | NavGroupWithPerm;

const mainNavItems: NavEntry[] = [
    { title: 'Dashboard', href: '/admin/dashboard', icon: LayoutGrid, permission: 'view-dashboard' },
    { title: 'Users & Roles', href: '/admin/users', icon: Users, permission: 'view-users' },
    { title: 'Centers', href: '/admin/centers', icon: MapPin, permission: 'manage-settings' },
    { title: 'Departments', href: '/admin/departments', icon: Building, permission: 'view-departments' },
    { title: 'Designations', href: '/admin/designations', icon: Award, permission: 'view-designations' },
    { title: 'Job Postings', href: '/admin/careers', icon: Briefcase, permission: 'view-jobs' },
    { title: 'Applications', href: '/admin/job-applications', icon: FileCheck, permission: 'view-jobs' },
    { title: 'Attendance', href: '/admin/attendance', icon: CalendarCheck, permission: 'view-attendance' },
    {
        title: 'Shifts', icon: Clock, permission: 'view-attendance', items: [
            { title: 'Templates & Assign', href: '/admin/shifts', icon: Clock, permission: 'view-attendance' },
            { title: 'Shift Roster', href: '/admin/shifts/roster', icon: UsersRound, permission: 'view-attendance' },
        ],
    },
    { title: 'Biometric Devices', href: '/admin/biometric', icon: Fingerprint, permission: 'view-attendance' },
    { title: 'Leave Requests', href: '/admin/leave-requests/manage', icon: FileCheck, permission: 'manage-leave-requests' },
    { title: 'Holidays', href: '/admin/holidays', icon: Calendar, permission: 'view-holidays' },
    {
        title: 'Salaries', icon: Wallet, items: [
            { title: 'Salary Components', href: '/admin/salaries/components', icon: DollarSign, permission: 'view-payroll' },
            { title: 'Employees', href: '/admin/salaries/employees', icon: UsersRound, permission: 'view-payroll' },
            { title: 'Payroll', href: '/admin/payroll', icon: IndianRupee, permission: 'view-payroll' },
        ],
    },
    { title: 'Workflows', href: '/admin/workflows', icon: Workflow, permission: 'view-workflows' },
    { title: 'Tasks & Activities', href: '/admin/tasks', icon: ClipboardList, permission: 'view-tasks' },
    { title: 'Projects', href: '/admin/projects', icon: Folder, permission: 'view-projects' },
    { title: 'Reports', href: '/admin/reports', icon: BarChart3, permission: 'view-payroll' },
];

function hasPermissionCheck(permissions: string[], slug: string | undefined): boolean {
    if (!slug) return true;
    if (permissions.includes('*')) return true;
    return permissions.includes(slug);
}

function filterNav(items: NavEntry[], permissions: string[]): NavEntry[] {
    return items.reduce<NavEntry[]>((acc, item) => {
        if ('items' in item) {
            const visibleChildren = item.items.filter((child) => hasPermissionCheck(permissions, child.permission));
            if (visibleChildren.length > 0) acc.push({ ...item, items: visibleChildren });
        } else {
            if (hasPermissionCheck(permissions, (item as NavItemWithPerm).permission)) acc.push(item);
        }
        return acc;
    }, []);
}

export function AppSidebar() {
    const { permissions } = useAuth();
    const isFullAccess = permissions.includes('*');
    const filteredMain = filterNav(mainNavItems, permissions);

    const navItems: NavEntry[] = [
        ...filteredMain,
        ...(isFullAccess ? [{ title: 'App Settings', href: '/admin/settings/app', icon: Settings } as NavItemWithPerm] : []),
    ];

    return (
        <Sidebar collapsible="icon" variant="inset">
            <SidebarHeader>
                <SidebarMenu>
                    <SidebarMenuItem>
                        <SidebarMenuButton size="lg" asChild>
                            <Link to="/admin/dashboard">
                                <AppLogo />
                            </Link>
                        </SidebarMenuButton>
                    </SidebarMenuItem>
                </SidebarMenu>
            </SidebarHeader>
            <SidebarContent>
                <NavMain items={navItems as (NavItem | NavGroup)[]} />
            </SidebarContent>
            <SidebarFooter>
                <NavFooter items={[]} className="mt-auto" />
                <NavUser />
            </SidebarFooter>
        </Sidebar>
    );
}
