import { useNavigate } from 'react-router-dom';
import {
    Banknote,
    ChevronDown,
    ChevronLeft,
    ChevronRight,
    ChevronsLeft,
    ChevronsRight,
    FileText,
    MoreHorizontal,
    RefreshCw,
    Search,
    UserPlus,
    Users,
    X,
} from 'lucide-react';
import { useEffect, useState } from 'react';
import axios from '@/lib/axios';

import { Avatar, AvatarFallback, AvatarImage } from '@/components/ui/avatar';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { Checkbox } from '@/components/ui/checkbox';
import {
    Dialog,
    DialogContent,
    DialogHeader,
    DialogTitle,
} from '@/components/ui/dialog';
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuSeparator,
    DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { Input } from '@/components/ui/input';
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
import { SalaryTabsPanel } from '@/components/salary-tabs-panel';
import AppLayout from '@/layouts/app-layout';
import { handleApiError } from '@/lib/toast';

function SalaryStructureDialog({
    open,
    onOpenChange,
    userId,
    userName,
}: {
    open: boolean;
    onOpenChange: (open: boolean) => void;
    userId: number;
    userName: string;
}) {
    return (
        <Dialog open={open} onOpenChange={onOpenChange}>
            <DialogContent className="max-w-lg max-h-[90vh] overflow-y-auto">
                <DialogHeader>
                    <DialogTitle className="flex items-center gap-2">
                        <Banknote className="h-5 w-5" />
                        Salary Structure &mdash; {userName}
                    </DialogTitle>
                </DialogHeader>
                {open && <SalaryTabsPanel userId={userId} />}
            </DialogContent>
        </Dialog>
    );
}

interface Employee {
    id: number;
    name: string;
    email: string;
    employee_id: string | null;
    status: string;
    avatar: string | null;
    photo: string | null;
    salary: string | null;
    department: { id: number; name: string } | null;
    designation: { id: number; name: string } | null;
    last_payslip_date: string | null;
}

interface FilterOption {
    id: number;
    name: string;
}

const STATUS_OPTIONS = [
    { value: 'active', label: 'Active Employees' },
    { value: 'inactive', label: 'Inactive Employees' },
    { value: 'suspended', label: 'Suspended Employees' },
    { value: 'all', label: 'All Employees' },
];

const formatCtc = (salary: string | null): string => {
    if (!salary || Number(salary) === 0) return '\u2014';
    return '\u20b9' + Number(salary).toLocaleString('en-IN', { minimumFractionDigits: 2, maximumFractionDigits: 2 }) + ' per month';
};

const getInitials = (name: string) =>
    name
        .split(' ')
        .slice(0, 2)
        .map((n) => n[0])
        .join('')
        .toUpperCase();

export default function EmployeesPage() {
    const navigate = useNavigate();
    const [employees, setEmployees] = useState<Employee[]>([]);
    const [loading, setLoading] = useState(false);
    const [search, setSearch] = useState('');
    const [status, setStatus] = useState('active');
    const [departmentId, setDepartmentId] = useState('all');
    const [designationId, setDesignationId] = useState('all');
    const [departments, setDepartments] = useState<FilterOption[]>([]);
    const [designations, setDesignations] = useState<FilterOption[]>([]);
    const [selected, setSelected] = useState<number[]>([]);
    const [salaryDialogEmployee, setSalaryDialogEmployee] = useState<{ id: number; name: string } | null>(null);

    const [currentPage, setCurrentPage] = useState(1);
    const [lastPage, setLastPage] = useState(1);
    const [perPage, setPerPage] = useState(15);
    const [total, setTotal] = useState(0);
    const [from, setFrom] = useState(0);
    const [to, setTo] = useState(0);

    const breadcrumbs = [
        { title: 'Salaries', href: '/admin/salaries/components' },
        { title: 'Employees', href: '/admin/salaries/employees' },
    ];

    const currentStatusLabel = STATUS_OPTIONS.find((s) => s.value === status)?.label ?? 'Employees';

    const fetchFilterOptions = async () => {
        try {
            const res = await axios.get('/admin/salaries/employees/filter-options');
            if (res.data.success) {
                setDepartments(res.data.data.departments);
                setDesignations(res.data.data.designations);
            }
        } catch {
            // silent
        }
    };

    const fetchEmployees = async () => {
        setLoading(true);
        try {
            const res = await axios.get('/admin/salaries/employees/list', {
                params: {
                    search: search || undefined,
                    status: status !== 'all' ? status : undefined,
                    department_id: departmentId !== 'all' ? departmentId : undefined,
                    designation_id: designationId !== 'all' ? designationId : undefined,
                    page: currentPage,
                    per_page: perPage,
                },
            });
            if (res.data.success) {
                const d = res.data.data;
                setEmployees(d.data);
                setLastPage(d.last_page);
                setTotal(d.total);
                setFrom(d.from ?? 0);
                setTo(d.to ?? 0);
                setSelected([]);
            }
        } catch (error) {
            handleApiError(error);
        } finally {
            setLoading(false);
        }
    };

    useEffect(() => {
        fetchFilterOptions();
    }, []);

    useEffect(() => {
        setCurrentPage(1);
    }, [search, status, departmentId, designationId, perPage]);

    useEffect(() => {
        fetchEmployees();
    }, [search, status, departmentId, designationId, currentPage, perPage]);

    const toggleSelect = (id: number) =>
        setSelected((prev) => (prev.includes(id) ? prev.filter((x) => x !== id) : [...prev, id]));

    const toggleAll = () =>
        setSelected((prev) => (prev.length === employees.length ? [] : employees.map((e) => e.id)));

    const allSelected = employees.length > 0 && selected.length === employees.length;
    const someSelected = selected.length > 0 && !allSelected;

    const hasFilters = search || departmentId !== 'all' || designationId !== 'all';

    return (
        <AppLayout breadcrumbs={breadcrumbs}>
            

            <div className="flex flex-1 flex-col gap-4">
                {/* Hero Header */}
                <div className="relative overflow-hidden rounded-2xl bg-gradient-to-br from-[#e8f2fd] via-[#d0e4f8] to-[#c4d8f0] dark:from-[#0d1e33] dark:via-[#0a1828] dark:to-[#071220] px-6 py-5 shadow-sm border border-white/60 dark:border-white/10 mb-2">
                    {/* decorative blob */}
                    <div className="pointer-events-none absolute -top-10 -right-10 w-48 h-48 opacity-20">
                        <svg viewBox="0 0 200 200" xmlns="http://www.w3.org/2000/svg">
                            <path fill="#071b3a" d="M44.7,-76.4C58.4,-69.7,70.3,-58.6,77.9,-44.9C85.5,-31.2,88.7,-15.6,87.4,-0.8C86,14,80,28,72.1,40.5C64.2,53,54.2,64,42.1,71.3C30,78.6,15,82.3,0.1,82.1C-14.8,81.9,-29.6,77.8,-42.7,70.5C-55.8,63.2,-67.3,52.7,-74.5,39.5C-81.7,26.3,-84.7,10.5,-83.1,-4.9C-81.6,-20.3,-75.5,-35.2,-66.3,-47.4C-57.1,-59.6,-44.8,-69.1,-31.6,-76.1C-18.4,-83.1,-4.6,-87.6,8.2,-86.2C21,-84.8,31,-83.1,44.7,-76.4Z" transform="translate(100 100)" />
                        </svg>
                    </div>
                    <div className="relative flex flex-col sm:flex-row sm:items-center justify-between gap-4">
                        <div className="flex items-center gap-4">
                            <div className="flex h-12 w-12 shrink-0 items-center justify-center rounded-xl bg-[#071b3a]/15 dark:bg-white/10 border border-[#071b3a]/20 dark:border-white/10 shadow-inner">
                                <Users className="h-6 w-6 text-[#071b3a] dark:text-blue-300" />
                            </div>
                            <div>
                                <DropdownMenu>
                                    <DropdownMenuTrigger asChild>
                                        <Button variant="ghost" className="text-xl font-bold tracking-tight text-[#001f3f] dark:text-white px-0 gap-1.5 hover:bg-transparent focus-visible:ring-0">
                                            {currentStatusLabel}
                                            <ChevronDown className="h-5 w-5 text-[#1e3a5f]/60 dark:text-blue-200/60" />
                                        </Button>
                                    </DropdownMenuTrigger>
                                    <DropdownMenuContent align="start">
                                        {STATUS_OPTIONS.map((opt) => (
                                            <DropdownMenuItem
                                                key={opt.value}
                                                onSelect={() => setStatus(opt.value)}
                                                className={status === opt.value ? 'font-semibold' : ''}
                                            >
                                                {opt.label}
                                            </DropdownMenuItem>
                                        ))}
                                    </DropdownMenuContent>
                                </DropdownMenu>
                                <p className="text-sm text-[#1e3a5f]/60 dark:text-blue-200/60 mt-0.5">
                                    Manage employees, cost to company, and payslips
                                </p>
                            </div>
                        </div>
                        <div className="flex items-center gap-3">
                            <Button
                                variant="outline"
                                onClick={fetchEmployees}
                                disabled={loading}
                                className="shrink-0 bg-white/50 border-white/60 hover:bg-white/80 dark:bg-black/20 dark:border-white/10 dark:hover:bg-black/40 text-[#001f3f] dark:text-white backdrop-blur-sm z-10"
                            >
                                <RefreshCw className={`h-4 w-4 mr-2 ${loading ? 'animate-spin' : ''}`} />
                                Refresh
                            </Button>
                            <Button
                                onClick={() => navigate('/admin/users')}
                                className="shrink-0 bg-gradient-to-r from-[#071b3a] to-[#0d4a8a] hover:from-[#040f22] hover:to-[#0a3272] text-white shadow-md shadow-blue-500/25 dark:shadow-blue-900/40 rounded-xl gap-2 z-10"
                            >
                                <UserPlus className="h-4 w-4" />
                                Add Employee
                            </Button>
                        </div>
                    </div>
                </div>

                {/* Filters */}
                <div className="flex flex-wrap items-center gap-2">
                    <div className="relative min-w-[220px]">
                        <Search className="absolute left-2.5 top-2.5 h-4 w-4 text-muted-foreground" />
                        <Input
                            placeholder="Search employees..."
                            className="pl-9"
                            value={search}
                            onChange={(e) => setSearch(e.target.value)}
                        />
                    </div>

                    <Select value={departmentId} onValueChange={setDepartmentId}>
                        <SelectTrigger className="w-44">
                            <SelectValue placeholder="Department" />
                        </SelectTrigger>
                        <SelectContent>
                            <SelectItem value="all">All Departments</SelectItem>
                            {departments.map((d) => (
                                <SelectItem key={d.id} value={String(d.id)}>
                                    {d.name}
                                </SelectItem>
                            ))}
                        </SelectContent>
                    </Select>

                    <Select value={designationId} onValueChange={setDesignationId}>
                        <SelectTrigger className="w-44">
                            <SelectValue placeholder="Designation" />
                        </SelectTrigger>
                        <SelectContent>
                            <SelectItem value="all">All Designations</SelectItem>
                            {designations.map((d) => (
                                <SelectItem key={d.id} value={String(d.id)}>
                                    {d.name}
                                </SelectItem>
                            ))}
                        </SelectContent>
                    </Select>

                    {hasFilters && (
                        <Button
                            variant="ghost"
                            size="sm"
                            onClick={() => { setSearch(''); setDepartmentId('all'); setDesignationId('all'); }}
                        >
                            <X className="h-4 w-4" />
                            Clear
                        </Button>
                    )}
                </div>

                {/* Table */}
                <Card>
                    <CardContent className="p-0">
                        {loading ? (
                            <div className="flex items-center justify-center py-16 text-sm text-muted-foreground">
                                Loading...
                            </div>
                        ) : employees.length === 0 ? (
                            <div className="flex items-center justify-center py-16 text-sm text-muted-foreground">
                                No employees found.
                            </div>
                        ) : (
                            <Table>
                                <TableHeader>
                                    <TableRow>
                                        <TableHead className="w-10">
                                            <Checkbox
                                                checked={allSelected}
                                                data-state={someSelected ? 'indeterminate' : allSelected ? 'checked' : 'unchecked'}
                                                onCheckedChange={toggleAll}
                                            />
                                        </TableHead>
                                        <TableHead>EMPLOYEE</TableHead>
                                        <TableHead>DEPARTMENT</TableHead>
                                        <TableHead>COST TO COMPANY</TableHead>
                                        <TableHead>LAST PAYSLIP</TableHead>
                                        <TableHead className="w-10"></TableHead>
                                    </TableRow>
                                </TableHeader>
                                <TableBody>
                                    {employees.map((emp) => (
                                        <TableRow key={emp.id} className={selected.includes(emp.id) ? 'bg-muted/40' : ''}>
                                            <TableCell>
                                                <Checkbox
                                                    checked={selected.includes(emp.id)}
                                                    onCheckedChange={() => toggleSelect(emp.id)}
                                                />
                                            </TableCell>
                                            <TableCell>
                                                <div className="flex items-center gap-3">
                                                    <Avatar className="h-9 w-9 shrink-0">
                                                        <AvatarImage src={emp.photo ?? emp.avatar ?? undefined} />
                                                        <AvatarFallback className="text-xs bg-muted">
                                                            {getInitials(emp.name)}
                                                        </AvatarFallback>
                                                    </Avatar>
                                                    <div>
                                                        <button
                                                            type="button"
                                                            className="text-sm font-medium text-primary hover:underline text-left"
                                                            onClick={() => navigate(`/admin/users/${emp.id}`)}
                                                        >
                                                            {emp.name}{emp.employee_id ? ` - ${emp.employee_id}` : ''}
                                                        </button>
                                                        <p className="text-xs text-muted-foreground">{emp.email}</p>
                                                        {emp.designation && (
                                                            <p className="text-xs text-muted-foreground">
                                                                {emp.designation.name}
                                                            </p>
                                                        )}
                                                    </div>
                                                </div>
                                            </TableCell>
                                            <TableCell className="text-sm">
                                                {emp.department?.name ?? '\u2014'}
                                            </TableCell>
                                            <TableCell className="text-sm">
                                                {formatCtc(emp.salary)}
                                            </TableCell>
                                            <TableCell className="text-sm text-muted-foreground">
                                                {emp.last_payslip_date
                                                    ? new Date(emp.last_payslip_date).toLocaleDateString('en-IN', { day: '2-digit', month: 'short', year: 'numeric' })
                                                    : '\u2014'}
                                            </TableCell>
                                            <TableCell>
                                                <DropdownMenu>
                                                    <DropdownMenuTrigger asChild>
                                                        <Button variant="ghost" size="icon">
                                                            <MoreHorizontal className="h-4 w-4" />
                                                        </Button>
                                                    </DropdownMenuTrigger>
                                                    <DropdownMenuContent align="end">
                                                        <DropdownMenuItem onSelect={() => setSalaryDialogEmployee({ id: emp.id, name: emp.name })}>
                                                            <Banknote className="h-4 w-4 mr-2" />
                                                            Salary Structure
                                                        </DropdownMenuItem>
                                                        <DropdownMenuSeparator />
                                                        <DropdownMenuItem onSelect={() => navigate(`/admin/salaries/employees/${emp.id}/payslips`)}>
                                                            <FileText className="h-4 w-4 mr-2" />
                                                            Payslips
                                                        </DropdownMenuItem>
                                                    </DropdownMenuContent>
                                                </DropdownMenu>
                                            </TableCell>
                                        </TableRow>
                                    ))}
                                </TableBody>
                            </Table>
                        )}
                    </CardContent>
                </Card>

                {/* Pagination */}
                {total > 0 && (
                    <div className="flex items-center justify-between text-sm text-muted-foreground">
                        <div className="flex items-center gap-2">
                            <span>Rows per page</span>
                            <Select value={String(perPage)} onValueChange={(v) => setPerPage(Number(v))}>
                                <SelectTrigger className="h-8 w-16">
                                    <SelectValue />
                                </SelectTrigger>
                                <SelectContent>
                                    {[10, 15, 25, 50, 100].map((n) => (
                                        <SelectItem key={n} value={String(n)}>{n}</SelectItem>
                                    ))}
                                </SelectContent>
                            </Select>
                        </div>
                        <div className="flex items-center gap-2">
                            <span>{from}&ndash;{to} of {total}</span>
                            <div className="flex items-center gap-1">
                                <Button variant="outline" size="icon" className="h-8 w-8" onClick={() => setCurrentPage(1)} disabled={currentPage === 1}>
                                    <ChevronsLeft className="h-4 w-4" />
                                </Button>
                                <Button variant="outline" size="icon" className="h-8 w-8" onClick={() => setCurrentPage((p) => p - 1)} disabled={currentPage === 1}>
                                    <ChevronLeft className="h-4 w-4" />
                                </Button>
                                <Button variant="outline" size="icon" className="h-8 w-8" onClick={() => setCurrentPage((p) => p + 1)} disabled={currentPage === lastPage}>
                                    <ChevronRight className="h-4 w-4" />
                                </Button>
                                <Button variant="outline" size="icon" className="h-8 w-8" onClick={() => setCurrentPage(lastPage)} disabled={currentPage === lastPage}>
                                    <ChevronsRight className="h-4 w-4" />
                                </Button>
                            </div>
                        </div>
                    </div>
                )}
            </div>
            {salaryDialogEmployee && (
                <SalaryStructureDialog
                    open={!!salaryDialogEmployee}
                    onOpenChange={(open) => { if (!open) setSalaryDialogEmployee(null); }}
                    userId={salaryDialogEmployee.id}
                    userName={salaryDialogEmployee.name}
                />
            )}
        </AppLayout>
    );
}
