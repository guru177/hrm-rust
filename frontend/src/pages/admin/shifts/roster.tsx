import axios from '@/lib/axios';
import { ArrowLeftRight, Calendar, RefreshCw, Users } from 'lucide-react';
import { useEffect, useState } from 'react';
import { Link } from 'react-router-dom';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogFooter,
    DialogHeader,
    DialogTitle,
} from '@/components/ui/dialog';
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
    is_default?: boolean;
}

interface RosterEmployee {
    assignment_id: number | null;
    user_id: number;
    name: string;
    email?: string | null;
    employee_id?: string | null;
    shift_template_id: number | null;
    shift_name: string;
    effective_from: string | null;
    effective_to: string | null;
}

function formatTime(value?: string) {
    if (!value) return '';
    const part = value.slice(0, 5);
    const [h, m] = part.split(':').map(Number);
    if (Number.isNaN(h) || Number.isNaN(m)) return value;
    const d = new Date();
    d.setHours(h, m, 0, 0);
    return d.toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit', hour12: true });
}

export default function ShiftRosterPage() {
    const [templates, setTemplates] = useState<ShiftTemplate[]>([]);
    const [selectedShiftId, setSelectedShiftId] = useState('0');
    const [asOfDate, setAsOfDate] = useState(new Date().toISOString().slice(0, 10));
    const [employees, setEmployees] = useState<RosterEmployee[]>([]);
    const [total, setTotal] = useState(0);
    const [loading, setLoading] = useState(true);

    const [changeOpen, setChangeOpen] = useState(false);
    const [changing, setChanging] = useState(false);
    const [changeUser, setChangeUser] = useState<RosterEmployee | null>(null);
    const [newShiftId, setNewShiftId] = useState('');
    const [effectiveFrom, setEffectiveFrom] = useState(new Date().toISOString().slice(0, 10));
    const [effectiveTo, setEffectiveTo] = useState('');

    useEffect(() => {
        void loadTemplates();
    }, []);

    useEffect(() => {
        void loadRoster();
    }, [selectedShiftId, asOfDate]);

    const loadTemplates = async () => {
        try {
            const res = await axios.get('/admin/shifts');
            const list: ShiftTemplate[] = res.data.data || [];
            setTemplates(list);
            if (list.length > 0 && selectedShiftId === '0') {
                const general = list.find((t) => t.is_default) ?? list.find((t) => t.name.toLowerCase() === 'general');
                setSelectedShiftId(String(general?.id ?? list[0].id));
            }
        } catch (error) {
            handleApiError(error);
        }
    };

    const loadRoster = async () => {
        setLoading(true);
        try {
            const res = await axios.get('/admin/shifts/roster', {
                params: {
                    shift_id: Number(selectedShiftId),
                    date: asOfDate,
                },
            });
            setEmployees(res.data.data?.employees || []);
            setTotal(res.data.data?.total || 0);
        } catch (error) {
            handleApiError(error);
            setEmployees([]);
            setTotal(0);
        } finally {
            setLoading(false);
        }
    };

    const openChangeDialog = (employee: RosterEmployee) => {
        setChangeUser(employee);
        setNewShiftId('');
        setEffectiveFrom(new Date().toISOString().slice(0, 10));
        setEffectiveTo('');
        setChangeOpen(true);
    };

    const submitChangeShift = async () => {
        if (!changeUser || !newShiftId || !effectiveFrom) return;
        setChanging(true);
        try {
            const response = await axios.post('/admin/shifts/assign-user', {
                user_id: changeUser.user_id,
                shift_template_id: Number(newShiftId),
                effective_from: effectiveFrom,
                effective_to: effectiveTo || null,
            });
            handleApiResponse(response);
            setChangeOpen(false);
            await loadRoster();
        } catch (error) {
            handleApiError(error);
        } finally {
            setChanging(false);
        }
    };

    const selectedTemplate = templates.find((t) => String(t.id) === selectedShiftId);
    const breadcrumbs = [
        { label: 'Shifts', href: '/admin/shifts' },
        { label: 'Roster', href: '/admin/shifts/roster' },
    ];

    return (
        <AppLayout breadcrumbs={breadcrumbs}>
            <div className="space-y-6">
                <div className="relative overflow-hidden rounded-2xl bg-gradient-to-br from-[#e8f2fd] via-[#d0e4f8] to-[#c4d8f0] dark:from-[#0d1e33] dark:via-[#0a1828] dark:to-[#071220] px-6 py-5 shadow-sm border border-white/60 dark:border-white/10">
                    <div className="relative flex flex-col sm:flex-row sm:items-center justify-between gap-4">
                        <div className="flex items-center gap-4">
                            <div className="flex h-12 w-12 shrink-0 items-center justify-center rounded-xl bg-[#071b3a]/15 dark:bg-white/10 border border-[#071b3a]/20 dark:border-white/10 shadow-inner">
                                <Users className="h-6 w-6 text-[#071b3a] dark:text-blue-300" />
                            </div>
                            <div>
                                <h1 className="text-xl font-bold tracking-tight text-[#001f3f] dark:text-white">
                                    Shift Roster
                                </h1>
                                <p className="text-sm text-[#1e3a5f]/60 dark:text-blue-200/60">
                                    See who is on each shift and change assignments
                                </p>
                            </div>
                        </div>
                        <div className="flex gap-2 shrink-0">
                            <Button variant="outline" asChild>
                                <Link to="/admin/shifts/daily">Daily Schedule</Link>
                            </Button>
                            <Button variant="outline" asChild>
                                <Link to="/admin/shifts">Manage Templates</Link>
                            </Button>
                        </div>
                    </div>
                </div>

                <Card>
                    <CardHeader>
                        <CardTitle>Filter by Shift</CardTitle>
                        <CardDescription>
                            Pick a shift to list all employees currently assigned to it
                        </CardDescription>
                    </CardHeader>
                    <CardContent>
                        <div className="flex flex-wrap gap-3 items-end">
                            <div className="space-y-2 min-w-[220px]">
                                <Label>Shift</Label>
                                <Select value={selectedShiftId} onValueChange={setSelectedShiftId}>
                                    <SelectTrigger>
                                        <SelectValue placeholder="Select shift" />
                                    </SelectTrigger>
                                    <SelectContent>
                                        {templates.map((t) => (
                                            <SelectItem key={t.id} value={String(t.id)}>
                                                {t.name} ({formatTime(t.start_time)} – {formatTime(t.end_time)})
                                            </SelectItem>
                                        ))}
                                        <SelectItem value="0">
                                            Unassigned (not on General shift)
                                        </SelectItem>
                                    </SelectContent>
                                </Select>
                            </div>
                            <div className="space-y-2">
                                <Label>As of date</Label>
                                <Input
                                    type="date"
                                    value={asOfDate}
                                    onChange={(e) => setAsOfDate(e.target.value)}
                                    className="w-[180px]"
                                />
                            </div>
                            <Button variant="outline" size="icon" onClick={loadRoster} disabled={loading}>
                                <RefreshCw className={`h-4 w-4 ${loading ? 'animate-spin' : ''}`} />
                            </Button>
                        </div>

                        {selectedTemplate && (
                            <div className="mt-4 flex flex-wrap items-center gap-2 text-sm">
                                <Badge variant="outline">{selectedTemplate.name}</Badge>
                                <span className="text-muted-foreground">
                                    {formatTime(selectedTemplate.start_time)} – {formatTime(selectedTemplate.end_time)}
                                </span>
                                <Badge>{total} employee{total !== 1 ? 's' : ''}</Badge>
                            </div>
                        )}
                        {selectedShiftId === '0' && (
                            <div className="mt-4">
                                <Badge variant="secondary">{total} without shift assignment</Badge>
                            </div>
                        )}
                    </CardContent>
                </Card>

                <Card>
                    <CardHeader>
                        <CardTitle>Employees in this shift</CardTitle>
                        <CardDescription>
                            Use <strong>Change Shift</strong> to move an employee to a different template
                        </CardDescription>
                    </CardHeader>
                    <CardContent>
                        <div className="rounded-md border overflow-x-auto">
                            <Table>
                                <TableHeader>
                                    <TableRow>
                                        <TableHead>Employee</TableHead>
                                        <TableHead>Employee ID</TableHead>
                                        <TableHead>Email</TableHead>
                                        <TableHead>Effective From</TableHead>
                                        <TableHead>Effective To</TableHead>
                                        <TableHead className="w-[140px]">Actions</TableHead>
                                    </TableRow>
                                </TableHeader>
                                <TableBody>
                                    {loading ? (
                                        <TableRow>
                                            <TableCell colSpan={6} className="text-center py-8">
                                                <div className="h-8 w-8 mx-auto animate-spin rounded-full border-4 border-primary border-t-transparent" />
                                            </TableCell>
                                        </TableRow>
                                    ) : employees.length === 0 ? (
                                        <TableRow>
                                            <TableCell colSpan={6} className="text-center py-8 text-muted-foreground">
                                                No employees in this shift for the selected date
                                            </TableCell>
                                        </TableRow>
                                    ) : (
                                        employees.map((emp) => (
                                            <TableRow key={emp.user_id}>
                                                <TableCell className="font-medium">{emp.name}</TableCell>
                                                <TableCell className="font-mono text-sm">
                                                    {emp.employee_id || '—'}
                                                </TableCell>
                                                <TableCell>{emp.email || '—'}</TableCell>
                                                <TableCell>
                                                    {emp.effective_from ? (
                                                        <span className="flex items-center gap-1.5">
                                                            <Calendar className="h-3.5 w-3.5 text-muted-foreground" />
                                                            {emp.effective_from}
                                                        </span>
                                                    ) : (
                                                        '—'
                                                    )}
                                                </TableCell>
                                                <TableCell>{emp.effective_to || 'Ongoing'}</TableCell>
                                                <TableCell>
                                                    <Button
                                                        variant="outline"
                                                        size="sm"
                                                        onClick={() => openChangeDialog(emp)}
                                                    >
                                                        <ArrowLeftRight className="mr-1.5 h-3.5 w-3.5" />
                                                        Change Shift
                                                    </Button>
                                                </TableCell>
                                            </TableRow>
                                        ))
                                    )}
                                </TableBody>
                            </Table>
                        </div>
                    </CardContent>
                </Card>
            </div>

            <Dialog open={changeOpen} onOpenChange={setChangeOpen}>
                <DialogContent>
                    <DialogHeader>
                        <DialogTitle>Change Shift</DialogTitle>
                        <DialogDescription>
                            Assign a new shift to <strong>{changeUser?.name}</strong>. The previous assignment
                            will end on the new effective date.
                        </DialogDescription>
                    </DialogHeader>
                    <div className="space-y-4">
                        <div className="rounded-lg border bg-muted/30 p-3 text-sm">
                            <p>
                                <span className="text-muted-foreground">Current:</span>{' '}
                                {changeUser?.shift_name}
                            </p>
                        </div>
                        <div className="space-y-2">
                            <Label>New Shift Template</Label>
                            <Select value={newShiftId} onValueChange={setNewShiftId}>
                                <SelectTrigger>
                                    <SelectValue placeholder="Select new shift" />
                                </SelectTrigger>
                                <SelectContent>
                                    {templates.filter((t) => t.is_active).map((t) => (
                                        <SelectItem key={t.id} value={String(t.id)}>
                                            {t.name} ({formatTime(t.start_time)} – {formatTime(t.end_time)})
                                        </SelectItem>
                                    ))}
                                </SelectContent>
                            </Select>
                        </div>
                        <div className="grid grid-cols-2 gap-4">
                            <div className="space-y-2">
                                <Label>Effective From</Label>
                                <Input
                                    type="date"
                                    value={effectiveFrom}
                                    onChange={(e) => setEffectiveFrom(e.target.value)}
                                />
                            </div>
                            <div className="space-y-2">
                                <Label>Effective To (optional)</Label>
                                <Input
                                    type="date"
                                    value={effectiveTo}
                                    onChange={(e) => setEffectiveTo(e.target.value)}
                                />
                            </div>
                        </div>
                    </div>
                    <DialogFooter>
                        <Button variant="outline" onClick={() => setChangeOpen(false)}>
                            Cancel
                        </Button>
                        <Button
                            onClick={submitChangeShift}
                            disabled={changing || !newShiftId || !effectiveFrom}
                        >
                            {changing ? 'Saving...' : 'Save New Shift'}
                        </Button>
                    </DialogFooter>
                </DialogContent>
            </Dialog>
        </AppLayout>
    );
}
