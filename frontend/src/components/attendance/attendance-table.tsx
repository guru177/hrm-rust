import axios from '@/lib/axios';
import {
    ChevronLeft,
    ChevronRight,
    ChevronsLeft,
    ChevronsRight,
    Search,
} from 'lucide-react';
import { useState, useEffect } from 'react';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
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
import { handleApiError } from '@/lib/toast';

interface ShiftInfo {
    template_name?: string;
    start_time?: string;
    end_time?: string;
}

interface AttendanceRecord {
    id: number;
    user_id: number;
    user?: {
        id: number;
        name: string;
        email: string;
    };
    date: string;
    clock_in: string;
    clock_out: string;
    duration_minutes: number;
    is_late: boolean;
    is_early_exit: boolean;
    status: string;
    source?: string;
    shift?: ShiftInfo | null;
}

export default function AttendanceTable() {
    const [records, setRecords] = useState<AttendanceRecord[]>([]);
    const [loading, setLoading] = useState(true);
    const [search, setSearch] = useState('');
    const [status, setStatus] = useState('all');
    const [currentPage, setCurrentPage] = useState(1);
    const [lastPage, setLastPage] = useState(1);
    const [total, setTotal] = useState(0);
    const [perPage, setPerPage] = useState(10);
    const [from, setFrom] = useState(0);
    const [to, setTo] = useState(0);

    useEffect(() => {
        fetchRecords();
    }, [search, status, currentPage, perPage]);

    const fetchRecords = async () => {
        setLoading(true);
        try {
            const response = await axios.get('/admin/attendance/list', {
                params: {
                    search,
                    status: status !== 'all' ? status : undefined,
                    page: currentPage,
                    per_page: perPage,
                },
            });

            if (response.data.success) {
                const payload = response.data.data;
                const rows = Array.isArray(payload) ? payload : (payload?.data ?? []);
                setRecords(rows);
                setCurrentPage(payload?.current_page ?? 1);
                setLastPage(payload?.last_page ?? 1);
                setTotal(payload?.total ?? rows.length);
                setFrom(payload?.from ?? (rows.length ? 1 : 0));
                setTo(payload?.to ?? rows.length);
            }
        } catch (error) {
            handleApiError(error);
        } finally {
            setLoading(false);
        }
    };

    const getStatusBadge = (status: string) => {
        const statusMap: Record<string, { variant: any; label: string }> = {
            present: { variant: 'default', label: 'Present' },
            absent: { variant: 'destructive', label: 'Absent' },
            half_day: { variant: 'secondary', label: 'Half Day' },
            leave: { variant: 'outline', label: 'Leave' },
            sick_leave: { variant: 'secondary', label: 'Sick Leave' },
            holiday: { variant: 'outline', label: 'Holiday' },
        };

        const config = statusMap[status] || { variant: 'outline', label: status };
        return <Badge variant={config.variant}>{config.label}</Badge>;
    };

    const formatTime = (time: string) => {
        if (!time) return '--:--';
        return new Date(time).toLocaleTimeString('en-US', {
            hour: '2-digit',
            minute: '2-digit',
            hour12: true,
        });
    };

    const formatShiftTime = (value?: string) => {
        if (!value) return '';
        const part = value.includes('T') ? value.split('T')[1]?.slice(0, 5) : value.slice(0, 5);
        const [h, m] = part.split(':').map(Number);
        if (Number.isNaN(h) || Number.isNaN(m)) return value;
        const d = new Date();
        d.setHours(h, m, 0, 0);
        return d.toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit', hour12: true });
    };

    return (
        <Card>
            <CardHeader>
                <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
                    <CardTitle>Attendance History</CardTitle>
                    <div className="flex flex-col gap-2 sm:flex-row sm:items-center">
                        {/* Search */}
                        <div className="relative w-full sm:w-64">
                            <Search className="absolute left-2.5 top-2.5 h-4 w-4 text-muted-foreground" />
                            <Input
                                placeholder="Search by name or email..."
                                value={search}
                                onChange={(e) => {
                                    setSearch(e.target.value);
                                    setCurrentPage(1);
                                }}
                                className="pl-8"
                            />
                        </div>

                        {/* Status Filter */}
                        <Select
                            value={status}
                            onValueChange={(value) => {
                                setStatus(value);
                                setCurrentPage(1);
                            }}
                        >
                            <SelectTrigger className="w-full sm:w-[140px]">
                                <SelectValue placeholder="Status" />
                            </SelectTrigger>
                            <SelectContent>
                                <SelectItem value="all">All Status</SelectItem>
                                <SelectItem value="present">Present</SelectItem>
                                <SelectItem value="absent">Absent</SelectItem>
                                <SelectItem value="half_day">Half Day</SelectItem>
                                <SelectItem value="leave">Leave</SelectItem>
                                <SelectItem value="sick_leave">Sick Leave</SelectItem>
                            </SelectContent>
                        </Select>

                        {/* Per Page Selector */}
                        <Select
                            value={perPage.toString()}
                            onValueChange={(value) => {
                                setPerPage(parseInt(value));
                                setCurrentPage(1);
                            }}
                        >
                            <SelectTrigger className="w-full sm:w-[100px]">
                                <SelectValue />
                            </SelectTrigger>
                            <SelectContent>
                                <SelectItem value="10">10</SelectItem>
                                <SelectItem value="25">25</SelectItem>
                                <SelectItem value="50">50</SelectItem>
                            </SelectContent>
                        </Select>
                    </div>
                </div>
            </CardHeader>

            <CardContent>
                <div className="rounded-md border">
                    <Table>
                        <TableHeader>
                            <TableRow>
                                <TableHead>Date</TableHead>
                                <TableHead>Employee</TableHead>
                                <TableHead>Clock In</TableHead>
                                <TableHead>Clock Out</TableHead>
                                <TableHead>Shift</TableHead>
                                <TableHead>Duration</TableHead>
                                <TableHead>Status</TableHead>
                                <TableHead>Source</TableHead>
                            </TableRow>
                        </TableHeader>
                        <TableBody>
                            {loading ? (
                                <TableRow>
                                    <TableCell
                                        colSpan={8}
                                        className="text-center py-8"
                                    >
                                        <div className="flex items-center justify-center">
                                            <div className="h-8 w-8 animate-spin rounded-full border-4 border-primary border-t-transparent" />
                                        </div>
                                    </TableCell>
                                </TableRow>
                            ) : records.length === 0 ? (
                                <TableRow>
                                    <TableCell
                                        colSpan={8}
                                        className="text-center py-8 text-muted-foreground"
                                    >
                                        No records found
                                    </TableCell>
                                </TableRow>
                            ) : (
                                records.map((record) => (
                                    <TableRow key={record.id}>
                                        <TableCell className="font-medium">
                                            {new Date(record.date).toLocaleDateString()}
                                        </TableCell>
                                        <TableCell>
                                            <div>
                                                <p className="font-medium">
                                                    {record.user?.name || `User #${record.user_id}`}
                                                </p>
                                                <p className="text-xs text-muted-foreground">
                                                    {record.user?.email || ''}
                                                </p>
                                            </div>
                                        </TableCell>
                                        <TableCell>
                                            <div>
                                                <p>{formatTime(record.clock_in)}</p>
                                                {record.is_late && (
                                                    <p className="text-xs text-red-600">
                                                        Late
                                                    </p>
                                                )}
                                            </div>
                                        </TableCell>
                                        <TableCell>
                                            <div>
                                                <p>{formatTime(record.clock_out)}</p>
                                                {record.is_early_exit && (
                                                    <p className="text-xs text-orange-600">
                                                        Early
                                                    </p>
                                                )}
                                            </div>
                                        </TableCell>
                                        <TableCell>
                                            {record.shift?.template_name ? (
                                                <div>
                                                    <p className="text-sm font-medium">{record.shift.template_name}</p>
                                                    {(record.shift.start_time || record.shift.end_time) && (
                                                        <p className="text-xs text-muted-foreground">
                                                            {formatShiftTime(record.shift.start_time)}
                                                            {record.shift.end_time ? ` – ${formatShiftTime(record.shift.end_time)}` : ''}
                                                        </p>
                                                    )}
                                                </div>
                                            ) : (
                                                <span className="text-muted-foreground text-sm">Default</span>
                                            )}
                                        </TableCell>
                                        <TableCell>
                                            {record.duration_minutes
                                                ? `${Math.floor(record.duration_minutes / 60)}h ${record.duration_minutes % 60}m`
                                                : '--'}
                                        </TableCell>
                                        <TableCell>
                                            {getStatusBadge(record.status)}
                                        </TableCell>
                                        <TableCell>
                                            <Badge variant="outline" className="capitalize">
                                                {record.source || 'manual'}
                                            </Badge>
                                        </TableCell>
                                    </TableRow>
                                ))
                            )}
                        </TableBody>
                    </Table>
                </div>

                {/* Pagination */}
                {!loading && records.length > 0 && (
                    <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between mt-4">
                        <div className="text-sm text-muted-foreground">
                            Showing {from} to {to} of {total} results
                        </div>
                        <div className="flex items-center gap-2">
                            <Button
                                variant="outline"
                                size="sm"
                                onClick={() => setCurrentPage(1)}
                                disabled={currentPage === 1}
                            >
                                <ChevronsLeft className="h-4 w-4" />
                            </Button>
                            <Button
                                variant="outline"
                                size="sm"
                                onClick={() => setCurrentPage(currentPage - 1)}
                                disabled={currentPage === 1}
                            >
                                <ChevronLeft className="h-4 w-4" />
                            </Button>
                            <span className="text-sm">
                                Page {currentPage} of {lastPage}
                            </span>
                            <Button
                                variant="outline"
                                size="sm"
                                onClick={() => setCurrentPage(currentPage + 1)}
                                disabled={currentPage === lastPage}
                            >
                                <ChevronRight className="h-4 w-4" />
                            </Button>
                            <Button
                                variant="outline"
                                size="sm"
                                onClick={() => setCurrentPage(lastPage)}
                                disabled={currentPage === lastPage}
                            >
                                <ChevronsRight className="h-4 w-4" />
                            </Button>
                        </div>
                    </div>
                )}
            </CardContent>
        </Card>
    );
}
