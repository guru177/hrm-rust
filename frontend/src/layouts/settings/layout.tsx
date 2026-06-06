import { Link } from 'react-router-dom';
import { type PropsWithChildren } from 'react';
import { User } from 'lucide-react';

import Heading from '@/components/heading';
import { Button } from '@/components/ui/button';
import { Separator } from '@/components/ui/separator';
import { useActiveUrl } from '@/hooks/use-active-url';
import { cn } from '@/lib/utils';
import { useAuth } from '@/contexts/AuthContext';
import { type NavItem } from '@/types';

const sidebarNavItems: NavItem[] = [
    { title: 'Profile', href: '/admin/settings/profile', icon: null },
    { title: 'Password', href: '/admin/settings/password', icon: null },
    { title: 'Appearance', href: '/admin/settings/appearance', icon: null },
];

const adminNavItems: NavItem[] = [
    { title: 'General Settings', href: '/admin/settings/app', icon: null },
    { title: 'Leave Types', href: '/admin/settings/leave-types', icon: null },
];

export default function SettingsLayout({ children }: PropsWithChildren) {
    const { urlIsActive } = useActiveUrl();
    const { user, permissions } = useAuth();

    if (typeof window === 'undefined') return null;

    const isAdmin = user?.roles?.some((role) => role.slug === 'admin') ?? permissions.includes('*');
    const allNavItems = [...sidebarNavItems, ...(isAdmin ? adminNavItems : [])];

    return (
        <div className="px-4 py-6">
            {/* Hero Header */}
            <div className="relative overflow-hidden rounded-2xl bg-gradient-to-br from-[#e8f2fd] via-[#d0e4f8] to-[#c4d8f0] dark:from-[#0d1e33] dark:via-[#0a1828] dark:to-[#071220] px-6 py-5 shadow-sm border border-white/60 dark:border-white/10 mb-8">
                <div className="pointer-events-none absolute -top-10 -right-10 w-48 h-48 opacity-20">
                    <svg viewBox="0 0 200 200" xmlns="http://www.w3.org/2000/svg">
                        <path fill="#071b3a" d="M44.7,-76.4C58.4,-69.7,70.3,-58.6,77.9,-44.9C85.5,-31.2,88.7,-15.6,87.4,-0.8C86,14,80,28,72.1,40.5C64.2,53,54.2,64,42.1,71.3C30,78.6,15,82.3,0.1,82.1C-14.8,81.9,-29.6,77.8,-42.7,70.5C-55.8,63.2,-67.3,52.7,-74.5,39.5C-81.7,26.3,-84.7,10.5,-83.1,-4.9C-81.6,-20.3,-75.5,-35.2,-66.3,-47.4C-57.1,-59.6,-44.8,-69.1,-31.6,-76.1C-18.4,-83.1,-4.6,-87.6,8.2,-86.2C21,-84.8,31,-83.1,44.7,-76.4Z" transform="translate(100 100)" />
                    </svg>
                </div>
                <div className="relative flex items-center justify-between gap-4">
                    <div className="flex items-center gap-4">
                        <div className="flex h-12 w-12 shrink-0 items-center justify-center rounded-xl bg-[#071b3a]/15 dark:bg-white/10 border border-[#071b3a]/20 dark:border-white/10 shadow-inner">
                            <User className="h-6 w-6 text-[#071b3a] dark:text-blue-300" />
                        </div>
                        <div>
                            <h1 className="text-xl font-bold tracking-tight text-[#001f3f] dark:text-white">Settings</h1>
                            <p className="text-sm text-[#1e3a5f]/60 dark:text-blue-200/60 mt-1">Manage your profile and account settings</p>
                        </div>
                    </div>
                </div>
            </div>

            <div className="flex flex-col lg:flex-row lg:space-x-8 items-start">
                <aside className="w-full lg:w-64 shrink-0">
                    <nav className="flex flex-col space-y-1.5 rounded-2xl bg-white/40 dark:bg-slate-900/40 backdrop-blur-md border border-white/60 dark:border-white/10 p-3 shadow-sm" aria-label="Settings">
                        {allNavItems.map((item, index) => {
                            const isActive = urlIsActive(item.href);
                            return (
                                <Button
                                    key={`${item.href}-${index}`}
                                    size="sm"
                                    variant="ghost"
                                    asChild
                                    className={cn(
                                        'w-full justify-start rounded-xl px-4 py-2.5 h-auto transition-all duration-200',
                                        isActive
                                            ? 'bg-gradient-to-r from-[#e8f2fd] to-[#d0e4f8] dark:from-[#0d1e33] dark:to-[#0a1828] text-[#001f3f] hover:text-[#001f3f] dark:text-blue-200 dark:hover:text-blue-200 shadow-sm border border-white/60 dark:border-white/5 font-semibold'
                                            : 'hover:bg-white/60 dark:hover:bg-white/5 text-[#1e3a5f]/70 dark:text-slate-400 hover:text-[#001f3f] dark:hover:text-blue-200 font-medium'
                                    )}
                                >
                                    <Link to={item.href} className="flex items-center gap-3">
                                        {item.icon && <item.icon className="h-4 w-4" />}
                                        {item.title}
                                    </Link>
                                </Button>
                            );
                        })}
                    </nav>
                </aside>

                <Separator className="my-6 lg:hidden" />

                <div className="flex-1">
                    <section className="space-y-12">{children}</section>
                </div>
            </div>
        </div>
    );
}
