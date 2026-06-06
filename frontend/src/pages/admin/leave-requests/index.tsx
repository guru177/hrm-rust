import { useNavigate } from 'react-router-dom';
import axios from '@/lib/axios';
import { Calendar, Clock, FileText, Plus } from 'lucide-react';
import { useState, useEffect } from 'react';

import LeaveRequestForm from '@/components/leave-requests/leave-request-form';
import LeaveRequestTable from '@/components/leave-requests/leave-request-table';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogHeader,
    DialogTitle,
} from '@/components/ui/dialog';
import { usePermissions } from '@/hooks/use-permissions';
import AppLayout from '@/layouts/app-layout';
import { handleApiError, handleApiResponse } from '@/lib/toast';

export default function LeaveRequestsPage() {
    const navigate = useNavigate();
    const { hasRole, hasPermission } = usePermissions();
    const [stats, setStats] = useState<any>(null);
    const [loading, setLoading] = useState(true);
    const [showForm, setShowForm] = useState(false);
    const [refreshKey, setRefreshKey] = useState(0);

    useEffect(() => {
        loadStats();
    }, []);

    const loadStats = async () => {
        setLoading(true);
        try {
            const response = await axios.get('/admin/leave-requests/stats');
            setStats(response.data.data);
        } catch (error) {
            handleApiError(error);
        } finally {
            setLoading(false);
        }
    };

    const handleRequestSubmitted = () => {
        setShowForm(false);
        setRefreshKey((prev) => prev + 1);
        loadStats();
    };

    const breadcrumbs = [
        { label: 'Attendance', href: '/admin/attendance' },
        { label: 'Leave Requests', href: '/admin/leave-requests' },
    ];

    if (loading) {
        return (
            <AppLayout breadcrumbs={breadcrumbs}>
                
                <div className="flex items-center justify-center min-h-96">
                    <div className="h-8 w-8 animate-spin rounded-full border-4 border-primary border-t-transparent" />
                </div>
            </AppLayout>
        );
    }

    return (
        <AppLayout breadcrumbs={breadcrumbs}>
            

            <div className="space-y-6">
                {/* Header */}
                <div className="flex items-center justify-between">
                    <div>
                        <h1 className="text-3xl font-bold tracking-tight flex items-center gap-2">
                            <FileText className="h-8 w-8 text-primary" />
                            Leave Requests
                        </h1>
                        <p className="text-muted-foreground mt-1">
                            Submit and manage your leave requests
                        </p>
                    </div>
                    <div className="flex items-center gap-3">
                        {(hasPermission('manage-leave-requests') || hasRole('admin')) && (
                            <Button
                                variant="outline"
                                onClick={() => navigate('/admin/leave-requests/manage')}
                            >
                                Manage Requests
                            </Button>
                        )}
                        <Button onClick={() => setShowForm(true)}>
                            <Plus className="h-4 w-4" />
                            New Leave Request
                        </Button>
                    </div>
                </div>

                {/* Stats Cards */}
                <div className="grid gap-4 grid-cols-1 sm:grid-cols-2 xl:grid-cols-4">
                    <Card>
                        <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                            <CardTitle className="text-sm font-medium">Total Requests</CardTitle>
                            <FileText className="h-4 w-4 text-muted-foreground" />
                        </CardHeader>
                        <CardContent>
                            <div className="text-2xl font-bold">{stats?.total_requests || 0}</div>
                        </CardContent>
                    </Card>

                    <Card>
                        <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                            <CardTitle className="text-sm font-medium">Pending</CardTitle>
                            <Clock className="h-4 w-4 text-orange-500" />
                        </CardHeader>
                        <CardContent>
                            <div className="text-2xl font-bold">{stats?.pending || 0}</div>
                        </CardContent>
                    </Card>

                    <Card>
                        <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                            <CardTitle className="text-sm font-medium">Approved</CardTitle>
                            <Calendar className="h-4 w-4 text-green-500" />
                        </CardHeader>
                        <CardContent>
                            <div className="text-2xl font-bold">{stats?.approved || 0}</div>
                        </CardContent>
                    </Card>

                    <Card>
                        <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                            <CardTitle className="text-sm font-medium">Leave Days Used</CardTitle>
                            <Calendar className="h-4 w-4 text-blue-500" />
                        </CardHeader>
                        <CardContent>
                            <div className="text-2xl font-bold">{stats?.total_leave_days || 0}</div>
                        </CardContent>
                    </Card>
                </div>

                {/* Requests Table */}
                <LeaveRequestTable
                    key={refreshKey}
                    onRefresh={() => {
                        setRefreshKey((prev) => prev + 1);
                        loadStats();
                    }}
                />
            </div>

            {/* New Request Dialog */}
            <Dialog open={showForm} onOpenChange={setShowForm}>
                <DialogContent className="max-w-2xl">
                    <DialogHeader>
                        <DialogTitle>New Leave Request</DialogTitle>
                        <DialogDescription>
                            Fill in the details below to submit a leave request
                        </DialogDescription>
                    </DialogHeader>
                    <LeaveRequestForm
                        onSuccess={handleRequestSubmitted}
                        onCancel={() => setShowForm(false)}
                    />
                </DialogContent>
            </Dialog>
        </AppLayout>
    );
}
