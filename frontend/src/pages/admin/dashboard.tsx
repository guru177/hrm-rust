import { useEffect, useState } from 'react';
import { apiGet } from '@/lib/api';
import { handleApiError } from '@/lib/toast';
import {
    TrendingUp,
    DollarSign,
    Users,
    FileText,
    CheckCircle2,
    Clock,
    TargetIcon,
    Calendar,
    Cake,
    Gift,
    ArrowUpRight,
    ArrowDownRight,
    Activity,
    Zap,
} from 'lucide-react';
import {
    BarChart,
    Bar,
    PieChart,
    Pie,
    Cell,
    XAxis,
    YAxis,
    CartesianGrid,
    Tooltip,
    ResponsiveContainer,
} from 'recharts';

import { Badge } from '@/components/ui/badge';
import AppLayout from '@/layouts/app-layout';
import { type BreadcrumbItem } from '@/types';

const breadcrumbs: BreadcrumbItem[] = [
    {
        title: 'Dashboard',
        href: '/admin/dashboard',
    },
];

interface HRDashboardData {
    metrics: {
        totalEmployees: number;
        attendancePercentage: number;
        attendanceCount: number;
        pendingRequests: number;
        activeProjects: number;
    };
    attendance: {
        leaveTypes: Record<string, number>;
        trends: Array<{ date: string; percentage: number; count: number }>;
        upcomingHolidays: Array<{ name: string; date: string; daysAway: number }>;
    };
    payroll: {
        currentMonth: number;
        previousMonth: number;
        change: number;
        byDepartment: Array<{
            department: string;
            totalCost: number;
            employees: number;
            average: number;
        }>;
    };
    operations: {
        taskProgress: { todo: number; in_progress: number; completed: number; on_hold: number };
        celebrations: Array<{ name: string; type: string; date: string; isSoon: boolean }>;
        recentWorkflows: Array<{
            id: string;
            process: string;
            status: string;
            step: string;
            timestamp: string;
        }>;
    };
}

const EMPTY_DASHBOARD_DATA: HRDashboardData = {
    metrics: {
        totalEmployees: 0,
        attendancePercentage: 0,
        attendanceCount: 0,
        pendingRequests: 0,
        activeProjects: 0,
    },
    attendance: {
        leaveTypes: {},
        trends: [],
        upcomingHolidays: [],
    },
    payroll: {
        currentMonth: 0,
        previousMonth: 0,
        change: 0,
        byDepartment: [],
    },
    operations: {
        taskProgress: {
            todo: 0,
            in_progress: 0,
            completed: 0,
            on_hold: 0,
        },
        celebrations: [],
        recentWorkflows: [],
    },
};

const CHART_COLORS = ['#071b3a', '#ef4444', '#10b981', '#f59e0b', '#8b5cf6', '#ec4899'];

// ─── Reusable glassy card wrapper ────────────────────────────────────────────
function GlassCard({
    children,
    className = '',
}: {
    children: React.ReactNode;
    className?: string;
}) {
    return (
        <div
            className={`relative overflow-hidden rounded-2xl bg-white/70 dark:bg-white/5 backdrop-blur-md border border-white/80 dark:border-white/10 shadow-[0_8px_32px_rgba(7,27,58,0.07)] dark:shadow-[0_8px_32px_rgba(0,0,0,0.3)] ${className}`}
        >
            <div className="absolute inset-x-0 top-0 h-px bg-gradient-to-r from-transparent via-blue-300/50 to-transparent dark:via-blue-500/20 pointer-events-none" />
            {children}
        </div>
    );
}

// ─── Section header ───────────────────────────────────────────────────────────
function SectionHeader({ children, icon: Icon }: { children: React.ReactNode; icon: any }) {
    return (
        <div className="flex items-center gap-3 mb-5">
            <div className="flex h-8 w-8 items-center justify-center rounded-xl bg-gradient-to-br from-[#071b3a] to-[#0d4a8a] shadow-md shadow-blue-500/25">
                <Icon className="h-4 w-4 text-white" />
            </div>
            <h2 className="text-lg font-bold tracking-tight text-foreground">{children}</h2>
            <div className="flex-1 h-px bg-gradient-to-r from-blue-200/60 to-transparent dark:from-blue-700/30" />
        </div>
    );
}

// ─── Metric card (glassy + animated) ─────────────────────────────────────────
function MetricCard({
    label,
    value,
    unit,
    icon: Icon,
    change,
    isPositive = true,
    gradient,
    shadow,
}: {
    label: string;
    value: number | string;
    unit?: string;
    icon: any;
    change?: number;
    isPositive?: boolean;
    gradient: string;
    shadow: string;
}) {
    return (
        <GlassCard className="hover:-translate-y-1 transition-transform duration-300 group">
            <div className="p-5">
                <div className="flex items-start justify-between mb-4">
                    <p className="text-xs font-medium text-muted-foreground/70 uppercase tracking-wider">{label}</p>
                    <div className={`flex h-10 w-10 items-center justify-center rounded-xl bg-gradient-to-br ${gradient} shadow-lg ${shadow} group-hover:scale-110 transition-transform duration-300`}>
                        <Icon className="h-5 w-5 text-white" />
                    </div>
                </div>
                <div className="flex items-baseline gap-2 mb-2">
                    <p className={`text-3xl font-bold bg-gradient-to-r ${gradient} bg-clip-text text-transparent`}>
                        {value}
                    </p>
                    {unit && <span className="text-sm text-muted-foreground">{unit}</span>}
                </div>
                {change !== undefined && (
                    <div className="flex items-center gap-1 text-xs">
                        {isPositive ? (
                            <ArrowUpRight className="h-3.5 w-3.5 text-emerald-500" />
                        ) : (
                            <ArrowDownRight className="h-3.5 w-3.5 text-red-500" />
                        )}
                        <span className={isPositive ? 'text-emerald-600 dark:text-emerald-400 font-medium' : 'text-red-500 font-medium'}>
                            {Math.abs(change)}% from last month
                        </span>
                    </div>
                )}
            </div>
        </GlassCard>
    );
}

// ─── Custom chart tooltip ─────────────────────────────────────────────────────
function CustomTooltip({ active, payload, label }: any) {
    if (active && payload && payload.length) {
        return (
            <div className="rounded-xl bg-white/90 dark:bg-slate-900/90 backdrop-blur-md border border-blue-100/80 dark:border-white/10 shadow-xl px-3 py-2">
                {label && <p className="text-xs text-muted-foreground mb-1">{label}</p>}
                {payload.map((p: any, i: number) => (
                    <p key={i} className="text-sm font-semibold" style={{ color: p.color || p.fill }}>
                        {typeof p.value === 'number' && p.name?.toLowerCase().includes('%')
                            ? `${p.value}%`
                            : p.value}
                    </p>
                ))}
            </div>
        );
    }
    return null;
}

// ─── Main Dashboard ───────────────────────────────────────────────────────────
export default function Dashboard() {
    const [hrData, setHrData] = useState<HRDashboardData | null>(null);
    const [loading, setLoading] = useState(true);

    useEffect(() => {
        document.title = 'Dashboard — HRM Portal';
        apiGet<HRDashboardData>('/admin/dashboard/hr-data')
            .then((res) => setHrData(res.data))
            .catch((err) => handleApiError(err))
            .finally(() => setLoading(false));
    }, []);

    const data: HRDashboardData = {
        metrics: { ...EMPTY_DASHBOARD_DATA.metrics, ...(hrData?.metrics ?? {}) },
        attendance: { ...EMPTY_DASHBOARD_DATA.attendance, ...(hrData?.attendance ?? {}) },
        payroll: { ...EMPTY_DASHBOARD_DATA.payroll, ...(hrData?.payroll ?? {}) },
        operations: {
            ...EMPTY_DASHBOARD_DATA.operations,
            ...(hrData?.operations ?? {}),
            taskProgress: { ...EMPTY_DASHBOARD_DATA.operations.taskProgress, ...(hrData?.operations?.taskProgress ?? {}) },
        },
    };

    const metricCards = [
        {
            label: 'Total Employees',
            value: data.metrics.totalEmployees,
            icon: Users,
            gradient: 'from-[#071b3a] to-[#0d4a8a]',
            shadow: 'shadow-blue-500/30',
        },
        {
            label: "Today's Attendance",
            value: data.metrics.attendancePercentage,
            unit: '%',
            icon: CheckCircle2,
            gradient: 'from-emerald-500 to-teal-500',
            shadow: 'shadow-emerald-500/30',
        },
        {
            label: 'Pending Requests',
            value: data.metrics.pendingRequests,
            icon: Clock,
            change: 0,
            gradient: 'from-amber-500 to-orange-500',
            shadow: 'shadow-amber-500/30',
        },
        {
            label: 'Active Projects',
            value: data.metrics.activeProjects,
            icon: TargetIcon,
            gradient: 'from-violet-500 to-purple-600',
            shadow: 'shadow-violet-500/30',
        },
        {
            label: 'Total Payroll',
            value: `₹${(data.payroll.currentMonth / 1000).toFixed(1)}K`,
            icon: DollarSign,
            change: data.payroll.change,
            isPositive: data.payroll.change >= 0,
            gradient: 'from-pink-500 to-rose-500',
            shadow: 'shadow-pink-500/30',
        },
    ];

    return (
        <AppLayout breadcrumbs={breadcrumbs}>

            <div className="flex h-full flex-1 flex-col gap-6 sm:gap-8 overflow-x-auto p-4 sm:p-6">

                {/* ── Hero Banner ── */}
                <div className="relative overflow-hidden rounded-2xl bg-gradient-to-br from-[#e8f2fd] via-[#d0e4f8] to-[#c4d8f0] dark:from-[#0d1e33] dark:via-[#0a1828] dark:to-[#071220] px-4 sm:px-6 py-4 sm:py-5 border border-white/60 dark:border-white/10 shadow-sm">
                    <div className="pointer-events-none absolute -top-12 -right-12 w-56 h-56 opacity-15">
                        <svg viewBox="0 0 200 200" xmlns="http://www.w3.org/2000/svg">
                            <path fill="#071b3a" d="M44.7,-76.4C58.4,-69.7,70.3,-58.6,77.9,-44.9C85.5,-31.2,88.7,-15.6,87.4,-0.8C86,14,80,28,72.1,40.5C64.2,53,54.2,64,42.1,71.3C30,78.6,15,82.3,0.1,82.1C-14.8,81.9,-29.6,77.8,-42.7,70.5C-55.8,63.2,-67.3,52.7,-74.5,39.5C-81.7,26.3,-84.7,10.5,-83.1,-4.9C-81.6,-20.3,-75.5,-35.2,-66.3,-47.4C-57.1,-59.6,-44.8,-69.1,-31.6,-76.1C-18.4,-83.1,-4.6,-87.6,8.2,-86.2C21,-84.8,31,-83.1,44.7,-76.4Z" transform="translate(100 100)" />
                        </svg>
                    </div>
                    <div className="relative flex items-center gap-4">
                        <div className="flex h-10 w-10 sm:h-12 sm:w-12 shrink-0 items-center justify-center rounded-xl bg-[#071b3a]/15 dark:bg-white/10 border border-[#071b3a]/20 dark:border-white/10">
                            <Activity className="h-5 w-5 sm:h-6 sm:w-6 text-[#071b3a] dark:text-blue-300" />
                        </div>
                        <div>
                            <h1 className="text-lg sm:text-xl font-bold tracking-tight text-[#001f3f] dark:text-white">
                                HR Dashboard
                            </h1>
                            <p className="text-sm text-[#1e3a5f]/60 dark:text-blue-200/60">
                                Real-time overview of your workforce metrics
                            </p>
                        </div>
                    </div>
                </div>

                {/* ── Section 1: High-Level Metrics ── */}
                <div>
                    <SectionHeader icon={Zap}>High-Level Metrics</SectionHeader>
                    {loading ? (
                        <div className="grid gap-4 grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-5">
                            {[...Array(5)].map((_, i) => (
                                <div key={i} className="h-32 animate-pulse rounded-2xl bg-blue-100/40 dark:bg-white/5" />
                            ))}
                        </div>
                    ) : (
                        <div className="grid gap-4 grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-5">
                            {metricCards.map((card, i) => (
                                <MetricCard key={i} {...card} />
                            ))}
                        </div>
                    )}
                </div>

                {/* ── Section 2: Time & Attendance ── */}
                <div>
                    <SectionHeader icon={Calendar}>Time &amp; Attendance</SectionHeader>
                    {loading ? (
                        <div className="grid gap-6 grid-cols-1 lg:grid-cols-3">
                            {[...Array(3)].map((_, i) => (
                                <div key={i} className="h-64 animate-pulse rounded-2xl bg-blue-100/40 dark:bg-white/5" />
                            ))}
                        </div>
                    ) : (
                        <div className="grid gap-6 grid-cols-1 lg:grid-cols-3">
                            {/* Leave Overview */}
                            <GlassCard>
                                <div className="px-5 pt-5 pb-1">
                                    <h3 className="font-semibold text-sm text-foreground">Leave Overview</h3>
                                    <p className="text-xs text-muted-foreground/60 mt-0.5">Types of leaves this month</p>
                                </div>
                                <div className="h-52 w-full px-2">
                                    <ResponsiveContainer width="100%" height="100%">
                                        <PieChart>
                                            <Pie
                                                data={Object.entries(data.attendance.leaveTypes).map(([name, value]) => ({ name, value }))}
                                                cx="50%"
                                                cy="50%"
                                                innerRadius={55}
                                                outerRadius={82}
                                                paddingAngle={3}
                                                dataKey="value"
                                                strokeWidth={0}
                                            >
                                                {Object.keys(data.attendance.leaveTypes).map((_, index) => (
                                                    <Cell key={`cell-${index}`} fill={CHART_COLORS[index % CHART_COLORS.length]} />
                                                ))}
                                            </Pie>
                                            <Tooltip content={<CustomTooltip />} />
                                        </PieChart>
                                    </ResponsiveContainer>
                                </div>
                                <div className="px-5 pb-5 space-y-2">
                                    {Object.entries(data.attendance.leaveTypes).map(([type, count], index) => (
                                        <div key={type} className="flex items-center justify-between text-sm">
                                            <div className="flex items-center gap-2">
                                                <div className="h-2.5 w-2.5 rounded-full shrink-0" style={{ backgroundColor: CHART_COLORS[index % CHART_COLORS.length] }} />
                                                <span className="capitalize text-foreground/80">{type}</span>
                                            </div>
                                            <span className="font-bold text-foreground">{count}</span>
                                        </div>
                                    ))}
                                </div>
                            </GlassCard>

                            {/* Attendance Trends */}
                            <GlassCard>
                                <div className="px-5 pt-5 pb-1">
                                    <h3 className="font-semibold text-sm text-foreground">Attendance Trends</h3>
                                    <p className="text-xs text-muted-foreground/60 mt-0.5">Last 7 days</p>
                                </div>
                                <div className="h-64 w-full px-2 pb-4">
                                    <ResponsiveContainer width="100%" height="100%">
                                        <BarChart data={data.attendance.trends} barSize={20}>
                                            <defs>
                                                <linearGradient id="barGrad" x1="0" y1="0" x2="0" y2="1">
                                                    <stop offset="0%" stopColor="#071b3a" />
                                                    <stop offset="100%" stopColor="#0d4a8a" />
                                                </linearGradient>
                                            </defs>
                                            <CartesianGrid strokeDasharray="3 3" stroke="rgba(7,27,58,0.08)" vertical={false} />
                                            <XAxis dataKey="date" tick={{ fontSize: 11, fill: '#94a3b8' }} axisLine={false} tickLine={false} />
                                            <YAxis tick={{ fontSize: 11, fill: '#94a3b8' }} axisLine={false} tickLine={false} />
                                            <Tooltip content={<CustomTooltip />} cursor={{ fill: 'rgba(7,27,58,0.05)' }} />
                                            <Bar dataKey="percentage" fill="url(#barGrad)" radius={[8, 8, 0, 0]} />
                                        </BarChart>
                                    </ResponsiveContainer>
                                </div>
                            </GlassCard>

                            {/* Upcoming Holidays */}
                            <GlassCard>
                                <div className="px-5 pt-5 pb-3">
                                    <h3 className="font-semibold text-sm text-foreground">Upcoming Holidays</h3>
                                    <p className="text-xs text-muted-foreground/60 mt-0.5">Next events</p>
                                </div>
                                <div className="px-5 pb-5 space-y-2.5">
                                    {data.attendance.upcomingHolidays.length > 0 ? (
                                        data.attendance.upcomingHolidays.map((holiday, index) => (
                                            <div key={index} className="group flex items-center justify-between rounded-xl border border-blue-100/60 dark:border-white/8 bg-blue-50/40 dark:bg-white/3 p-3 hover:border-blue-300/60 dark:hover:border-blue-600/40 transition-colors">
                                                <div className="flex items-center gap-3">
                                                    <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg bg-gradient-to-br from-[#071b3a] to-[#0d4a8a] shadow-sm">
                                                        <Calendar className="h-4 w-4 text-white" />
                                                    </div>
                                                    <div>
                                                        <p className="text-sm font-semibold text-foreground">{holiday.name}</p>
                                                        <p className="text-xs text-muted-foreground/60">{holiday.date}</p>
                                                    </div>
                                                </div>
                                                <span className="inline-flex items-center justify-center h-6 min-w-[32px] px-2 rounded-full bg-[#071b3a]/10 dark:bg-blue-900/30 text-[11px] font-bold text-[#071b3a] dark:text-blue-300 border border-[#071b3a]/15">
                                                    {holiday.daysAway > 0 ? `${holiday.daysAway}d` : 'Today'}
                                                </span>
                                            </div>
                                        ))
                                    ) : (
                                        <p className="text-sm text-muted-foreground/50 py-4 text-center">No upcoming holidays</p>
                                    )}
                                </div>
                            </GlassCard>
                        </div>
                    )}
                </div>

                {/* ── Section 3: Payroll & Salaries ── */}
                <div>
                    <SectionHeader icon={DollarSign}>Payroll &amp; Salaries</SectionHeader>
                    {loading ? (
                        <div className="grid gap-6 grid-cols-1 lg:grid-cols-2">
                            {[...Array(2)].map((_, i) => (
                                <div key={i} className="h-64 animate-pulse rounded-2xl bg-blue-100/40 dark:bg-white/5" />
                            ))}
                        </div>
                    ) : (
                        <div className="grid gap-6 grid-cols-1 lg:grid-cols-2">
                            {/* Monthly Payroll Summary */}
                            <GlassCard>
                                <div className="px-5 pt-5 pb-3">
                                    <h3 className="font-semibold text-sm text-foreground">Monthly Payroll Summary</h3>
                                    <p className="text-xs text-muted-foreground/60 mt-0.5">Current vs previous month</p>
                                </div>
                                <div className="px-5 pb-5 space-y-4">
                                    {/* Current Month */}
                                    <div className="rounded-xl bg-gradient-to-br from-[#071b3a]/10 to-[#0d4a8a]/10 dark:from-blue-900/30 dark:to-blue-800/20 border border-[#071b3a]/15 dark:border-blue-700/30 p-4">
                                        <p className="text-xs text-muted-foreground/70 mb-1">Current Month</p>
                                        <p className="text-3xl font-bold bg-gradient-to-r from-[#071b3a] to-[#0d4a8a] bg-clip-text text-transparent">
                                            ₹{(data.payroll.currentMonth / 1000).toFixed(1)}K
                                        </p>
                                    </div>
                                    {/* Previous Month */}
                                    <div className="rounded-xl border border-blue-100/60 dark:border-white/8 bg-white/50 dark:bg-white/3 p-4">
                                        <p className="text-xs text-muted-foreground/70 mb-1">Previous Month</p>
                                        <p className="text-2xl font-semibold text-foreground">
                                            ₹{(data.payroll.previousMonth / 1000).toFixed(1)}K
                                        </p>
                                    </div>
                                    {/* Change indicator */}
                                    <div className={`flex items-center gap-3 rounded-xl border p-4 ${data.payroll.change >= 0 ? 'border-emerald-200/60 dark:border-emerald-700/30 bg-emerald-50/50 dark:bg-emerald-900/15' : 'border-red-200/60 dark:border-red-700/30 bg-red-50/50 dark:bg-red-900/15'}`}>
                                        {data.payroll.change >= 0 ? (
                                            <div className="flex h-9 w-9 items-center justify-center rounded-xl bg-emerald-500 shadow-md shadow-emerald-500/30">
                                                <ArrowUpRight className="h-5 w-5 text-white" />
                                            </div>
                                        ) : (
                                            <div className="flex h-9 w-9 items-center justify-center rounded-xl bg-red-500 shadow-md shadow-red-500/30">
                                                <ArrowDownRight className="h-5 w-5 text-white" />
                                            </div>
                                        )}
                                        <div>
                                            <p className={`font-semibold text-sm ${data.payroll.change >= 0 ? 'text-emerald-700 dark:text-emerald-400' : 'text-red-600 dark:text-red-400'}`}>
                                                {Math.abs(data.payroll.change)}% {data.payroll.change >= 0 ? 'increase' : 'decrease'}
                                            </p>
                                            <p className="text-xs text-muted-foreground/60">from last month</p>
                                        </div>
                                    </div>
                                </div>
                            </GlassCard>

                            {/* Salary Distribution by Department */}
                            <GlassCard>
                                <div className="px-5 pt-5 pb-3">
                                    <h3 className="font-semibold text-sm text-foreground">Salary Distribution</h3>
                                    <p className="text-xs text-muted-foreground/60 mt-0.5">By department</p>
                                </div>
                                <div className="px-5 pb-5 space-y-2.5 max-h-72 overflow-y-auto">
                                    {data.payroll.byDepartment.length > 0 ? (
                                        data.payroll.byDepartment.map((dept, index) => (
                                            <div key={index} className="flex items-center justify-between rounded-xl border border-blue-100/60 dark:border-white/8 bg-blue-50/30 dark:bg-white/3 p-3 hover:border-blue-300/60 dark:hover:border-blue-600/40 transition-colors">
                                                <div className="flex items-center gap-3">
                                                    <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg bg-gradient-to-br from-[#071b3a]/15 to-[#0d4a8a]/15 border border-[#071b3a]/15 dark:border-blue-700/30">
                                                        <span className="text-[10px] font-bold text-[#071b3a] dark:text-blue-300">
                                                            {dept.department.charAt(0)}
                                                        </span>
                                                    </div>
                                                    <div>
                                                        <p className="font-semibold text-sm text-foreground">{dept.department}</p>
                                                        <p className="text-xs text-muted-foreground/60">{dept.employees} employees</p>
                                                    </div>
                                                </div>
                                                <div className="text-right">
                                                    <p className="font-bold text-sm bg-gradient-to-r from-[#071b3a] to-[#0d4a8a] bg-clip-text text-transparent">
                                                        ₹{(dept.totalCost / 1000).toFixed(1)}K
                                                    </p>
                                                    <p className="text-xs text-muted-foreground/60">
                                                        Avg ₹{(dept.average / 1000).toFixed(1)}K
                                                    </p>
                                                </div>
                                            </div>
                                        ))
                                    ) : (
                                        <p className="text-sm text-muted-foreground/50 py-8 text-center">No payroll data available</p>
                                    )}
                                </div>
                            </GlassCard>
                        </div>
                    )}
                </div>

                {/* ── Section 4: Operations & Tasks ── */}
                <div>
                    <SectionHeader icon={FileText}>Operations &amp; Tasks</SectionHeader>
                    {loading ? (
                        <div className="grid gap-6 grid-cols-1 lg:grid-cols-3">
                            {[...Array(3)].map((_, i) => (
                                <div key={i} className="h-64 animate-pulse rounded-2xl bg-blue-100/40 dark:bg-white/5" />
                            ))}
                        </div>
                    ) : (
                        <div className="grid gap-6 grid-cols-1 lg:grid-cols-3">
                            {/* Task Progress */}
                            <GlassCard>
                                <div className="px-5 pt-5 pb-1">
                                    <h3 className="font-semibold text-sm text-foreground">Task Progress</h3>
                                    <p className="text-xs text-muted-foreground/60 mt-0.5">Status overview</p>
                                </div>
                                <div className="h-52 w-full px-2">
                                    <ResponsiveContainer width="100%" height="100%">
                                        <PieChart>
                                            <Pie
                                                data={[
                                                    { name: 'To-Do', value: data.operations.taskProgress.todo },
                                                    { name: 'In Progress', value: data.operations.taskProgress.in_progress },
                                                    { name: 'Completed', value: data.operations.taskProgress.completed },
                                                    { name: 'On Hold', value: data.operations.taskProgress.on_hold },
                                                ]}
                                                cx="50%"
                                                cy="50%"
                                                innerRadius={55}
                                                outerRadius={82}
                                                paddingAngle={3}
                                                dataKey="value"
                                                strokeWidth={0}
                                            >
                                                <Cell fill="#ef4444" />
                                                <Cell fill="#f59e0b" />
                                                <Cell fill="#10b981" />
                                                <Cell fill="#6b7280" />
                                            </Pie>
                                            <Tooltip content={<CustomTooltip />} />
                                        </PieChart>
                                    </ResponsiveContainer>
                                </div>
                                <div className="px-5 pb-5 space-y-2">
                                    {[
                                        { label: 'To-Do', color: 'bg-red-500', value: data.operations.taskProgress.todo },
                                        { label: 'In Progress', color: 'bg-amber-500', value: data.operations.taskProgress.in_progress },
                                        { label: 'Completed', color: 'bg-emerald-500', value: data.operations.taskProgress.completed },
                                        { label: 'On Hold', color: 'bg-gray-500', value: data.operations.taskProgress.on_hold },
                                    ].map(({ label, color, value }) => (
                                        <div key={label} className="flex items-center justify-between text-sm">
                                            <div className="flex items-center gap-2">
                                                <div className={`h-2.5 w-2.5 rounded-full ${color}`} />
                                                <span className="text-foreground/80">{label}</span>
                                            </div>
                                            <span className="font-bold text-foreground">{value}</span>
                                        </div>
                                    ))}
                                </div>
                            </GlassCard>

                            {/* Celebrations */}
                            <GlassCard>
                                <div className="px-5 pt-5 pb-3">
                                    <h3 className="font-semibold text-sm text-foreground">Celebrations</h3>
                                    <p className="text-xs text-muted-foreground/60 mt-0.5">Birthdays &amp; Anniversaries</p>
                                </div>
                                <div className="px-5 pb-5 space-y-2.5 max-h-72 overflow-y-auto">
                                    {data.operations.celebrations.length > 0 ? (
                                        data.operations.celebrations.map((celebration, index) => (
                                            <div key={index} className="flex items-center justify-between rounded-xl border border-blue-100/60 dark:border-white/8 bg-blue-50/30 dark:bg-white/3 p-3 hover:border-blue-300/60 dark:hover:border-blue-600/40 transition-colors">
                                                <div className="flex items-center gap-3">
                                                    <div className={`flex h-8 w-8 shrink-0 items-center justify-center rounded-lg shadow-sm ${celebration.type === 'birthday' ? 'bg-gradient-to-br from-orange-400 to-orange-500' : 'bg-gradient-to-br from-pink-400 to-rose-500'}`}>
                                                        {celebration.type === 'birthday' ? (
                                                            <Cake className="h-4 w-4 text-white" />
                                                        ) : (
                                                            <Gift className="h-4 w-4 text-white" />
                                                        )}
                                                    </div>
                                                    <div>
                                                        <p className="text-sm font-semibold text-foreground">{celebration.name}</p>
                                                        <p className="text-xs text-muted-foreground/60">{celebration.date}</p>
                                                    </div>
                                                </div>
                                                <span className={`inline-flex items-center h-6 px-2 rounded-full text-[11px] font-semibold capitalize border ${celebration.type === 'birthday' ? 'bg-orange-50 dark:bg-orange-900/30 text-orange-600 dark:text-orange-300 border-orange-200/60 dark:border-orange-700/30' : 'bg-pink-50 dark:bg-pink-900/30 text-pink-600 dark:text-pink-300 border-pink-200/60 dark:border-pink-700/30'}`}>
                                                    {celebration.type}
                                                </span>
                                            </div>
                                        ))
                                    ) : (
                                        <p className="text-sm text-muted-foreground/50 py-4 text-center">No celebrations coming up</p>
                                    )}
                                </div>
                            </GlassCard>

                            {/* Recent Workflows */}
                            <GlassCard>
                                <div className="px-5 pt-5 pb-3">
                                    <h3 className="font-semibold text-sm text-foreground">Recent Workflows</h3>
                                    <p className="text-xs text-muted-foreground/60 mt-0.5">Latest activities</p>
                                </div>
                                <div className="px-5 pb-5 space-y-2.5 max-h-72 overflow-y-auto">
                                    {data.operations.recentWorkflows.length > 0 ? (
                                        data.operations.recentWorkflows.map((workflow, index) => (
                                            <div key={index} className="rounded-xl border border-blue-100/60 dark:border-white/8 bg-blue-50/30 dark:bg-white/3 p-3 hover:border-blue-300/60 dark:hover:border-blue-600/40 transition-colors">
                                                <div className="flex items-start justify-between gap-2">
                                                    <div className="flex-1 min-w-0">
                                                        <p className="font-semibold text-sm text-foreground truncate">{workflow.process}</p>
                                                        <p className="text-xs text-muted-foreground/60 mt-0.5 truncate">{workflow.step}</p>
                                                        <p className="text-xs text-muted-foreground/40 mt-0.5">{workflow.timestamp}</p>
                                                    </div>
                                                    <span className={`inline-flex items-center h-6 px-2 rounded-full text-[11px] font-semibold capitalize border shrink-0 ${
                                                        workflow.status === 'completed'
                                                            ? 'bg-emerald-50 dark:bg-emerald-900/30 text-emerald-700 dark:text-emerald-300 border-emerald-200/60 dark:border-emerald-700/30'
                                                            : workflow.status === 'pending'
                                                            ? 'bg-amber-50 dark:bg-amber-900/30 text-amber-700 dark:text-amber-300 border-amber-200/60 dark:border-amber-700/30'
                                                            : 'bg-blue-50 dark:bg-blue-900/30 text-[#071b3a] dark:text-blue-300 border-blue-200/60 dark:border-blue-700/30'
                                                    }`}>
                                                        {workflow.status}
                                                    </span>
                                                </div>
                                            </div>
                                        ))
                                    ) : (
                                        <p className="text-sm text-muted-foreground/50 py-4 text-center">No recent workflows</p>
                                    )}
                                </div>
                            </GlassCard>
                        </div>
                    )}
                </div>

            </div>
        </AppLayout>
    );
}
