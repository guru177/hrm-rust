import { Link, useNavigate } from 'react-router-dom';
import AppLayout from '@/layouts/app-layout';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import {
    Table,
    TableBody,
    TableCell,
    TableHead,
    TableHeader,
    TableRow,
} from '@/components/ui/table';
import {
    ArrowLeft,
    Pencil,
    PlayCircle,
    PauseCircle,
    Copy,
    Trash2,
} from 'lucide-react';
import { handleApiError, handleApiResponse } from '@/lib/toast';
import axios from '@/lib/axios';
import { useState } from 'react';

interface Action {
    type: string;
    config: Record<string, any>;
}

interface Workflow {
    id: number;
    name: string;
    description: string | null;
    trigger_type: string;
    actions: Action[];
    is_active: boolean;
    execution_count: number;
    last_executed_at: string | null;
    created_by?: {
        id: number;
        name: string;
    };
    created_at: string;
    updated_at: string;
}

interface Props {
    workflow?: Workflow;
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

const actionTypeLabels: Record<string, string> = {
    send_email: 'Send Email',
    assign_to_user: 'Assign to User',
    update_status: 'Update Status',
    create_task: 'Create Task',
    send_notification: 'Send Notification',
    update_field: 'Update Field',
};

export default function View({ workflow = {} as Workflow }: Props) {
    const navigate = useNavigate();
    const [loading, setLoading] = useState(false);

    const breadcrumbs = [
        // { label: 'Dashboard', href: '/admin/dashboard' },
        { label: 'Workflows', href: '/admin/workflows' },
        { label: workflow.name },
    ];

    const handleToggle = async () => {
        setLoading(true);
        try {
            const response = await axios.post(
                `/admin/workflows/${workflow.id}/toggle`,
            );
            handleApiResponse(response);
            window.location.reload();
        } catch (error) {
            handleApiError(error);
        } finally {
            setLoading(false);
        }
    };

    const handleDuplicate = async () => {
        setLoading(true);
        try {
            const response = await axios.post(
                `/admin/workflows/${workflow.id}/duplicate`,
            );
            handleApiResponse(response);
            navigate('/admin/workflows');
        } catch (error) {
            handleApiError(error);
        } finally {
            setLoading(false);
        }
    };

    const handleDelete = async () => {
        if (
            !confirm(
                `Are you sure you want to delete "${workflow.name}"? This action cannot be undone.`,
            )
        ) {
            return;
        }

        setLoading(true);
        try {
            const response = await axios.delete(
                `/admin/workflows/${workflow.id}`,
            );
            handleApiResponse(response);
            navigate('/admin/workflows');
        } catch (error) {
            handleApiError(error);
        } finally {
            setLoading(false);
        }
    };

    const renderActionDetails = (action: Action) => {
        const details: string[] = [];

        Object.entries(action.config).forEach(([key, value]) => {
            details.push(`${key}: ${value}`);
        });

        return details.join(', ') || 'No configuration';
    };

    return (
        <AppLayout breadcrumbs={breadcrumbs}>
            

            <div className="space-y-6">
                {/* Header */}
                <div className="flex items-center justify-between">
                    <div>
                        <h1 className="text-3xl font-bold tracking-tight">
                            {workflow.name}
                        </h1>
                        <p className="text-muted-foreground">
                            {workflow.description || 'No description'}
                        </p>
                    </div>
                    <div className="flex gap-2">
                        <Button
                            variant="outline"
                            onClick={() => navigate('/admin/workflows')}
                        >
                            <ArrowLeft className="mr-2 h-4 w-4" />
                            Back
                        </Button>
                        <Link to={`/admin/workflows/${workflow.id}/edit`}>
                            <Button variant="outline">
                                <Pencil className="mr-2 h-4 w-4" />
                                Edit
                            </Button>
                        </Link>
                        <Button
                            variant="outline"
                            onClick={handleToggle}
                            disabled={loading}
                        >
                            {workflow.is_active ? (
                                <>
                                    <PauseCircle className="mr-2 h-4 w-4" />
                                    Deactivate
                                </>
                            ) : (
                                <>
                                    <PlayCircle className="mr-2 h-4 w-4" />
                                    Activate
                                </>
                            )}
                        </Button>
                        <Button
                            variant="outline"
                            onClick={handleDuplicate}
                            disabled={loading}
                        >
                            <Copy className="mr-2 h-4 w-4" />
                            Duplicate
                        </Button>
                        <Button
                            variant="destructive"
                            onClick={handleDelete}
                            disabled={loading}
                        >
                            <Trash2 className="mr-2 h-4 w-4" />
                            Delete
                        </Button>
                    </div>
                </div>

                <div className="grid gap-6 grid-cols-1 sm:grid-cols-2 lg:grid-cols-3">
                    {/* Status Card */}
                    <Card>
                        <CardHeader>
                            <CardTitle>Status</CardTitle>
                        </CardHeader>
                        <CardContent>
                            <Badge
                                variant={
                                    workflow.is_active ? 'default' : 'secondary'
                                }
                                className="text-lg"
                            >
                                {workflow.is_active ? 'Active' : 'Inactive'}
                            </Badge>
                        </CardContent>
                    </Card>

                    {/* Executions Card */}
                    <Card>
                        <CardHeader>
                            <CardTitle>Executions</CardTitle>
                        </CardHeader>
                        <CardContent>
                            <div className="text-3xl font-bold">
                                {workflow.execution_count}
                            </div>
                            <p className="text-sm text-muted-foreground">
                                Total runs
                            </p>
                        </CardContent>
                    </Card>

                    {/* Last Executed Card */}
                    <Card>
                        <CardHeader>
                            <CardTitle>Last Executed</CardTitle>
                        </CardHeader>
                        <CardContent>
                            <div className="text-lg font-medium">
                                {workflow.last_executed_at
                                    ? new Date(
                                        workflow.last_executed_at,
                                    ).toLocaleString()
                                    : 'Never'}
                            </div>
                        </CardContent>
                    </Card>
                </div>

                {/* Trigger Configuration */}
                <Card>
                    <CardHeader>
                        <CardTitle>Trigger Configuration</CardTitle>
                    </CardHeader>
                    <CardContent>
                        <div className="space-y-2">
                            <div>
                                <span className="text-sm font-medium text-muted-foreground">
                                    Trigger Type:
                                </span>
                                <div className="mt-1">
                                    <Badge variant="outline">
                                        {triggerTypeLabels[
                                            workflow.trigger_type
                                        ] || workflow.trigger_type}
                                    </Badge>
                                </div>
                            </div>
                        </div>
                    </CardContent>
                </Card>

                {/* Actions */}
                <Card>
                    <CardHeader>
                        <CardTitle>
                            Actions ({(workflow.actions || []).length})
                        </CardTitle>
                    </CardHeader>
                    <CardContent>
                        <Table>
                            <TableHeader>
                                <TableRow>
                                    <TableHead>#</TableHead>
                                    <TableHead>Action Type</TableHead>
                                    <TableHead>Configuration</TableHead>
                                </TableRow>
                            </TableHeader>
                            <TableBody>
                                {(workflow.actions || []).map((action, index) => (
                                    <TableRow key={index}>
                                        <TableCell>{index + 1}</TableCell>
                                        <TableCell>
                                            <Badge variant="outline">
                                                {actionTypeLabels[action.type] ||
                                                    action.type}
                                            </Badge>
                                        </TableCell>
                                        <TableCell className="font-mono text-sm">
                                            {renderActionDetails(action)}
                                        </TableCell>
                                    </TableRow>
                                ))}
                            </TableBody>
                        </Table>
                    </CardContent>
                </Card>

                {/* Metadata */}
                <Card>
                    <CardHeader>
                        <CardTitle>Workflow Information</CardTitle>
                    </CardHeader>
                    <CardContent>
                        <div className="grid gap-4 grid-cols-1 sm:grid-cols-2">
                            <div>
                                <span className="text-sm font-medium text-muted-foreground">
                                    Created By
                                </span>
                                <p className="mt-1">
                                    {workflow.created_by?.name || 'Unknown'}
                                </p>
                            </div>
                            <div>
                                <span className="text-sm font-medium text-muted-foreground">
                                    Created At
                                </span>
                                <p className="mt-1">
                                    {new Date(
                                        workflow.created_at,
                                    ).toLocaleString()}
                                </p>
                            </div>
                            <div>
                                <span className="text-sm font-medium text-muted-foreground">
                                    Last Updated
                                </span>
                                <p className="mt-1">
                                    {new Date(
                                        workflow.updated_at,
                                    ).toLocaleString()}
                                </p>
                            </div>
                            <div>
                                <span className="text-sm font-medium text-muted-foreground">
                                    Workflow ID
                                </span>
                                <p className="mt-1">#{workflow.id}</p>
                            </div>
                        </div>
                    </CardContent>
                </Card>
            </div>
        </AppLayout>
    );
}
