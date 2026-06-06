import axios from '@/lib/axios';
import { Wallet } from 'lucide-react';
import { useEffect, useState } from 'react';

import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import AppLayout from '@/layouts/app-layout';
import { handleApiError } from '@/lib/toast';

interface PayslipRow {
    id: number;
    month: number;
    year: number;
    gross_salary: string;
    total_deductions: string;
    net_salary: string;
    status: string;
}

export default function MyPayslipsPage() {
    const [payslips, setPayslips] = useState<PayslipRow[]>([]);
    const [loading, setLoading] = useState(true);

    useEffect(() => {
        void loadPayslips();
    }, []);

    const loadPayslips = async () => {
        setLoading(true);
        try {
            const res = await axios.get('/admin/me/payslips');
            setPayslips(res.data.data || []);
        } catch (error) {
            handleApiError(error);
            setPayslips([]);
        } finally {
            setLoading(false);
        }
    };

    const monthName = (m: number) =>
        new Date(2000, m - 1, 1).toLocaleString('en-US', { month: 'long' });

    return (
        <AppLayout breadcrumbs={[{ title: 'My Payslips', href: '/admin/my-payslips' }]}>
            <div className="space-y-6">
                <div className="flex items-center gap-3">
                    <Wallet className="h-8 w-8 text-primary" />
                    <div>
                        <h1 className="text-2xl font-bold">My Payslips</h1>
                        <p className="text-muted-foreground text-sm">View your generated payslips</p>
                    </div>
                </div>

                <Card>
                    <CardHeader>
                        <CardTitle>Payslip History</CardTitle>
                        <CardDescription>Only generated payslips are shown here</CardDescription>
                    </CardHeader>
                    <CardContent>
                        <Table>
                            <TableHeader>
                                <TableRow>
                                    <TableHead>Period</TableHead>
                                    <TableHead>Gross</TableHead>
                                    <TableHead>Deductions</TableHead>
                                    <TableHead>Net</TableHead>
                                    <TableHead>Status</TableHead>
                                </TableRow>
                            </TableHeader>
                            <TableBody>
                                {loading ? (
                                    <TableRow>
                                        <TableCell colSpan={5} className="text-center py-8">
                                            Loading...
                                        </TableCell>
                                    </TableRow>
                                ) : payslips.length === 0 ? (
                                    <TableRow>
                                        <TableCell colSpan={5} className="text-center py-8 text-muted-foreground">
                                            No payslips yet
                                        </TableCell>
                                    </TableRow>
                                ) : (
                                    payslips.map((p) => (
                                        <TableRow key={p.id}>
                                            <TableCell>
                                                {monthName(p.month)} {p.year}
                                            </TableCell>
                                            <TableCell>₹{p.gross_salary}</TableCell>
                                            <TableCell>₹{p.total_deductions}</TableCell>
                                            <TableCell className="font-medium">₹{p.net_salary}</TableCell>
                                            <TableCell>{p.status}</TableCell>
                                        </TableRow>
                                    ))
                                )}
                            </TableBody>
                        </Table>
                    </CardContent>
                </Card>
            </div>
        </AppLayout>
    );
}
