import axios from '@/lib/axios';
import { BarChart3 } from 'lucide-react';
import { useEffect, useState } from 'react';

import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import AppLayout from '@/layouts/app-layout';
import { handleApiError } from '@/lib/toast';

export default function ReportsPage() {
    const now = new Date();
    const [month, setMonth] = useState(now.getMonth() + 1);
    const [year, setYear] = useState(now.getFullYear());
    const [attendance, setAttendance] = useState<any[]>([]);
    const [payroll, setPayroll] = useState<any[]>([]);
    const [payrollSplit, setPayrollSplit] = useState<any[]>([]);
    const [leave, setLeave] = useState<any[]>([]);
    const [loading, setLoading] = useState(false);

    useEffect(() => {
        void loadReports();
    }, [month, year]);

    const loadReports = async () => {
        setLoading(true);
        try {
            const [att, pay, split, lev] = await Promise.all([
                axios.get('/admin/reports/attendance-summary', { params: { month, year } }),
                axios.get('/admin/reports/payroll-register', { params: { month, year } }),
                axios.get('/admin/reports/payroll-split', { params: { month, year } }),
                axios.get('/admin/reports/leave-balance'),
            ]);
            setAttendance(att.data.data?.employees || []);
            setPayroll(pay.data.data?.payslips || []);
            setPayrollSplit(split.data.data?.rows || []);
            setLeave(lev.data.data || []);
        } catch (error) {
            handleApiError(error);
        } finally {
            setLoading(false);
        }
    };

    return (
        <AppLayout breadcrumbs={[{ title: 'Reports', href: '/admin/reports' }]}>
            <div className="space-y-6">
                <div className="flex items-center gap-3">
                    <BarChart3 className="h-8 w-8 text-primary" />
                    <div>
                        <h1 className="text-2xl font-bold">Reports</h1>
                        <p className="text-muted-foreground text-sm">Attendance, payroll, and leave summaries</p>
                    </div>
                </div>

                <div className="flex gap-4 items-end">
                    <div className="space-y-2">
                        <Label>Month</Label>
                        <Input type="number" min={1} max={12} value={month} onChange={(e) => setMonth(Number(e.target.value))} className="w-24" />
                    </div>
                    <div className="space-y-2">
                        <Label>Year</Label>
                        <Input type="number" value={year} onChange={(e) => setYear(Number(e.target.value))} className="w-28" />
                    </div>
                </div>

                <Tabs defaultValue="attendance">
                    <TabsList>
                        <TabsTrigger value="attendance">Attendance</TabsTrigger>
                        <TabsTrigger value="payroll">Payroll Register</TabsTrigger>
                        <TabsTrigger value="split">Salary Split</TabsTrigger>
                        <TabsTrigger value="leave">Leave Balance</TabsTrigger>
                    </TabsList>

                    <TabsContent value="attendance">
                        <Card>
                            <CardHeader><CardTitle>Attendance Summary</CardTitle></CardHeader>
                            <CardContent>
                                <Table>
                                    <TableHeader>
                                        <TableRow>
                                            <TableHead>Employee</TableHead>
                                            <TableHead>Present Days</TableHead>
                                            <TableHead>Late</TableHead>
                                            <TableHead>Early Exit</TableHead>
                                        </TableRow>
                                    </TableHeader>
                                    <TableBody>
                                        {loading ? (
                                            <TableRow><TableCell colSpan={4} className="text-center">Loading...</TableCell></TableRow>
                                        ) : attendance.map((r) => (
                                            <TableRow key={r.user_id}>
                                                <TableCell>{r.name}</TableCell>
                                                <TableCell>{r.present_days}</TableCell>
                                                <TableCell>{r.late_marks}</TableCell>
                                                <TableCell>{r.early_exits}</TableCell>
                                            </TableRow>
                                        ))}
                                    </TableBody>
                                </Table>
                            </CardContent>
                        </Card>
                    </TabsContent>

                    <TabsContent value="payroll">
                        <Card>
                            <CardHeader><CardTitle>Payroll Register</CardTitle></CardHeader>
                            <CardContent>
                                <Table>
                                    <TableHeader>
                                        <TableRow>
                                            <TableHead>Employee</TableHead>
                                            <TableHead>LOP</TableHead>
                                            <TableHead>Penalty</TableHead>
                                            <TableHead>Gross</TableHead>
                                            <TableHead>Net</TableHead>
                                            <TableHead>Status</TableHead>
                                        </TableRow>
                                    </TableHeader>
                                    <TableBody>
                                        {payroll.map((r) => (
                                            <TableRow key={r.payslip_id}>
                                                <TableCell>{r.name}</TableCell>
                                                <TableCell>₹{(r.lop_deduction ?? 0).toFixed?.(2) ?? r.lop_deduction}</TableCell>
                                                <TableCell>₹{(r.shift_penalty ?? 0).toFixed?.(2) ?? r.shift_penalty}</TableCell>
                                                <TableCell>₹{r.gross_salary?.toFixed?.(2) ?? r.gross_salary}</TableCell>
                                                <TableCell>₹{r.net_salary?.toFixed?.(2) ?? r.net_salary}</TableCell>
                                                <TableCell>{r.status}</TableCell>
                                            </TableRow>
                                        ))}
                                    </TableBody>
                                </Table>
                            </CardContent>
                        </Card>
                    </TabsContent>

                    <TabsContent value="split">
                        <Card>
                            <CardHeader><CardTitle>Salary Split (CTC / LOP / Statutory)</CardTitle></CardHeader>
                            <CardContent className="overflow-x-auto">
                                <Table>
                                    <TableHeader>
                                        <TableRow>
                                            <TableHead>Employee</TableHead>
                                            <TableHead>Yearly CTC</TableHead>
                                            <TableHead>Basic</TableHead>
                                            <TableHead>HRA</TableHead>
                                            <TableHead>Conv</TableHead>
                                            <TableHead>Gross</TableHead>
                                            <TableHead>LOP</TableHead>
                                            <TableHead>PF</TableHead>
                                            <TableHead>ESI</TableHead>
                                            <TableHead>Prof Tax</TableHead>
                                            <TableHead>Net</TableHead>
                                        </TableRow>
                                    </TableHeader>
                                    <TableBody>
                                        {payrollSplit.map((r) => (
                                            <TableRow key={r.user_id}>
                                                <TableCell>{r.name}</TableCell>
                                                <TableCell>{r.yearly_ctc ? `₹${Number(r.yearly_ctc).toLocaleString('en-IN')}` : '—'}</TableCell>
                                                <TableCell>{r.basic != null ? `₹${Number(r.basic).toFixed(0)}` : '—'}</TableCell>
                                                <TableCell>{r.hra != null ? `₹${Number(r.hra).toFixed(0)}` : '—'}</TableCell>
                                                <TableCell>{r.conveyance != null ? `₹${Number(r.conveyance).toFixed(0)}` : '—'}</TableCell>
                                                <TableCell>{r.gross_salary != null ? `₹${Number(r.gross_salary).toFixed(0)}` : '—'}</TableCell>
                                                <TableCell>{r.lop_deduction != null ? `₹${Number(r.lop_deduction).toFixed(0)}` : '—'}</TableCell>
                                                <TableCell>{r.pf_deduction != null ? `₹${Number(r.pf_deduction).toFixed(0)}` : '—'}</TableCell>
                                                <TableCell>{r.esi_deduction != null ? `₹${Number(r.esi_deduction).toFixed(0)}` : '—'}</TableCell>
                                                <TableCell>{r.prof_tax != null ? `₹${Number(r.prof_tax).toFixed(0)}` : '—'}</TableCell>
                                                <TableCell className="font-medium">{r.net_salary != null ? `₹${Number(r.net_salary).toFixed(0)}` : '—'}</TableCell>
                                            </TableRow>
                                        ))}
                                    </TableBody>
                                </Table>
                            </CardContent>
                        </Card>
                    </TabsContent>

                    <TabsContent value="leave">
                        <Card>
                            <CardHeader><CardTitle>Leave Balance</CardTitle></CardHeader>
                            <CardContent>
                                <Table>
                                    <TableHeader>
                                        <TableRow>
                                            <TableHead>Employee</TableHead>
                                            <TableHead>Quota</TableHead>
                                            <TableHead>Used</TableHead>
                                            <TableHead>Pending</TableHead>
                                            <TableHead>Available</TableHead>
                                        </TableRow>
                                    </TableHeader>
                                    <TableBody>
                                        {leave.map((r) => (
                                            <TableRow key={r.user_id}>
                                                <TableCell>{r.name}</TableCell>
                                                <TableCell>{r.annual_quota}</TableCell>
                                                <TableCell>{r.used_days}</TableCell>
                                                <TableCell>{r.pending_days ?? 0}</TableCell>
                                                <TableCell>{r.available_days ?? r.balance}</TableCell>
                                            </TableRow>
                                        ))}
                                    </TableBody>
                                </Table>
                            </CardContent>
                        </Card>
                    </TabsContent>
                </Tabs>
            </div>
        </AppLayout>
    );
}
