import axios from '@/lib/axios';
import { CalendarDays, ChevronLeft, ChevronRight, RefreshCw, Save } from 'lucide-react';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { Link } from 'react-router-dom';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
    Select,
    SelectContent,
    SelectItem,
    SelectTrigger,
    SelectValue,
} from '@/components/ui/select';
import {
    Table,
    TableBody,
    TableCell,
    TableHead,
    TableHeader,
    TableRow,
} from '@/components/ui/table';
import AppLayout from '@/layouts/app-layout';
import { handleApiError, handleApiResponse } from '@/lib/toast';

interface ShiftTemplate {
    id: number;
    name: string;
    start_time: string;
    end_time: string;
    is_active: boolean;
}

interface DayCell {
    is_daily_override: boolean;
    is_day_off: boolean;
    shift_template_id: number | null;
    shift_name?: string | null;
    schedule_source?: string;
}

interface RosterEmployee {
    user_id: number;
    name: string;
    employee_id?: string | null;
    days: Record<string, DayCell>;
}

const DEFAULT_VALUE = '__default__';
const OFF_VALUE = '__off__';

function addDays(iso: string, n: number): string {
    const d = new Date(iso + 'T12:00:00');
    d.setDate(d.getDate() + n);
    return d.toISOString().slice(0, 10);
}

function formatDayHeader(iso: string) {
    const d = new Date(iso + 'T12:00:00');
    return d.toLocaleDateString('en-IN', { weekday: 'short', day: 'numeric', month: 'short' });
}

function cellToSelectValue(cell: DayCell | undefined): string {
    if (!cell) return DEFAULT_VALUE;
    if (cell.is_daily_override && cell.is_day_off) return OFF_VALUE;
    if (cell.is_daily_override && cell.shift_template_id) return String(cell.shift_template_id);
    return DEFAULT_VALUE;
}

function shortShiftName(name?: string | null, max = 12) {
    if (!name) return '—';
    return name.length > max ? `${name.slice(0, max)}…` : name;
}

export default function DailyShiftSchedulePage() {
    const [weekStart, setWeekStart] = useState(() => {
        const d = new Date();
        const day = d.getDay();
        const diff = day === 0 ? -6 : 1 - day;
        d.setDate(d.getDate() + diff);
        return d.toISOString().slice(0, 10);
    });
    const [templates, setTemplates] = useState<ShiftTemplate[]>([]);
    const [dates, setDates] = useState<string[]>([]);
    const [employees, setEmployees] = useState<RosterEmployee[]>([]);
    const [loading, setLoading] = useState(true);
    const [saving, setSaving] = useState(false);
    const [pending, setPending] = useState<
        Record<string, { user_id: number; roster_date: string; shift_template_id: number | null; is_day_off: boolean }>
    >({});

    const weekEnd = useMemo(() => (dates.length ? dates[dates.length - 1] : addDays(weekStart, 6)), [dates, weekStart]);

    const loadTemplates = useCallback(async () => {
        const res = await axios.get('/admin/shifts');
        setTemplates((res.data.data || []).filter((t: ShiftTemplate) => t.is_active));
    }, []);

    const loadSchedule = useCallback(async () => {
        setLoading(true);
        try {
            const res = await axios.get('/admin/shifts/daily-roster', {
                params: { week_start: weekStart },
            });
            setDates(res.data.data?.dates || []);
            setEmployees(res.data.data?.employees || []);
            setPending({});
        } catch (error) {
            handleApiError(error);
            setEmployees([]);
            setDates([]);
        } finally {
            setLoading(false);
        }
    }, [weekStart]);

    useEffect(() => {
        void loadTemplates();
    }, [loadTemplates]);

    useEffect(() => {
        void loadSchedule();
    }, [loadSchedule]);

    const setCell = (userId: number, date: string, value: string) => {
        const key = `${userId}:${date}`;
        if (value === DEFAULT_VALUE) {
            setPending((p) => {
                const next = { ...p };
                next[key] = { user_id: userId, roster_date: date, shift_template_id: null, is_day_off: false };
                return next;
            });
            setEmployees((prev) =>
                prev.map((emp) => {
                    if (emp.user_id !== userId) return emp;
                    const base = emp.days[date];
                    return {
                        ...emp,
                        days: {
                            ...emp.days,
                            [date]: {
                                ...base,
                                is_daily_override: true,
                                is_day_off: false,
                                shift_template_id: null,
                                shift_name: 'Default',
                            },
                        },
                    };
                }),
            );
            return;
        }
        if (value === OFF_VALUE) {
            setPending((p) => ({
                ...p,
                [key]: { user_id: userId, roster_date: date, shift_template_id: null, is_day_off: true },
            }));
            setEmployees((prev) =>
                prev.map((emp) => {
                    if (emp.user_id !== userId) return emp;
                    return {
                        ...emp,
                        days: {
                            ...emp.days,
                            [date]: {
                                is_daily_override: true,
                                is_day_off: true,
                                shift_template_id: null,
                                shift_name: 'Off',
                            },
                        },
                    };
                }),
            );
            return;
        }
        const shiftId = Number(value);
        const tmpl = templates.find((t) => t.id === shiftId);
        setPending((p) => ({
            ...p,
            [key]: { user_id: userId, roster_date: date, shift_template_id: shiftId, is_day_off: false },
        }));
        setEmployees((prev) =>
            prev.map((emp) => {
                if (emp.user_id !== userId) return emp;
                return {
                    ...emp,
                    days: {
                        ...emp.days,
                        [date]: {
                            is_daily_override: true,
                            is_day_off: false,
                            shift_template_id: shiftId,
                            shift_name: tmpl?.name ?? 'Shift',
                        },
                    },
                };
            }),
        );
    };

    const saveChanges = async () => {
        const entries = Object.values(pending);
        if (entries.length === 0) return;
        setSaving(true);
        try {
            const res = await axios.post('/admin/shifts/daily-roster', { entries });
            handleApiResponse(res);
            await loadSchedule();
        } catch (error) {
            handleApiError(error);
        } finally {
            setSaving(false);
        }
    };

    const pendingCount = Object.keys(pending).length;
    const breadcrumbs = [
        { label: 'Shifts', href: '/admin/shifts' },
        { label: 'Daily Schedule', href: '/admin/shifts/daily' },
    ];

    return (
        <AppLayout breadcrumbs={breadcrumbs}>
            <div className="space-y-6">
                <div className="relative overflow-hidden rounded-2xl bg-gradient-to-br from-[#e8f2fd] via-[#d0e4f8] to-[#c4d8f0] dark:from-[#0d1e33] dark:via-[#0a1828] dark:to-[#071220] px-6 py-5 shadow-sm border border-white/60 dark:border-white/10">
                    <div className="relative flex flex-col sm:flex-row sm:items-center justify-between gap-4">
                        <div className="flex items-center gap-4">
                            <div className="flex h-12 w-12 shrink-0 items-center justify-center rounded-xl bg-[#071b3a]/15 dark:bg-white/10 border border-[#071b3a]/20 dark:border-white/10 shadow-inner">
                                <CalendarDays className="h-6 w-6 text-[#071b3a] dark:text-blue-300" />
                            </div>
                            <div>
                                <h1 className="text-xl font-bold tracking-tight text-[#001f3f] dark:text-white">
                                    Daily Shift Schedule
                                </h1>
                                <p className="text-sm text-[#1e3a5f]/60 dark:text-blue-200/60">
                                    Set a different shift per employee per day — overrides the default assignment
                                </p>
                            </div>
                        </div>
                        <div className="flex gap-2 shrink-0">
                            <Button variant="outline" asChild>
                                <Link to="/admin/shifts">Templates</Link>
                            </Button>
                            <Button
                                onClick={saveChanges}
                                disabled={saving || pendingCount === 0}
                                className="gap-2"
                            >
                                <Save className="h-4 w-4" />
                                {saving ? 'Saving…' : `Save${pendingCount ? ` (${pendingCount})` : ''}`}
                            </Button>
                        </div>
                    </div>
                </div>

                <Card>
                    <CardHeader>
                        <CardTitle>Week</CardTitle>
                        <CardDescription>
                            <strong>Default</strong> = use the employee&apos;s assigned shift. Pick a template for a
                            one-day override, or <strong>Off</strong> for an extra weekly off.
                        </CardDescription>
                    </CardHeader>
                    <CardContent className="flex flex-wrap items-end gap-3">
                        <div className="space-y-2">
                            <Label>Week starting</Label>
                            <Input
                                type="date"
                                value={weekStart}
                                onChange={(e) => setWeekStart(e.target.value)}
                                className="w-[180px]"
                            />
                        </div>
                        <Button variant="outline" size="icon" onClick={() => setWeekStart(addDays(weekStart, -7))}>
                            <ChevronLeft className="h-4 w-4" />
                        </Button>
                        <Button variant="outline" size="icon" onClick={() => setWeekStart(addDays(weekStart, 7))}>
                            <ChevronRight className="h-4 w-4" />
                        </Button>
                        <Badge variant="secondary">
                            {weekStart} → {weekEnd}
                        </Badge>
                        <Button variant="outline" size="icon" onClick={loadSchedule} disabled={loading}>
                            <RefreshCw className={`h-4 w-4 ${loading ? 'animate-spin' : ''}`} />
                        </Button>
                    </CardContent>
                </Card>

                <Card>
                    <CardContent className="pt-6">
                        <div className="rounded-md border overflow-x-auto">
                            <Table>
                                <TableHeader>
                                    <TableRow>
                                        <TableHead className="min-w-[160px] sticky left-0 bg-background z-10">
                                            Employee
                                        </TableHead>
                                        {dates.map((date) => (
                                            <TableHead key={date} className="min-w-[130px] text-center">
                                                {formatDayHeader(date)}
                                            </TableHead>
                                        ))}
                                    </TableRow>
                                </TableHeader>
                                <TableBody>
                                    {loading ? (
                                        <TableRow>
                                            <TableCell colSpan={dates.length + 1} className="text-center py-10">
                                                <div className="h-8 w-8 mx-auto animate-spin rounded-full border-4 border-primary border-t-transparent" />
                                            </TableCell>
                                        </TableRow>
                                    ) : employees.length === 0 ? (
                                        <TableRow>
                                            <TableCell colSpan={dates.length + 1} className="text-center py-10 text-muted-foreground">
                                                No employees found
                                            </TableCell>
                                        </TableRow>
                                    ) : (
                                        employees.map((emp) => (
                                            <TableRow key={emp.user_id}>
                                                <TableCell className="sticky left-0 bg-background z-10 font-medium">
                                                    <div>{emp.name}</div>
                                                    {emp.employee_id && (
                                                        <div className="text-xs text-muted-foreground font-mono">
                                                            {emp.employee_id}
                                                        </div>
                                                    )}
                                                </TableCell>
                                                {dates.map((date) => {
                                                    const cell = emp.days[date];
                                                    const selectVal = cellToSelectValue(cell);
                                                    const isOverride = cell?.is_daily_override;
                                                    return (
                                                        <TableCell key={date} className="p-1">
                                                            <Select
                                                                value={selectVal}
                                                                onValueChange={(v) => setCell(emp.user_id, date, v)}
                                                            >
                                                                <SelectTrigger
                                                                    className={`h-9 text-xs ${isOverride ? 'border-primary/50 bg-primary/5' : ''}`}
                                                                >
                                                                    <SelectValue>
                                                                        {selectVal === OFF_VALUE
                                                                            ? 'Off'
                                                                            : selectVal === DEFAULT_VALUE
                                                                              ? shortShiftName(cell?.shift_name) || 'Default'
                                                                              : shortShiftName(cell?.shift_name)}
                                                                    </SelectValue>
                                                                </SelectTrigger>
                                                                <SelectContent>
                                                                    <SelectItem value={DEFAULT_VALUE}>
                                                                        Default ({shortShiftName(cell?.shift_name, 20)})
                                                                    </SelectItem>
                                                                    <SelectItem value={OFF_VALUE}>Off (weekly off)</SelectItem>
                                                                    {templates.map((t) => (
                                                                        <SelectItem key={t.id} value={String(t.id)}>
                                                                            {t.name}
                                                                        </SelectItem>
                                                                    ))}
                                                                </SelectContent>
                                                            </Select>
                                                        </TableCell>
                                                    );
                                                })}
                                            </TableRow>
                                        ))
                                    )}
                                </TableBody>
                            </Table>
                        </div>
                    </CardContent>
                </Card>
            </div>
        </AppLayout>
    );
}
