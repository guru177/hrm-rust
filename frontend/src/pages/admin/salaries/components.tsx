// Head removed - use document.title instead
import axios from '@/lib/axios';
import {
    DollarSign,
    Edit,
    Info,
    Minus,
    Plus,
    RefreshCw,
    Search,
    Trash2,
    Wallet,
    X,
} from 'lucide-react';
import { useEffect, useState } from 'react';

import {
    AlertDialog,
    AlertDialogAction,
    AlertDialogCancel,
    AlertDialogContent,
    AlertDialogDescription,
    AlertDialogFooter,
    AlertDialogHeader,
    AlertDialogTitle,
} from '@/components/ui/alert-dialog';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
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
import { Textarea } from '@/components/ui/textarea';
import AppLayout from '@/layouts/app-layout';
import { handleApiError, handleApiResponse } from '@/lib/toast';

type ComponentType = 'earning' | 'deduction' | 'reimbursement';

interface SalaryComponent {
    id: number;
    name: string;
    type: ComponentType;
    description: string | null;
    is_active: boolean;
    // earnings
    earning_type: string | null;
    name_in_payslip: string | null;
    calculation_type: 'flat_amount' | 'percentage_of_basic' | 'percentage_of_ctc' | 'percentage_of_gross' | null;
    amount: string | null;
    // deductions
    deduction_type: string | null;
    deduction_frequency: 'recurring' | 'one_time' | null;
    is_pre_tax: boolean;
    // reimbursements
    reimbursement_type: string | null;
    max_amount_per_month: string | null;
}

interface FormState {
    name: string;
    type: ComponentType;
    description: string;
    is_active: boolean;
    // earnings
    earning_type: string;
    name_in_payslip: string;
    calculation_type: 'flat_amount' | 'percentage_of_basic' | 'percentage_of_ctc' | 'percentage_of_gross';
    amount: string;
    // deductions
    deduction_type: string;
    deduction_frequency: 'recurring' | 'one_time';
    is_pre_tax: boolean;
    // reimbursements
    reimbursement_type: string;
    max_amount_per_month: string;
}

const defaultForm = (type: ComponentType): FormState => ({
    name: '',
    type,
    description: '',
    is_active: true,
    earning_type: '',
    name_in_payslip: '',
    calculation_type: 'flat_amount',
    amount: '0',
    deduction_type: '',
    deduction_frequency: 'recurring',
    is_pre_tax: true,
    reimbursement_type: '',
    max_amount_per_month: '0',
});

const formatInrAmount = (value: string | number | null | undefined) => {
    const amount = Number(value ?? 0);

    return new Intl.NumberFormat('en-IN', {
        style: 'currency',
        currency: 'INR',
        maximumFractionDigits: 0,
    }).format(Number.isFinite(amount) ? amount : 0);
};

const calcTypeLabel = (calc: string | null | undefined) => {
    if (calc === 'percentage_of_basic') return 'Percentage of Basic';
    if (calc === 'percentage_of_ctc') return 'Percentage of CTC';
    if (calc === 'percentage_of_gross') return 'Percentage of Gross';
    if (calc === 'flat_amount') return 'Flat Amount';
    return '-';
};

const formatComponentValue = (
    calc: string | null | undefined,
    amount: string | null | undefined,
) => {
    if (calc === 'percentage_of_basic' || calc === 'percentage_of_ctc' || calc === 'percentage_of_gross') return `${amount ?? 0}%`;
    return formatInrAmount(amount);
};

function CalculationTypeFields({
    calculationType,
    amount,
    onCalculationTypeChange,
    onAmountChange,
    showCtcOption = true,
    showGrossOption = false,
}: {
    calculationType: 'flat_amount' | 'percentage_of_basic' | 'percentage_of_ctc' | 'percentage_of_gross';
    amount: string;
    onCalculationTypeChange: (v: 'flat_amount' | 'percentage_of_basic' | 'percentage_of_ctc' | 'percentage_of_gross') => void;
    onAmountChange: (v: string) => void;
    showCtcOption?: boolean;
    showGrossOption?: boolean;
}) {
    return (
        <>
            <div className="space-y-1.5">
                <Label>Calculation Type *</Label>
                <div className="flex flex-col gap-2 pt-1">
                    <label className="flex items-center gap-2 cursor-pointer">
                        <input
                            type="radio"
                            checked={calculationType === 'flat_amount'}
                            onChange={() => onCalculationTypeChange('flat_amount')}
                            className="accent-primary"
                        />
                        <span className="text-sm">Flat Amount</span>
                    </label>
                    {showCtcOption && (
                        <label className="flex items-center gap-2 cursor-pointer">
                            <input
                                type="radio"
                                checked={calculationType === 'percentage_of_ctc'}
                                onChange={() => onCalculationTypeChange('percentage_of_ctc')}
                                className="accent-primary"
                            />
                            <span className="text-sm">Percentage of CTC</span>
                        </label>
                    )}
                    {showGrossOption && (
                        <label className="flex items-center gap-2 cursor-pointer">
                            <input
                                type="radio"
                                checked={calculationType === 'percentage_of_gross'}
                                onChange={() => onCalculationTypeChange('percentage_of_gross')}
                                className="accent-primary"
                            />
                            <span className="text-sm">Percentage of Gross</span>
                        </label>
                    )}
                    <label className="flex items-center gap-2 cursor-pointer">
                        <input
                            type="radio"
                            checked={calculationType === 'percentage_of_basic'}
                            onChange={() => onCalculationTypeChange('percentage_of_basic')}
                            className="accent-primary"
                        />
                        <span className="text-sm">Percentage of Basic</span>
                    </label>
                </div>
            </div>
            <div className="space-y-1.5">
                <Label>
                    {calculationType === 'flat_amount' ? 'Enter Amount (INR)' : 'Percentage (%)'}
                </Label>
                <div className="relative">
                    {calculationType === 'flat_amount' && (
                        <span className="absolute left-3 top-1/2 -translate-y-1/2 text-sm text-muted-foreground">INR</span>
                    )}
                    <Input
                        type="number"
                        min={0}
                        value={amount}
                        onChange={(e) => onAmountChange(e.target.value)}
                        className={calculationType === 'flat_amount' ? 'pl-7' : ''}
                    />
                </div>
            </div>
        </>
    );
}

export default function SalaryComponents() {
    const [activeTab, setActiveTab] = useState<ComponentType>('earning');
    const [components, setComponents] = useState<SalaryComponent[]>([]);
    const [loading, setLoading] = useState(false);
    const [search, setSearch] = useState('');

    const [showModal, setShowModal] = useState(false);
    const [editing, setEditing] = useState<SalaryComponent | null>(null);
    const [form, setForm] = useState<FormState>(defaultForm('earning'));
    const [saving, setSaving] = useState(false);

    const [deleteTarget, setDeleteTarget] = useState<SalaryComponent | null>(null);
    const [deleting, setDeleting] = useState(false);

    const breadcrumbs = [
        { title: 'Salaries', href: '/admin/salaries/components' },
        { title: 'Salary Components', href: '/admin/salaries/components' },
    ];

    const fetchComponents = async () => {
        setLoading(true);
        try {
            const response = await axios.get('/admin/salaries/components/list', {
                params: { type: activeTab, search: search || undefined },
            });
            if (response.data.success) setComponents(response.data.data);
        } catch (error) {
            handleApiError(error);
        } finally {
            setLoading(false);
        }
    };

    useEffect(() => {
        fetchComponents();
    }, [activeTab, search]);

    const openCreate = () => {
        setEditing(null);
        setForm(defaultForm(activeTab));
        setShowModal(true);
    };

    const openEdit = (c: SalaryComponent) => {
        setEditing(c);
        setForm({
            name: c.name,
            type: c.type,
            description: c.description ?? '',
            is_active: c.is_active,
            earning_type: c.earning_type ?? '',
            name_in_payslip: c.name_in_payslip ?? '',
            calculation_type:
                c.calculation_type === 'percentage_of_basic'
                    ? 'percentage_of_basic'
                    : c.calculation_type === 'percentage_of_ctc'
                      ? 'percentage_of_ctc'
                      : c.calculation_type === 'percentage_of_gross'
                        ? 'percentage_of_gross'
                        : 'flat_amount',
            amount: c.amount ?? c.max_amount_per_month ?? '0',
            deduction_type: c.deduction_type ?? '',
            deduction_frequency: c.deduction_frequency ?? 'recurring',
            is_pre_tax: c.is_pre_tax,
            reimbursement_type: c.reimbursement_type ?? '',
            max_amount_per_month: c.max_amount_per_month ?? c.amount ?? '0',
        });
        setShowModal(true);
    };

    const handleSave = async () => {
        setSaving(true);
        try {
            const payload = {
                ...form,
                max_amount_per_month:
                    form.type === 'reimbursement' && form.calculation_type === 'flat_amount'
                        ? form.amount
                        : form.max_amount_per_month,
            };
            const response = editing
                ? await axios.put(`/admin/salaries/components/${editing.id}`, payload)
                : await axios.post('/admin/salaries/components', payload);
            handleApiResponse(response);
            setShowModal(false);
            fetchComponents();
        } catch (error) {
            handleApiError(error);
        } finally {
            setSaving(false);
        }
    };

    const handleDelete = async () => {
        if (!deleteTarget) return;
        setDeleting(true);
        try {
            const response = await axios.delete(`/admin/salaries/components/${deleteTarget.id}`);
            handleApiResponse(response);
            setDeleteTarget(null);
            fetchComponents();
        } catch (error) {
            handleApiError(error);
        } finally {
            setDeleting(false);
        }
    };

    // Earnings table
    const EarningsTable = () => (
        <Table>
            <TableHeader>
                <TableRow>
                    <TableHead>NAME</TableHead>
                    <TableHead>EARNING TYPE</TableHead>
                    <TableHead>NAME IN PAYSLIP</TableHead>
                    <TableHead>CALCULATION TYPE</TableHead>
                    <TableHead>AMOUNT</TableHead>
                    <TableHead>STATUS</TableHead>
                    <TableHead className="text-right">ACTIONS</TableHead>
                </TableRow>
            </TableHeader>
            <TableBody>
                {components.map((c) => (
                    <TableRow key={c.id}>
                        <TableCell className="font-medium text-primary cursor-pointer hover:underline" onClick={() => openEdit(c)}>
                            {c.name}
                        </TableCell>
                        <TableCell>{c.earning_type ?? '-'}</TableCell>
                        <TableCell>{c.name_in_payslip ?? '-'}</TableCell>
                        <TableCell>{calcTypeLabel(c.calculation_type)}</TableCell>
                        <TableCell>{formatComponentValue(c.calculation_type, c.amount)}</TableCell>
                        <TableCell>
                            <Badge variant={c.is_active ? 'default' : 'secondary'}>
                                {c.is_active ? 'Active' : 'Inactive'}
                            </Badge>
                        </TableCell>
                        <TableCell className="text-right">
                            <ActionButtons comp={c} />
                        </TableCell>
                    </TableRow>
                ))}
            </TableBody>
        </Table>
    );

    // Deductions table (grouped)
    const DeductionsTable = () => {
        const preTax = components.filter((c) => c.is_pre_tax);
        const postTax = components.filter((c) => !c.is_pre_tax);

        const DeductionGroup = ({ label, items }: { label: string; items: SalaryComponent[] }) => (
            <>
                <TableRow className="bg-muted/40 hover:bg-muted/40">
                    <TableCell colSpan={7} className="py-2 font-semibold text-sm">
                        {label}
                        <Info className="inline h-3.5 w-3.5 ml-1.5 text-muted-foreground" />
                    </TableCell>
                </TableRow>
                {items.length === 0 ? (
                    <TableRow>
                        <TableCell colSpan={7} className="text-sm text-muted-foreground italic py-3 pl-6">
                            No components
                        </TableCell>
                    </TableRow>
                ) : (
                    items.map((c) => (
                        <TableRow key={c.id}>
                            <TableCell className="font-medium text-primary cursor-pointer hover:underline pl-6" onClick={() => openEdit(c)}>
                                {c.name}
                            </TableCell>
                            <TableCell>{c.deduction_type ?? '-'}</TableCell>
                            <TableCell>
                                {c.deduction_frequency === 'recurring' ? 'Recurring' : c.deduction_frequency === 'one_time' ? 'One Time' : '-'}
                            </TableCell>
                            <TableCell>{calcTypeLabel(c.calculation_type)}</TableCell>
                            <TableCell>{formatComponentValue(c.calculation_type, c.amount)}</TableCell>
                            <TableCell>
                                <Badge variant={c.is_active ? 'default' : 'secondary'}>
                                    {c.is_active ? 'Active' : 'Inactive'}
                                </Badge>
                            </TableCell>
                            <TableCell className="text-right">
                                <ActionButtons comp={c} />
                            </TableCell>
                        </TableRow>
                    ))
                )}
            </>
        );

        return (
            <Table>
                <TableHeader>
                    <TableRow>
                        <TableHead>NAME</TableHead>
                        <TableHead>DEDUCTION TYPE</TableHead>
                        <TableHead>DEDUCTION FREQUENCY</TableHead>
                        <TableHead>CALCULATION TYPE</TableHead>
                        <TableHead>AMOUNT</TableHead>
                        <TableHead>STATUS</TableHead>
                        <TableHead className="text-right">ACTIONS</TableHead>
                    </TableRow>
                </TableHeader>
                <TableBody>
                    <DeductionGroup label="Pre-Tax Deductions" items={preTax} />
                    <DeductionGroup label="Post-Tax Deductions" items={postTax} />
                </TableBody>
            </Table>
        );
    };

    // Reimbursements table
    const ReimbursementsTable = () => (
        <>
            <div className="flex items-start gap-2 rounded-md bg-orange-50 border border-orange-200 px-4 py-3 text-sm text-orange-800 mb-4">
                <Info className="mt-0.5 h-4 w-4 shrink-0" />
                <span>
                    With these reimbursement components, employees can claim reimbursements for the
                    components which are part of their salary structure.
                </span>
            </div>
            <Table>
                <TableHeader>
                    <TableRow>
                        <TableHead>NAME</TableHead>
                        <TableHead>REIMBURSEMENT TYPE</TableHead>
                        <TableHead>CALCULATION TYPE</TableHead>
                        <TableHead>VALUE</TableHead>
                        <TableHead>STATUS</TableHead>
                        <TableHead className="text-right">ACTIONS</TableHead>
                    </TableRow>
                </TableHeader>
                <TableBody>
                    {components.map((c) => (
                        <TableRow key={c.id}>
                            <TableCell className="font-medium text-primary cursor-pointer hover:underline" onClick={() => openEdit(c)}>
                                {c.name}
                            </TableCell>
                            <TableCell>{c.reimbursement_type ?? '-'}</TableCell>
                            <TableCell>{calcTypeLabel(c.calculation_type)}</TableCell>
                            <TableCell>
                                {formatComponentValue(
                                    c.calculation_type,
                                    c.amount ?? c.max_amount_per_month,
                                )}
                                {c.calculation_type === 'flat_amount' ? ' per month' : ''}
                            </TableCell>
                            <TableCell>
                                <Badge variant={c.is_active ? 'default' : 'secondary'}>
                                    {c.is_active ? 'Active' : 'Inactive'}
                                </Badge>
                            </TableCell>
                            <TableCell className="text-right">
                                <ActionButtons comp={c} />
                            </TableCell>
                        </TableRow>
                    ))}
                </TableBody>
            </Table>
        </>
    );

    const ActionButtons = ({ comp }: { comp: SalaryComponent }) => (
        <div className="flex items-center justify-end gap-1">
            <Button variant="ghost" size="icon" onClick={() => openEdit(comp)}>
                <Edit className="h-4 w-4" />
            </Button>
            <Button
                variant="ghost"
                size="icon"
                className="text-destructive hover:text-destructive"
                onClick={() => setDeleteTarget(comp)}
            >
                <Trash2 className="h-4 w-4" />
            </Button>
        </div>
    );

    const tabConfig = [
        { value: 'earning' as ComponentType, label: 'Earnings', icon: <DollarSign className="h-4 w-4" /> },
        { value: 'deduction' as ComponentType, label: 'Deductions', icon: <Minus className="h-4 w-4" /> },
        { value: 'reimbursement' as ComponentType, label: 'Reimbursements', icon: <Wallet className="h-4 w-4" /> },
    ];

    const canSave = () => {
        if (!form.name.trim()) return false;
        if (form.type === 'earning') return !!form.earning_type.trim() && !!form.name_in_payslip.trim();
        if (form.type === 'deduction') return !!form.deduction_type.trim();
        if (form.type === 'reimbursement') return !!form.reimbursement_type.trim();
        return true;
    };

    return (
        <AppLayout breadcrumbs={breadcrumbs}>
            

            <div className="flex flex-1 flex-col gap-6">
                {/* Hero Header */}
                <div className="relative overflow-hidden rounded-2xl bg-gradient-to-br from-[#e8f2fd] via-[#d0e4f8] to-[#c4d8f0] dark:from-[#0d1e33] dark:via-[#0a1828] dark:to-[#071220] px-6 py-5 shadow-sm border border-white/60 dark:border-white/10">
                    {/* decorative blob */}
                    <div className="pointer-events-none absolute -top-10 -right-10 w-48 h-48 opacity-20">
                        <svg viewBox="0 0 200 200" xmlns="http://www.w3.org/2000/svg">
                            <path fill="#071b3a" d="M44.7,-76.4C58.4,-69.7,70.3,-58.6,77.9,-44.9C85.5,-31.2,88.7,-15.6,87.4,-0.8C86,14,80,28,72.1,40.5C64.2,53,54.2,64,42.1,71.3C30,78.6,15,82.3,0.1,82.1C-14.8,81.9,-29.6,77.8,-42.7,70.5C-55.8,63.2,-67.3,52.7,-74.5,39.5C-81.7,26.3,-84.7,10.5,-83.1,-4.9C-81.6,-20.3,-75.5,-35.2,-66.3,-47.4C-57.1,-59.6,-44.8,-69.1,-31.6,-76.1C-18.4,-83.1,-4.6,-87.6,8.2,-86.2C21,-84.8,31,-83.1,44.7,-76.4Z" transform="translate(100 100)" />
                        </svg>
                    </div>
                    <div className="relative flex flex-col sm:flex-row sm:items-center justify-between gap-4">
                        <div className="flex items-center gap-4">
                            <div className="flex h-12 w-12 shrink-0 items-center justify-center rounded-xl bg-[#071b3a]/15 dark:bg-white/10 border border-[#071b3a]/20 dark:border-white/10 shadow-inner">
                                <Wallet className="h-6 w-6 text-[#071b3a] dark:text-blue-300" />
                            </div>
                            <div>
                                <h1 className="text-xl font-bold tracking-tight text-[#001f3f] dark:text-white">
                                    Salary Components
                                </h1>
                                <p className="text-sm text-[#1e3a5f]/60 dark:text-blue-200/60">
                                    Manage earnings, deductions, and reimbursement components
                                </p>
                            </div>
                        </div>
                        <div className="flex items-center gap-3">
                            <Button
                                variant="outline"
                                onClick={fetchComponents}
                                disabled={loading}
                                className="shrink-0 bg-white/50 border-white/60 hover:bg-white/80 dark:bg-black/20 dark:border-white/10 dark:hover:bg-black/40 text-[#001f3f] dark:text-white backdrop-blur-sm z-10"
                            >
                                <RefreshCw className={`h-4 w-4 mr-2 ${loading ? 'animate-spin' : ''}`} />
                                Refresh
                            </Button>
                            <Button
                                onClick={openCreate}
                                className="shrink-0 bg-gradient-to-r from-[#071b3a] to-[#0d4a8a] hover:from-[#040f22] hover:to-[#0a3272] text-white shadow-md shadow-blue-500/25 dark:shadow-blue-900/40 rounded-xl gap-2 z-10"
                            >
                                <Plus className="h-4 w-4" />
                                Add Component
                            </Button>
                        </div>
                    </div>
                </div>

                {/* Search */}
                <div className="relative max-w-xs">
                    <Search className="absolute left-2.5 top-2.5 h-4 w-4 text-muted-foreground" />
                    <Input
                        className="pl-9"
                        placeholder="Search components..."
                        value={search}
                        onChange={(e) => setSearch(e.target.value)}
                    />
                    {search && (
                        <button
                            className="absolute right-2.5 top-2.5 text-muted-foreground hover:text-foreground"
                            onClick={() => setSearch('')}
                        >
                            <X className="h-4 w-4" />
                        </button>
                    )}
                </div>

                {/* Tabs */}
                <Tabs value={activeTab} onValueChange={(v) => { setActiveTab(v as ComponentType); setSearch(''); }}>
                    <TabsList>
                        {tabConfig.map((tab) => (
                            <TabsTrigger key={tab.value} value={tab.value} className="gap-2">
                                {tab.icon}
                                {tab.label}
                            </TabsTrigger>
                        ))}
                    </TabsList>

                    {tabConfig.map((tab) => (
                        <TabsContent key={tab.value} value={tab.value} className="mt-4">
                            <Card>
                                <CardContent className="p-0">
                                    {loading ? (
                                        <div className="flex items-center justify-center py-16 text-sm text-muted-foreground">
                                            Loading...
                                        </div>
                                    ) : components.length === 0 ? (
                                        <div className="flex flex-col items-center justify-center py-16 gap-3 text-sm text-muted-foreground">
                                            <span>No {tab.label.toLowerCase()} components found.</span>
                                            <Button variant="outline" size="sm" onClick={openCreate}>
                                                <Plus className="h-4 w-4" /> Add {tab.label.slice(0, -1)}
                                            </Button>
                                        </div>
                                    ) : (
                                        <>
                                            {tab.value === 'earning' && <EarningsTable />}
                                            {tab.value === 'deduction' && <DeductionsTable />}
                                            {tab.value === 'reimbursement' && (
                                                <div className="p-4">
                                                    <ReimbursementsTable />
                                                </div>
                                            )}
                                        </>
                                    )}
                                </CardContent>
                            </Card>
                        </TabsContent>
                    ))}
                </Tabs>
            </div>

            {/* Create / Edit Modal */}
            <Dialog open={showModal} onOpenChange={setShowModal}>
                <DialogContent className="max-w-lg">
                    <DialogHeader>
                        <DialogTitle>
                            {editing ? 'Edit' : 'Add'}{' '}
                            {activeTab === 'earning' ? 'Earning' : activeTab === 'deduction' ? 'Deduction' : 'Reimbursement'}{' '}
                            Component
                        </DialogTitle>
                    </DialogHeader>

                    <div className="space-y-4 py-2">
                        {/* EARNINGS fields */}
                        {form.type === 'earning' && (
                            <>
                                <div className="space-y-1.5">
                                    <Label>Earning Type *</Label>
                                    <Input
                                        value={form.earning_type}
                                        onChange={(e) => setForm({ ...form, earning_type: e.target.value, name: e.target.value, name_in_payslip: e.target.value })}
                                        placeholder="e.g. Conveyance Allowance"
                                    />
                                </div>
                                <div className="space-y-1.5">
                                    <Label>Earning Name *</Label>
                                    <Input
                                        value={form.name}
                                        onChange={(e) => setForm({ ...form, name: e.target.value })}
                                        placeholder="e.g. Conveyance Allowance"
                                    />
                                </div>
                                <div className="space-y-1.5">
                                    <Label>Name in Payslip *</Label>
                                    <Input
                                        value={form.name_in_payslip}
                                        onChange={(e) => setForm({ ...form, name_in_payslip: e.target.value })}
                                        placeholder="e.g. Conveyance"
                                    />
                                </div>
                                <CalculationTypeFields
                                    calculationType={form.calculation_type}
                                    amount={form.amount}
                                    onCalculationTypeChange={(v) => setForm({ ...form, calculation_type: v })}
                                    onAmountChange={(v) => setForm({ ...form, amount: v })}
                                    showCtcOption={form.type === 'earning'}
                                    showGrossOption={form.type === 'earning'}
                                />
                            </>
                        )}

                        {/* DEDUCTION fields */}
                        {form.type === 'deduction' && (
                            <>
                                <div className="space-y-1.5">
                                    <Label>Deduction Name *</Label>
                                    <Input
                                        value={form.name}
                                        onChange={(e) => setForm({ ...form, name: e.target.value, deduction_type: e.target.value })}
                                        placeholder="e.g. Voluntary Provident Fund"
                                    />
                                </div>
                                <div className="space-y-1.5">
                                    <Label>Deduction Type *</Label>
                                    <Input
                                        value={form.deduction_type}
                                        onChange={(e) => setForm({ ...form, deduction_type: e.target.value })}
                                        placeholder="e.g. Voluntary Provident Fund"
                                    />
                                </div>
                                <div className="space-y-1.5">
                                    <Label>Deduction Frequency *</Label>
                                    <Select
                                        value={form.deduction_frequency}
                                        onValueChange={(v) => setForm({ ...form, deduction_frequency: v as 'recurring' | 'one_time' })}
                                    >
                                        <SelectTrigger>
                                            <SelectValue />
                                        </SelectTrigger>
                                        <SelectContent>
                                            <SelectItem value="recurring">Recurring</SelectItem>
                                            <SelectItem value="one_time">One Time</SelectItem>
                                        </SelectContent>
                                    </Select>
                                </div>
                                <div className="space-y-1.5">
                                    <Label>Tax Category *</Label>
                                    <Select
                                        value={form.is_pre_tax ? 'pre' : 'post'}
                                        onValueChange={(v) => setForm({ ...form, is_pre_tax: v === 'pre' })}
                                    >
                                        <SelectTrigger>
                                            <SelectValue />
                                        </SelectTrigger>
                                        <SelectContent>
                                            <SelectItem value="pre">Pre-Tax</SelectItem>
                                            <SelectItem value="post">Post-Tax</SelectItem>
                                        </SelectContent>
                                    </Select>
                                </div>
                                <CalculationTypeFields
                                    calculationType={form.calculation_type}
                                    amount={form.amount}
                                    onCalculationTypeChange={(v) => setForm({ ...form, calculation_type: v })}
                                    onAmountChange={(v) => setForm({ ...form, amount: v })}
                                    showCtcOption={form.type === 'earning'}
                                />
                            </>
                        )}

                        {/* REIMBURSEMENT fields */}
                        {form.type === 'reimbursement' && (
                            <>
                                <div className="space-y-1.5">
                                    <Label>Reimbursement Name *</Label>
                                    <Input
                                        value={form.name}
                                        onChange={(e) => setForm({ ...form, name: e.target.value, reimbursement_type: e.target.value })}
                                        placeholder="e.g. Fuel Reimbursement"
                                    />
                                </div>
                                <div className="space-y-1.5">
                                    <Label>Reimbursement Type *</Label>
                                    <Input
                                        value={form.reimbursement_type}
                                        onChange={(e) => setForm({ ...form, reimbursement_type: e.target.value })}
                                        placeholder="e.g. Fuel Reimbursement"
                                    />
                                </div>
                                <CalculationTypeFields
                                    calculationType={form.calculation_type}
                                    amount={form.amount}
                                    onCalculationTypeChange={(v) => setForm({ ...form, calculation_type: v })}
                                    onAmountChange={(v) => setForm({ ...form, amount: v, max_amount_per_month: v })}
                                />
                            </>
                        )}

                        {/* Description (all types) */}
                        <div className="space-y-1.5">
                            <Label>Description</Label>
                            <Textarea
                                rows={2}
                                value={form.description}
                                onChange={(e) => setForm({ ...form, description: e.target.value })}
                                placeholder="Optional..."
                            />
                        </div>

                        {/* Active toggle */}
                        <div className="flex items-center gap-3">
                            <Checkbox
                                id="comp-active"
                                checked={form.is_active}
                                onCheckedChange={(checked) => setForm({ ...form, is_active: !!checked })}
                            />
                            <Label htmlFor="comp-active" className="cursor-pointer">
                                Mark this as Active
                            </Label>
                        </div>
                    </div>

                    <DialogFooter>
                        <Button variant="outline" onClick={() => setShowModal(false)}>
                            Cancel
                        </Button>
                        <Button onClick={handleSave} disabled={saving || !canSave()}>
                            {saving ? 'Saving...' : editing ? 'Update' : 'Create'}
                        </Button>
                    </DialogFooter>
                </DialogContent>
            </Dialog>

            {/* Delete Confirm */}
            <AlertDialog open={!!deleteTarget} onOpenChange={(open) => !open && setDeleteTarget(null)}>
                <AlertDialogContent>
                    <AlertDialogHeader>
                        <AlertDialogTitle>Delete Component</AlertDialogTitle>
                        <AlertDialogDescription>
                            Are you sure you want to delete <strong>{deleteTarget?.name}</strong>? This action
                            cannot be undone.
                        </AlertDialogDescription>
                    </AlertDialogHeader>
                    <AlertDialogFooter>
                        <AlertDialogCancel>Cancel</AlertDialogCancel>
                        <AlertDialogAction
                            onClick={handleDelete}
                            disabled={deleting}
                            className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
                        >
                            {deleting ? 'Deleting...' : 'Delete'}
                        </AlertDialogAction>
                    </AlertDialogFooter>
                </AlertDialogContent>
            </AlertDialog>
        </AppLayout>
    );
}
