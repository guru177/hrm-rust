import { useState, useEffect } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import AppLayout from '@/layouts/app-layout';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import {
    Table,
    TableBody,
    TableCell,
    TableHead,
    TableHeader,
    TableRow,
} from '@/components/ui/table';
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuSeparator,
    DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import {
    Select,
    SelectContent,
    SelectItem,
    SelectTrigger,
    SelectValue,
} from '@/components/ui/select';
import { Badge } from '@/components/ui/badge';
import {
    Plus,
    Search,
    MoreVertical,
    Pencil,
    Trash2,
    Eye,
    Copy,
    PlayCircle,
    PauseCircle,
    Workflow as WorkflowIcon,
    RefreshCw,
} from 'lucide-react';
import { handleApiError, handleApiResponse } from '@/lib/toast';
import axios from '@/lib/axios';

interface Workflow {
    id: number;
    name: string;
    description: string | null;
    trigger_type: string;
    is_active: boolean;
    execution_count: number;
    last_executed_at: string | null;
    created_by: {
        id: number;
        name: string;
    };
    created_at: string;
}

const triggerTypeLabels: Record<string, string> = {
    leave_request_submitted: 'Leave Request Submitted',
    leave_request_approved: 'Leave Request Approved',
    leave_request_rejected: 'Leave Request Rejected',
    attendance_clock_in: 'Attendance Clock-In',
    user_created: 'User Created',
    task_due: 'Task Due',
    time_based: 'Time-Based',
};

export default function Index() {
    const navigate = useNavigate();
    const [workflows, setWorkflows] = useState<Workflow[]>([]);
    const [loadingData, setLoadingData] = useState(true);
    const [search, setSearch] = useState('');
    const [statusFilter, setStatusFilter] = useState('all');
    const [triggerFilter, setTriggerFilter] = useState('all');
    const [sortBy, setSortBy] = useState('created_at');
    const [sortOrder, setSortOrder] = useState<'asc' | 'desc'>('desc');
    const [perPage, setPerPage] = useState(15);
    const [currentPage, setCurrentPage] = useState(1);
    const [lastPage, setLastPage] = useState(1);
    const [total, setTotal] = useState(0);
    const [from, setFrom] = useState(0);
    const [to, setTo] = useState(0);
    const [actionLoading, setActionLoading] = useState<number | null>(null);

    const breadcrumbs = [{ title: 'Workflows', href: '/admin/workflows' }];

    useEffect(() => {
        fetchWorkflows();
    }, [search, statusFilter, triggerFilter, sortBy, sortOrder, currentPage, perPage]);

    const fetchWorkflows = async () => {
        setLoadingData(true);
        try {
            const response = await axios.get('/admin/workflows/list', {
                params: {
                    search: search || undefined,
                    status: statusFilter !== 'all' ? statusFilter : undefined,
                    trigger_type: triggerFilter !== 'all' ? triggerFilter : undefined,
                    sort_by: sortBy,
                    sort_order: sortOrder,
                    page: currentPage,
                    per_page: perPage,
                },
            });
            if (response.data.success) {
                const resData = response.data.data;
                if (Array.isArray(resData)) {
                    setWorkflows(resData);
                    setTotal(resData.length);
                    setFrom(resData.length > 0 ? 1 : 0);
                    setTo(resData.length);
                    setLastPage(1);
                } else {
                    setWorkflows(resData.data || []);
                    setCurrentPage(resData.current_page || 1);
                    setLastPage(resData.last_page || 1);
                    setTotal(resData.total || 0);
                    setFrom(resData.from || 0);
                    setTo(resData.to || 0);
                }
            }
        } catch (error) {
            handleApiError(error);
        } finally {
            setLoadingData(false);
        }
    };

    const handleSort = (column: string) => {
        if (sortBy === column) {
            setSortOrder(sortOrder === 'asc' ? 'desc' : 'asc');
        } else {
            setSortBy(column);
            setSortOrder('asc');
        }
    };

    const handleToggle = async (workflow: Workflow) => {
        setActionLoading(workflow.id);
        try {
            const response = await axios.post(`/admin/workflows/${workflow.id}/toggle`);
            handleApiResponse(response);
            fetchWorkflows();
        } catch (error) {
            handleApiError(error);
        } finally {
            setActionLoading(null);
        }
    };

    const handleDuplicate = async (workflow: Workflow) => {
        setActionLoading(workflow.id);
        try {
            const response = await axios.post(`/admin/workflows/${workflow.id}/duplicate`);
            handleApiResponse(response);
            fetchWorkflows();
        } catch (error) {
            handleApiError(error);
        } finally {
            setActionLoading(null);
        }
    };

    const handleDelete = async (workflow: Workflow) => {
        if (!confirm(`Are you sure you want to delete "${workflow.name}"? This action cannot be undone.`)) return;
        setActionLoading(workflow.id);
        try {
            const response = await axios.delete(`/admin/workflows/${workflow.id}`);
            handleApiResponse(response);
            fetchWorkflows();
        } catch (error) {
            handleApiError(error);
        } finally {
            setActionLoading(null);
        }
    };

    return (
        <AppLayout breadcrumbs={breadcrumbs}>

            <div className="space-y-6">
                {/* Hero Header */}
                <div className="relative overflow-hidden rounded-2xl bg-gradient-to-br from-[#e8f2fd] via-[#d0e4f8] to-[#c4d8f0] dark:from-[#0d1e33] dark:via-[#0a1828] dark:to-[#071220] px-6 py-5 shadow-sm border border-white/60 dark:border-white/10">
                    <div className="pointer-events-none absolute -top-10 -right-10 w-48 h-48 opacity-20">
                        <svg viewBox="0 0 200 200" xmlns="http://www.w3.org/2000/svg">
                            <path fill="#071b3a" d="M44.7,-76.4C58.4,-69.7,70.3,-58.6,77.9,-44.9C85.5,-31.2,88.7,-15.6,87.4,-0.8C86,14,80,28,72.1,40.5C64.2,53,54.2,64,42.1,71.3C30,78.6,15,82.3,0.1,82.1C-14.8,81.9,-29.6,77.8,-42.7,70.5C-55.8,63.2,-67.3,52.7,-74.5,39.5C-81.7,26.3,-84.7,10.5,-83.1,-4.9C-81.6,-20.3,-75.5,-35.2,-66.3,-47.4C-57.1,-59.6,-44.8,-69.1,-31.6,-76.1C-18.4,-83.1,-4.6,-87.6,8.2,-86.2C21,-84.8,31,-83.1,44.7,-76.4Z" transform="translate(100 100)" />
                        </svg>
                    </div>
                    <div className="relative flex flex-col sm:flex-row sm:items-center justify-between gap-4">
                        <div className="flex items-center gap-4">
                            <div className="flex h-12 w-12 shrink-0 items-center justify-center rounded-xl bg-[#071b3a]/15 dark:bg-white/10 border border-[#071b3a]/20 dark:border-white/10 shadow-inner">
                                <WorkflowIcon className="h-6 w-6 text-[#071b3a] dark:text-blue-300" />
                            </div>
                            <div>
                                <h1 className="text-xl font-bold tracking-tight text-[#001f3f] dark:text-white">
                                    Workflows
                                </h1>
                                <p className="text-sm text-[#1e3a5f]/60 dark:text-blue-200/60">
                                    Automate your business processes with custom workflows
                                </p>
                            </div>
                        </div>
                        <Link to="/admin/workflows/create">
                            <Button className="shrink-0 bg-gradient-to-r from-[#071b3a] to-[#0d4a8a] hover:from-[#040f22] hover:to-[#0a3272] text-white shadow-md shadow-blue-500/25 dark:shadow-blue-900/40 rounded-xl gap-2 z-10">
                                <Plus className="h-4 w-4" />
                                Create Workflow
                            </Button>
                        </Link>
                    </div>
                </div>

                {/* Filters */}
                <div className="flex flex-col gap-4 md:flex-row md:items-center">
                    <div className="flex-1">
                        <div className="relative">
                            <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
                            <Input
                                placeholder="Search workflows..."
                                value={search}
                                onChange={(e) => setSearch(e.target.value)}
                                className="pl-9"
                            />
                        </div>
                    </div>
                    <Select value={statusFilter} onValueChange={setStatusFilter}>
                        <SelectTrigger className="w-[150px]">
                            <SelectValue placeholder="Status" />
                        </SelectTrigger>
                        <SelectContent>
                            <SelectItem value="all">All Status</SelectItem>
                            <SelectItem value="active">Active</SelectItem>
                            <SelectItem value="inactive">Inactive</SelectItem>
                        </SelectContent>
                    </Select>
                    <Select value={triggerFilter} onValueChange={setTriggerFilter}>
                        <SelectTrigger className="w-[180px]">
                            <SelectValue placeholder="Trigger" />
                        </SelectTrigger>
                        <SelectContent>
                            <SelectItem value="all">All Triggers</SelectItem>
                            {Object.entries(triggerTypeLabels).map(([value, label]) => (
                                <SelectItem key={value} value={value}>{label}</SelectItem>
                            ))}
                        </SelectContent>
                    </Select>
                    <Button variant="outline" size="icon" onClick={fetchWorkflows} title="Refresh">
                        <RefreshCw className={`h-4 w-4 ${loadingData ? 'animate-spin' : ''}`} />
                    </Button>
                </div>

                {/* Table */}
                <div className="rounded-md border bg-card">
                    <Table>
                        <TableHeader>
                            <TableRow>
                                <TableHead className="cursor-pointer select-none hover:bg-muted/50" onClick={() => handleSort('id')}>
                                    <div className="flex items-center gap-1">
                                        ID {sortBy === 'id' && <span className="text-xs">{sortOrder === 'asc' ? '↑' : '↓'}</span>}
                                    </div>
                                </TableHead>
                                <TableHead className="cursor-pointer select-none hover:bg-muted/50" onClick={() => handleSort('name')}>
                                    <div className="flex items-center gap-1">
                                        Name {sortBy === 'name' && <span className="text-xs">{sortOrder === 'asc' ? '↑' : '↓'}</span>}
                                    </div>
                                </TableHead>
                                <TableHead>Trigger</TableHead>
                                <TableHead>Status</TableHead>
                                <TableHead className="cursor-pointer select-none hover:bg-muted/50" onClick={() => handleSort('execution_count')}>
                                    <div className="flex items-center gap-1">
                                        Executions {sortBy === 'execution_count' && <span className="text-xs">{sortOrder === 'asc' ? '↑' : '↓'}</span>}
                                    </div>
                                </TableHead>
                                <TableHead>Last Executed</TableHead>
                                <TableHead>Created By</TableHead>
                                <TableHead className="text-right">Actions</TableHead>
                            </TableRow>
                        </TableHeader>
                        <TableBody>
                            {loadingData ? (
                                <TableRow>
                                    <TableCell colSpan={8} className="h-24 text-center">
                                        <div className="flex items-center justify-center">
                                            <div className="h-8 w-8 animate-spin rounded-full border-4 border-primary border-t-transparent" />
                                        </div>
                                    </TableCell>
                                </TableRow>
                            ) : workflows.length === 0 ? (
                                <TableRow>
                                    <TableCell colSpan={8} className="h-24 text-center text-muted-foreground">
                                        No workflows found.
                                    </TableCell>
                                </TableRow>
                            ) : (
                                workflows.map((workflow) => (
                                    <TableRow key={workflow.id}>
                                        <TableCell className="font-medium">#{workflow.id}</TableCell>
                                        <TableCell>
                                            <div>
                                                <div className="font-medium">{workflow.name}</div>
                                                {workflow.description && (
                                                    <div className="text-sm text-muted-foreground">{workflow.description}</div>
                                                )}
                                            </div>
                                        </TableCell>
                                        <TableCell>
                                            <Badge variant="outline">
                                                {triggerTypeLabels[workflow.trigger_type] || workflow.trigger_type}
                                            </Badge>
                                        </TableCell>
                                        <TableCell>
                                            <Badge variant={workflow.is_active ? 'default' : 'secondary'}>
                                                {workflow.is_active ? 'Active' : 'Inactive'}
                                            </Badge>
                                        </TableCell>
                                        <TableCell>{workflow.execution_count}</TableCell>
                                        <TableCell>
                                            {workflow.last_executed_at
                                                ? new Date(workflow.last_executed_at).toLocaleString()
                                                : 'Never'}
                                        </TableCell>
                                        <TableCell>{workflow.created_by?.name || '-'}</TableCell>
                                        <TableCell className="text-right">
                                            <DropdownMenu>
                                                <DropdownMenuTrigger asChild>
                                                    <Button variant="ghost" size="sm" disabled={actionLoading === workflow.id}>
                                                        <MoreVertical className="h-4 w-4" />
                                                    </Button>
                                                </DropdownMenuTrigger>
                                                <DropdownMenuContent align="end">
                                                    <DropdownMenuItem onClick={() => navigate(`/admin/workflows/${workflow.id}`)}>
                                                        <Eye className="mr-2 h-4 w-4" />View
                                                    </DropdownMenuItem>
                                                    <DropdownMenuItem onClick={() => navigate(`/admin/workflows/${workflow.id}/edit`)}>
                                                        <Pencil className="mr-2 h-4 w-4" />Edit
                                                    </DropdownMenuItem>
                                                    <DropdownMenuItem onClick={() => handleToggle(workflow)}>
                                                        {workflow.is_active ? (
                                                            <><PauseCircle className="mr-2 h-4 w-4" />Deactivate</>
                                                        ) : (
                                                            <><PlayCircle className="mr-2 h-4 w-4" />Activate</>
                                                        )}
                                                    </DropdownMenuItem>
                                                    <DropdownMenuItem onClick={() => handleDuplicate(workflow)}>
                                                        <Copy className="mr-2 h-4 w-4" />Duplicate
                                                    </DropdownMenuItem>
                                                    <DropdownMenuSeparator />
                                                    <DropdownMenuItem onClick={() => handleDelete(workflow)} className="text-destructive">
                                                        <Trash2 className="mr-2 h-4 w-4" />Delete
                                                    </DropdownMenuItem>
                                                </DropdownMenuContent>
                                            </DropdownMenu>
                                        </TableCell>
                                    </TableRow>
                                ))
                            )}
                        </TableBody>
                    </Table>
                </div>

                {/* Pagination */}
                {total > 0 && (
                    <div className="flex items-center justify-between">
                        <div className="text-sm text-muted-foreground">
                            Showing {from} to {to} of {total} results
                        </div>
                        <div className="flex items-center gap-2">
                            <Button variant="outline" size="sm" onClick={() => setCurrentPage(p => Math.max(1, p - 1))} disabled={currentPage <= 1}>
                                Previous
                            </Button>
                            <span className="text-sm px-2">Page {currentPage} of {lastPage}</span>
                            <Button variant="outline" size="sm" onClick={() => setCurrentPage(p => Math.min(lastPage, p + 1))} disabled={currentPage >= lastPage}>
                                Next
                            </Button>
                        </div>
                    </div>
                )}
            </div>
        </AppLayout>
    );
}
