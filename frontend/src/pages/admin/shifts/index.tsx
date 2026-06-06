import axios from '@/lib/axios';
import { Clock, Edit3, Plus, RefreshCw, Trash2, UserPlus } from 'lucide-react';
import { useEffect, useState } from 'react';
import { Link } from 'react-router-dom';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Checkbox } from '@/components/ui/checkbox';
import {
    Dialog,
    DialogContent,
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
import { Switch } from '@/components/ui/switch';
import {
    Table,
    TableBody,
    TableCell,
    TableHead,
    TableHeader,
    TableRow,
} from '@/components/ui/table';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import AppLayout from '@/layouts/app-layout';
import { handleApiError, handleApiResponse, showToast } from '@/lib/toast';

interface ShiftTemplate {
    id: number;
    name: string;
    start_time: string;
    end_time: string;
    grace_in_minutes: number;
    grace_out_minutes: number;
    is_active: boolean;
    is_default?: boolean;
    assigned_count?: number;
    working_days?: string[];
    working_days_label?: string;
}

interface ShiftTemplateForm {
    id?: number;
    name: string;
    start_time: string;
    end_time: string;
    grace_in_minutes: number;
    grace_out_minutes: number;
    is_active: boolean;
    is_default: boolean;
    working_days: string[];
}

interface UserOption {
    id: number;
    name: string;
    email?: string;
}

interface UserAssignment {
    id: number;
    user_id: number;
    shift_template_id: number;
    effective_from: string;
    effective_to: string | null;
    template: {
        name: string;
        start_time: string;
        end_time: string;
        grace_in_minutes: number;
        grace_out_minutes: number;
        working_days?: string[];
        working_days_label?: string;
    };
}

const WEEKDAYS = [
    { key: 'mon', label: 'Mon' },
    { key: 'tue', label: 'Tue' },
    { key: 'wed', label: 'Wed' },
    { key: 'thu', label: 'Thu' },
    { key: 'fri', label: 'Fri' },
    { key: 'sat', label: 'Sat' },
    { key: 'sun', label: 'Sun' },
] as const;

const DEFAULT_WORKING_DAYS = ['mon', 'tue', 'wed', 'thu', 'fri'];

const defaultForm: ShiftTemplateForm = {
    name: '',
    start_time: '09:00',
    end_time: '18:00',
    grace_in_minutes: 10,
    grace_out_minutes: 5,
    is_active: true,
    is_default: false,
    working_days: [...DEFAULT_WORKING_DAYS],
};

function formatTime(value: string) {
    const part = value?.slice(0, 5) || value;
    const [h, m] = part.split(':').map(Number);
    if (Number.isNaN(h) || Number.isNaN(m)) return value;
    const d = new Date();
    d.setHours(h, m, 0, 0);
    return d.toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit', hour12: true });
}

function toApiTime(value: string) {
    return value.length === 5 ? `${value}:00` : value;
}

export default function ShiftsPage() {
    const [templates, setTemplates] = useState<ShiftTemplate[]>([]);
    const [users, setUsers] = useState<UserOption[]>([]);
    const [loading, setLoading] = useState(true);
    const [saving, setSaving] = useState(false);
    const [dialogOpen, setDialogOpen] = useState(false);
    const [form, setForm] = useState<ShiftTemplateForm>(defaultForm);

    const [assignUserId, setAssignUserId] = useState('');
    const [assignShiftId, setAssignShiftId] = useState('');
    const [assignFrom, setAssignFrom] = useState(new Date().toISOString().slice(0, 10));
    const [assignTo, setAssignTo] = useState('');
    const [assigning, setAssigning] = useState(false);
    const [userAssignment, setUserAssignment] = useState<UserAssignment | null>(null);
    const [loadingAssignment, setLoadingAssignment] = useState(false);

    useEffect(() => {
        void loadData();
    }, []);

    useEffect(() => {
        if (assignUserId) {
            void loadUserAssignment(Number(assignUserId));
        } else {
            setUserAssignment(null);
        }
    }, [assignUserId]);

    const loadData = async () => {
        setLoading(true);
        try {
            const [shiftsRes, usersRes] = await Promise.all([
                axios.get('/admin/shifts'),
                axios.get('/admin/users/list'),
            ]);
            setTemplates(shiftsRes.data.data || []);
            setUsers(usersRes.data.data || []);
        } catch (error) {
            handleApiError(error);
        } finally {
            setLoading(false);
        }
    };

    const loadUserAssignment = async (userId: number) => {
        setLoadingAssignment(true);
        try {
            const res = await axios.get(`/admin/shifts/user/${userId}`);
            setUserAssignment(res.data.data?.assignment || null);
        } catch (error) {
            handleApiError(error);
            setUserAssignment(null);
        } finally {
            setLoadingAssignment(false);
        }
    };

    const openCreate = () => {
        setForm(defaultForm);
        setDialogOpen(true);
    };

    const openEdit = (template: ShiftTemplate) => {
        setForm({
            id: template.id,
            name: template.name,
            start_time: template.start_time.slice(0, 5),
            end_time: template.end_time.slice(0, 5),
            grace_in_minutes: template.grace_in_minutes,
            grace_out_minutes: template.grace_out_minutes,
            is_active: template.is_active,
            is_default: template.is_default ?? false,
            working_days: template.working_days?.length
                ? [...template.working_days]
                : [...DEFAULT_WORKING_DAYS],
        });
        setDialogOpen(true);
    };

    const deleteTemplate = async (template: ShiftTemplate) => {
        const count = template.assigned_count ?? 0;
        if (count > 0) {
            showToast({
                type: 'warning',
                message: `Cannot delete "${template.name}": ${count} employee(s) assigned. Reassign them in Shift Roster first.`,
            });
            return;
        }
        if (
            !confirm(
                `Delete shift "${template.name}"?\n\nThis cannot be undone.`,
            )
        ) {
            return;
        }
        try {
            const response = await axios.delete(`/admin/shifts/${template.id}`);
            handleApiResponse(response);
            await loadData();
        } catch (error) {
            handleApiError(error);
        }
    };

    const saveTemplate = async () => {
        setSaving(true);
        try {
            const payload = {
                name: form.name.trim(),
                start_time: toApiTime(form.start_time),
                end_time: toApiTime(form.end_time),
                grace_in_minutes: form.grace_in_minutes,
                grace_out_minutes: form.grace_out_minutes,
                is_active: form.is_active,
                is_default: form.is_default,
                working_days: form.working_days,
            };
            const response = form.id
                ? await axios.put(`/admin/shifts/${form.id}`, payload)
                : await axios.post('/admin/shifts', payload);
            handleApiResponse(response);
            setDialogOpen(false);
            await loadData();
        } catch (error) {
            handleApiError(error);
        } finally {
            setSaving(false);
        }
    };

    const assignShift = async () => {
        if (!assignUserId || !assignShiftId || !assignFrom) return;
        setAssigning(true);
        try {
            const response = await axios.post('/admin/shifts/assign-user', {
                user_id: Number(assignUserId),
                shift_template_id: Number(assignShiftId),
                effective_from: assignFrom,
                effective_to: assignTo || null,
            });
            handleApiResponse(response);
            await loadUserAssignment(Number(assignUserId));
        } catch (error) {
            handleApiError(error);
        } finally {
            setAssigning(false);
        }
    };

    const breadcrumbs = [{ label: 'Shifts', href: '/admin/shifts' }];

    return (
        <AppLayout breadcrumbs={breadcrumbs}>
            <div className="space-y-6">
                <div className="relative overflow-hidden rounded-2xl bg-gradient-to-br from-[#e8f2fd] via-[#d0e4f8] to-[#c4d8f0] dark:from-[#0d1e33] dark:via-[#0a1828] dark:to-[#071220] px-6 py-5 shadow-sm border border-white/60 dark:border-white/10">
                    <div className="relative flex flex-col sm:flex-row sm:items-center justify-between gap-4">
                        <div className="flex items-center gap-4">
                            <div className="flex h-12 w-12 shrink-0 items-center justify-center rounded-xl bg-[#071b3a]/15 dark:bg-white/10 border border-[#071b3a]/20 dark:border-white/10 shadow-inner">
                                <Clock className="h-6 w-6 text-[#071b3a] dark:text-blue-300" />
                            </div>
                            <div>
                                <h1 className="text-xl font-bold tracking-tight text-[#001f3f] dark:text-white">
                                    Shift Management
                                </h1>
                                <p className="text-sm text-[#1e3a5f]/60 dark:text-blue-200/60">
                                    Define work times, weekly working days, and assign shifts to employees
                                </p>
                            </div>
                        </div>
                        <div className="flex gap-2 shrink-0">
                            <Button variant="outline" asChild>
                                <Link to="/admin/shifts/daily">Daily Schedule</Link>
                            </Button>
                            <Button variant="outline" asChild>
                                <Link to="/admin/shifts/roster">View Roster</Link>
                            </Button>
                            <Button
                                onClick={openCreate}
                                className="bg-gradient-to-r from-[#071b3a] to-[#0d4a8a] hover:from-[#040f22] hover:to-[#0a3272] text-white shadow-md rounded-xl gap-2"
                            >
                                <Plus className="h-4 w-4" />
                                New Shift Template
                            </Button>
                        </div>
                    </div>
                </div>

                <Tabs defaultValue="templates" className="w-full">
                    <TabsList className="grid w-full max-w-md grid-cols-2">
                        <TabsTrigger value="templates">Shift Templates</TabsTrigger>
                        <TabsTrigger value="assign">Assign to Users</TabsTrigger>
                    </TabsList>

                    <TabsContent value="templates" className="space-y-4">
                        <Card>
                            <CardHeader className="flex flex-row items-center justify-between">
                                <div>
                                    <CardTitle>Templates</CardTitle>
                                    <CardDescription>
                                        Times, grace periods, and which days count as working days for payroll &amp; LOP
                                    </CardDescription>
                                </div>
                                <Button variant="outline" size="icon" onClick={loadData} disabled={loading}>
                                    <RefreshCw className={`h-4 w-4 ${loading ? 'animate-spin' : ''}`} />
                                </Button>
                            </CardHeader>
                            <CardContent>
                                <div className="rounded-md border overflow-x-auto">
                                    <Table>
                                        <TableHeader>
                                            <TableRow>
                                                <TableHead>Name</TableHead>
                                                <TableHead>Start</TableHead>
                                                <TableHead>End</TableHead>
                                                <TableHead>Grace In</TableHead>
                                                <TableHead>Grace Out</TableHead>
                                                <TableHead>Working Days</TableHead>
                                                <TableHead>Employees</TableHead>
                                                <TableHead>Status</TableHead>
                                                <TableHead className="w-[100px]">Actions</TableHead>
                                            </TableRow>
                                        </TableHeader>
                                        <TableBody>
                                            {loading ? (
                                                <TableRow>
                                                    <TableCell colSpan={9} className="text-center py-8">
                                                        <div className="h-8 w-8 mx-auto animate-spin rounded-full border-4 border-primary border-t-transparent" />
                                                    </TableCell>
                                                </TableRow>
                                            ) : templates.length === 0 ? (
                                                <TableRow>
                                                    <TableCell colSpan={9} className="text-center py-8 text-muted-foreground">
                                                        No shift templates yet. Create one to get started.
                                                    </TableCell>
                                                </TableRow>
                                            ) : (
                                                templates.map((t) => (
                                                    <TableRow key={t.id}>
                                                        <TableCell className="font-medium">
                                                            <span className="inline-flex items-center gap-2">
                                                                {t.name}
                                                                {t.is_default && (
                                                                    <Badge variant="outline" className="text-xs">
                                                                        Default
                                                                    </Badge>
                                                                )}
                                                            </span>
                                                        </TableCell>
                                                        <TableCell>{formatTime(t.start_time)}</TableCell>
                                                        <TableCell>{formatTime(t.end_time)}</TableCell>
                                                        <TableCell>{t.grace_in_minutes} min</TableCell>
                                                        <TableCell>{t.grace_out_minutes} min</TableCell>
                                                        <TableCell className="text-sm text-muted-foreground max-w-[140px]">
                                                            {t.working_days_label ?? 'Mon–Fri'}
                                                        </TableCell>
                                                        <TableCell>{t.assigned_count ?? 0}</TableCell>
                                                        <TableCell>
                                                            <Badge variant={t.is_active ? 'default' : 'secondary'}>
                                                                {t.is_active ? 'Active' : 'Inactive'}
                                                            </Badge>
                                                        </TableCell>
                                                        <TableCell>
                                                            <div className="flex gap-1">
                                                                <Button variant="outline" size="icon" onClick={() => openEdit(t)} title="Edit shift">
                                                                    <Edit3 className="h-4 w-4" />
                                                                </Button>
                                                                <Button
                                                                    variant="outline"
                                                                    size="icon"
                                                                    className="text-destructive hover:text-destructive"
                                                                    onClick={() => deleteTemplate(t)}
                                                                    title={
                                                                        (t.assigned_count ?? 0) > 0
                                                                            ? `${t.assigned_count} employee(s) assigned — reassign first`
                                                                            : 'Delete shift'
                                                                    }
                                                                    disabled={(t.assigned_count ?? 0) > 0}
                                                                >
                                                                    <Trash2 className="h-4 w-4" />
                                                                </Button>
                                                            </div>
                                                        </TableCell>
                                                    </TableRow>
                                                ))
                                            )}
                                        </TableBody>
                                    </Table>
                                </div>
                            </CardContent>
                        </Card>
                    </TabsContent>

                    <TabsContent value="assign" className="space-y-4">
                        <Card>
                            <CardHeader>
                                <CardTitle className="flex items-center gap-2">
                                    <UserPlus className="h-5 w-5" />
                                    Assign Shift to Employee
                                </CardTitle>
                                <CardDescription>
                                    Assign a shift template with an effective date range
                                </CardDescription>
                            </CardHeader>
                            <CardContent className="space-y-4">
                                <div className="grid gap-4 sm:grid-cols-2">
                                    <div className="space-y-2">
                                        <Label>Employee</Label>
                                        <Select value={assignUserId} onValueChange={setAssignUserId}>
                                            <SelectTrigger>
                                                <SelectValue placeholder="Select employee" />
                                            </SelectTrigger>
                                            <SelectContent>
                                                {users.map((u) => (
                                                    <SelectItem key={u.id} value={String(u.id)}>
                                                        {u.name}{u.email ? ` (${u.email})` : ''}
                                                    </SelectItem>
                                                ))}
                                            </SelectContent>
                                        </Select>
                                    </div>
                                    <div className="space-y-2">
                                        <Label>Shift Template</Label>
                                        <Select value={assignShiftId} onValueChange={setAssignShiftId}>
                                            <SelectTrigger>
                                                <SelectValue placeholder="Select shift" />
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
                                    <div className="space-y-2">
                                        <Label>Effective From</Label>
                                        <Input
                                            type="date"
                                            value={assignFrom}
                                            onChange={(e) => setAssignFrom(e.target.value)}
                                        />
                                    </div>
                                    <div className="space-y-2">
                                        <Label>Effective To (optional)</Label>
                                        <Input
                                            type="date"
                                            value={assignTo}
                                            onChange={(e) => setAssignTo(e.target.value)}
                                        />
                                    </div>
                                </div>

                                <Button
                                    onClick={assignShift}
                                    disabled={assigning || !assignUserId || !assignShiftId || !assignFrom}
                                >
                                    {assigning ? 'Assigning...' : 'Assign Shift'}
                                </Button>

                                {assignUserId && (
                                    <div className="rounded-lg border bg-muted/30 p-4 space-y-2">
                                        <p className="text-sm font-medium">Current Assignment</p>
                                        {loadingAssignment ? (
                                            <p className="text-sm text-muted-foreground">Loading...</p>
                                        ) : userAssignment ? (
                                            <div className="text-sm space-y-1">
                                                <p>
                                                    <span className="text-muted-foreground">Shift:</span>{' '}
                                                    <span className="font-medium">{userAssignment.template.name}</span>
                                                </p>
                                                <p>
                                                    <span className="text-muted-foreground">Working days:</span>{' '}
                                                    {userAssignment.template.working_days_label
                                                        ?? userAssignment.template.working_days?.join(', ')
                                                        ?? 'Mon–Fri'}
                                                </p>
                                                <p>
                                                    <span className="text-muted-foreground">Hours:</span>{' '}
                                                    {formatTime(userAssignment.template.start_time)} – {formatTime(userAssignment.template.end_time)}
                                                </p>
                                                <p>
                                                    <span className="text-muted-foreground">Effective:</span>{' '}
                                                    {userAssignment.effective_from}
                                                    {userAssignment.effective_to ? ` to ${userAssignment.effective_to}` : ' onwards'}
                                                </p>
                                            </div>
                                        ) : (
                                            <p className="text-sm text-muted-foreground">
                                                No shift assigned — General shift will be applied automatically
                                            </p>
                                        )}
                                    </div>
                                )}
                            </CardContent>
                        </Card>
                    </TabsContent>
                </Tabs>
            </div>

            <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
                <DialogContent>
                    <DialogHeader>
                        <DialogTitle>{form.id ? 'Edit Shift Template' : 'New Shift Template'}</DialogTitle>
                    </DialogHeader>
                    <div className="space-y-4">
                        <div className="space-y-2">
                            <Label>Name</Label>
                            <Input
                                value={form.name}
                                onChange={(e) => setForm({ ...form, name: e.target.value })}
                                placeholder="e.g. General 9-6"
                            />
                        </div>
                        <div className="grid grid-cols-2 gap-4">
                            <div className="space-y-2">
                                <Label>Start Time</Label>
                                <Input
                                    type="time"
                                    value={form.start_time}
                                    onChange={(e) => setForm({ ...form, start_time: e.target.value })}
                                />
                            </div>
                            <div className="space-y-2">
                                <Label>End Time</Label>
                                <Input
                                    type="time"
                                    value={form.end_time}
                                    onChange={(e) => setForm({ ...form, end_time: e.target.value })}
                                />
                            </div>
                        </div>
                        <div className="grid grid-cols-2 gap-4">
                            <div className="space-y-2">
                                <Label>Grace In (minutes)</Label>
                                <Input
                                    type="number"
                                    min={0}
                                    value={form.grace_in_minutes}
                                    onChange={(e) => setForm({ ...form, grace_in_minutes: Number(e.target.value) })}
                                />
                            </div>
                            <div className="space-y-2">
                                <Label>Grace Out (minutes)</Label>
                                <Input
                                    type="number"
                                    min={0}
                                    value={form.grace_out_minutes}
                                    onChange={(e) => setForm({ ...form, grace_out_minutes: Number(e.target.value) })}
                                />
                            </div>
                        </div>
                        <div className="space-y-2">
                            <Label>Working days</Label>
                            <p className="text-xs text-muted-foreground">
                                Checked days are scheduled work days. Unchecked days are weekly off (no LOP).
                                Assign different shifts to employees for different week-offs.
                            </p>
                            <div className="flex flex-wrap gap-2">
                                {WEEKDAYS.map(({ key, label }) => {
                                    const checked = form.working_days.includes(key);
                                    return (
                                        <label
                                            key={key}
                                            className={`flex cursor-pointer items-center gap-1.5 rounded-md border px-2.5 py-1.5 text-sm ${
                                                checked ? 'border-primary bg-primary/10' : 'border-muted'
                                            }`}
                                        >
                                            <Checkbox
                                                checked={checked}
                                                onCheckedChange={(v) => {
                                                    setForm((prev) => {
                                                        const next = v
                                                            ? [...prev.working_days, key]
                                                            : prev.working_days.filter((d) => d !== key);
                                                        return {
                                                            ...prev,
                                                            working_days: next.length > 0 ? next : [...DEFAULT_WORKING_DAYS],
                                                        };
                                                    });
                                                }}
                                            />
                                            {label}
                                        </label>
                                    );
                                })}
                            </div>
                        </div>
                        <div className="flex items-center justify-between rounded-lg border p-3">
                            <div>
                                <p className="text-sm font-medium">Active</p>
                                <p className="text-xs text-muted-foreground">Inactive shifts cannot be newly assigned</p>
                            </div>
                            <Switch
                                checked={form.is_active}
                                onCheckedChange={(checked) => setForm({ ...form, is_active: checked })}
                            />
                        </div>
                        <div className="flex items-center justify-between rounded-lg border p-3">
                            <div>
                                <p className="text-sm font-medium">Default shift</p>
                                <p className="text-xs text-muted-foreground">
                                    Auto-assigned to employees without a shift. You can rename times and grace freely.
                                </p>
                            </div>
                            <Switch
                                checked={form.is_default}
                                onCheckedChange={(checked) => setForm({ ...form, is_default: checked })}
                            />
                        </div>
                    </div>
                    <DialogFooter>
                        <Button variant="outline" onClick={() => setDialogOpen(false)}>
                            Cancel
                        </Button>
                        <Button onClick={saveTemplate} disabled={saving || !form.name.trim()}>
                            {saving ? 'Saving...' : 'Save'}
                        </Button>
                    </DialogFooter>
                </DialogContent>
            </Dialog>
        </AppLayout>
    );
}
