import { useState, FormEvent } from 'react';
import { useNavigate } from 'react-router-dom';
import AppLayout from '@/layouts/app-layout';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import {
    Select,
    SelectContent,
    SelectItem,
    SelectTrigger,
    SelectValue,
} from '@/components/ui/select';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Switch } from '@/components/ui/switch';
import { Plus, Trash2, ArrowLeft, Network } from 'lucide-react';
import { handleApiError, handleApiResponse } from '@/lib/toast';
import axios from '@/lib/axios';

interface Action {
    type: string;
    config: Record<string, any>;
}

const triggerTypes = [
    { value: 'leave_request_submitted', label: 'Leave Request Submitted' },
    { value: 'leave_request_approved', label: 'Leave Request Approved' },
    { value: 'leave_request_rejected', label: 'Leave Request Rejected' },
    { value: 'attendance_clock_in', label: 'Attendance Clock-In' },
    { value: 'user_created', label: 'User Created' },
    { value: 'task_due', label: 'Task Due' },
    { value: 'time_based', label: 'Time-Based (Scheduled)' },
];

const actionTypes = [
    { value: 'send_email', label: 'Send Email' },
    { value: 'assign_to_user', label: 'Assign to User' },
    { value: 'update_status', label: 'Update Status' },
    { value: 'create_task', label: 'Create Task' },
    { value: 'send_notification', label: 'Send Notification' },
    { value: 'update_field', label: 'Update Field' },
];

export default function Create() {
    const navigate = useNavigate();
    const [name, setName] = useState('');
    const [description, setDescription] = useState('');
    const [triggerType, setTriggerType] = useState('');
    const [actions, setActions] = useState<Action[]>([
        { type: '', config: {} },
    ]);
    const [isActive, setIsActive] = useState(true);
    const [loading, setLoading] = useState(false);
    const [errors, setErrors] = useState<Record<string, string[]>>({});

    const breadcrumbs = [
        { label: 'Dashboard', href: '/admin/dashboard' },
        { label: 'Workflows', href: '/admin/workflows' },
        { label: 'Create Workflow' },
    ];

    const addAction = () => {
        setActions([...actions, { type: '', config: {} }]);
    };

    const removeAction = (index: number) => {
        setActions(actions.filter((_, i) => i !== index));
    };

    const updateAction = (
        index: number,
        field: 'type' | 'config',
        value: any,
    ) => {
        const newActions = [...actions];
        if (field === 'type') {
            newActions[index].type = value;
            newActions[index].config = {}; // Reset config when type changes
        } else {
            newActions[index].config = value;
        }
        setActions(newActions);
    };

    const handleSubmit = async (e: FormEvent) => {
        e.preventDefault();
        setLoading(true);
        setErrors({});

        try {
            const response = await axios.post('/admin/workflows', {
                name,
                description,
                trigger_type: triggerType,
                actions,
                is_active: isActive,
            });

            handleApiResponse(response);
            navigate('/admin/workflows');
        } catch (error: any) {
            if (error.response?.data?.errors) {
                setErrors(error.response.data.errors);
            }
            handleApiError(error);
        } finally {
            setLoading(false);
        }
    };

    const renderActionConfig = (action: Action, index: number) => {
        switch (action.type) {
            case 'send_email':
                return (
                    <div className="space-y-3">
                        <div>
                            <Label>Email Template</Label>
                            <Input
                                placeholder="Welcome Email"
                                value={action.config.template || ''}
                                onChange={(e) =>
                                    updateAction(index, 'config', {
                                        ...action.config,
                                        template: e.target.value,
                                    })
                                }
                            />
                        </div>
                        <div>
                            <Label>Subject</Label>
                            <Input
                                placeholder="Email subject"
                                value={action.config.subject || ''}
                                onChange={(e) =>
                                    updateAction(index, 'config', {
                                        ...action.config,
                                        subject: e.target.value,
                                    })
                                }
                            />
                        </div>
                    </div>
                );
            case 'assign_to_user':
                return (
                    <div>
                        <Label>User ID</Label>
                        <Input
                            type="number"
                            placeholder="User ID to assign to"
                            value={action.config.user_id || ''}
                            onChange={(e) =>
                                updateAction(index, 'config', {
                                    ...action.config,
                                    user_id: e.target.value,
                                })
                            }
                        />
                    </div>
                );
            case 'update_status':
                return (
                    <div>
                        <Label>New Status</Label>
                        <Input
                            placeholder="Status value"
                            value={action.config.status || ''}
                            onChange={(e) =>
                                updateAction(index, 'config', {
                                    ...action.config,
                                    status: e.target.value,
                                })
                            }
                        />
                    </div>
                );
            case 'create_task':
                return (
                    <div className="space-y-3">
                        <div>
                            <Label>Task Title</Label>
                            <Input
                                placeholder="Notify manager on leave approval"
                                value={action.config.title || ''}
                                onChange={(e) =>
                                    updateAction(index, 'config', {
                                        ...action.config,
                                        title: e.target.value,
                                    })
                                }
                            />
                        </div>
                        <div>
                            <Label>Due Date (days from now)</Label>
                            <Input
                                type="number"
                                placeholder="3"
                                value={action.config.due_days || ''}
                                onChange={(e) =>
                                    updateAction(index, 'config', {
                                        ...action.config,
                                        due_days: e.target.value,
                                    })
                                }
                            />
                        </div>
                    </div>
                );
            case 'send_notification':
                return (
                    <div>
                        <Label>Notification Message</Label>
                        <Textarea
                            placeholder="Notification message"
                            value={action.config.message || ''}
                            onChange={(e) =>
                                updateAction(index, 'config', {
                                    ...action.config,
                                    message: e.target.value,
                                })
                            }
                        />
                    </div>
                );
            case 'update_field':
                return (
                    <div className="space-y-3">
                        <div>
                            <Label>Field Name</Label>
                            <Input
                                placeholder="priority"
                                value={action.config.field || ''}
                                onChange={(e) =>
                                    updateAction(index, 'config', {
                                        ...action.config,
                                        field: e.target.value,
                                    })
                                }
                            />
                        </div>
                        <div>
                            <Label>New Value</Label>
                            <Input
                                placeholder="high"
                                value={action.config.value || ''}
                                onChange={(e) =>
                                    updateAction(index, 'config', {
                                        ...action.config,
                                        value: e.target.value,
                                    })
                                }
                            />
                        </div>
                    </div>
                );
            default:
                return null;
        }
    };

    return (
        <AppLayout breadcrumbs={breadcrumbs}>
            

            <div className="space-y-6">
                {/* Header */}
                <div className="relative overflow-hidden rounded-2xl bg-gradient-to-br from-[#e8f2fd] via-[#d0e4f8] to-[#c4d8f0] dark:from-[#0d1e33] dark:via-[#0a1828] dark:to-[#071220] px-4 sm:px-6 py-4 sm:py-5 border border-white/60 dark:border-white/10 shadow-sm">
                    <div className="pointer-events-none absolute -top-12 -right-12 w-56 h-56 opacity-15">
                        <svg viewBox="0 0 200 200" xmlns="http://www.w3.org/2000/svg">
                            <path fill="#071b3a" d="M44.7,-76.4C58.4,-69.7,70.3,-58.6,77.9,-44.9C85.5,-31.2,88.7,-15.6,87.4,-0.8C86,14,80,28,72.1,40.5C64.2,53,54.2,64,42.1,71.3C30,78.6,15,82.3,0.1,82.1C-14.8,81.9,-29.6,77.8,-42.7,70.5C-55.8,63.2,-67.3,52.7,-74.5,39.5C-81.7,26.3,-84.7,10.5,-83.1,-4.9C-81.6,-20.3,-75.5,-35.2,-66.3,-47.4C-57.1,-59.6,-44.8,-69.1,-31.6,-76.1C-18.4,-83.1,-4.6,-87.6,8.2,-86.2C21,-84.8,31,-83.1,44.7,-76.4Z" transform="translate(100 100)" />
                        </svg>
                    </div>
                    <div className="relative flex flex-col sm:flex-row sm:items-center justify-between gap-4">
                        <div className="flex items-center gap-4">
                            <Button
                                variant="outline"
                                size="icon"
                                onClick={() => navigate('/admin/workflows')}
                                className="h-10 w-10 shrink-0 rounded-xl bg-white/50 hover:bg-white border-white/60 dark:bg-slate-900/50 dark:hover:bg-slate-900 dark:border-slate-700 backdrop-blur-sm transition-all"
                            >
                                <ArrowLeft className="h-4 w-4" />
                            </Button>
                            <div className="flex h-10 w-10 sm:h-12 sm:w-12 shrink-0 items-center justify-center rounded-xl bg-[#071b3a]/15 dark:bg-white/10 border border-[#071b3a]/20 dark:border-white/10">
                                <Network className="h-5 w-5 sm:h-6 sm:w-6 text-[#071b3a] dark:text-blue-300" />
                            </div>
                            <div>
                                <h1 className="text-lg sm:text-xl font-bold tracking-tight text-[#001f3f] dark:text-white">
                                    Create Workflow
                                </h1>
                                <p className="text-sm text-[#1e3a5f]/60 dark:text-blue-200/60">
                                    Build custom automation workflows for your business
                                </p>
                            </div>
                        </div>
                    </div>
                </div>

                <form onSubmit={handleSubmit} className="space-y-6">
                    {/* Basic Info */}
                    <Card>
                        <CardHeader>
                            <CardTitle>Basic Information</CardTitle>
                        </CardHeader>
                        <CardContent className="space-y-4">
                            <div>
                                <Label htmlFor="name">
                                    Workflow Name{' '}
                                    <span className="text-destructive">*</span>
                                </Label>
                                <Input
                                    id="name"
                                    value={name}
                                    onChange={(e) => setName(e.target.value)}
                                    placeholder="e.g., Notify on new leave request"
                                />
                                {errors.name && (
                                    <p className="mt-1 text-sm text-destructive">
                                        {errors.name[0]}
                                    </p>
                                )}
                            </div>
                            <div>
                                <Label htmlFor="description">Description</Label>
                                <Textarea
                                    id="description"
                                    value={description}
                                    onChange={(e) =>
                                        setDescription(e.target.value)
                                    }
                                    placeholder="Brief description of what this workflow does"
                                    rows={3}
                                />
                            </div>
                            <div className="flex items-center justify-between">
                                <div>
                                    <Label htmlFor="is_active">
                                        Active Status
                                    </Label>
                                    <p className="text-sm text-muted-foreground">
                                        Enable this workflow to start running
                                    </p>
                                </div>
                                <Switch
                                    id="is_active"
                                    checked={isActive}
                                    onCheckedChange={setIsActive}
                                />
                            </div>
                        </CardContent>
                    </Card>

                    {/* Trigger */}
                    <Card>
                        <CardHeader>
                            <CardTitle>Trigger</CardTitle>
                        </CardHeader>
                        <CardContent>
                            <div>
                                <Label htmlFor="trigger_type">
                                    When should this workflow run?{' '}
                                    <span className="text-destructive">*</span>
                                </Label>
                                <Select
                                    value={triggerType}
                                    onValueChange={setTriggerType}
                                >
                                    <SelectTrigger id="trigger_type">
                                        <SelectValue placeholder="Select a trigger" />
                                    </SelectTrigger>
                                    <SelectContent>
                                        {triggerTypes.map((trigger) => (
                                            <SelectItem
                                                key={trigger.value}
                                                value={trigger.value}
                                            >
                                                {trigger.label}
                                            </SelectItem>
                                        ))}
                                    </SelectContent>
                                </Select>
                                {errors.trigger_type && (
                                    <p className="mt-1 text-sm text-destructive">
                                        {errors.trigger_type[0]}
                                    </p>
                                )}
                            </div>
                        </CardContent>
                    </Card>

                    {/* Actions */}
                    <Card>
                        <CardHeader>
                            <div className="flex items-center justify-between">
                                <CardTitle>Actions</CardTitle>
                                <Button
                                    type="button"
                                    variant="outline"
                                    size="sm"
                                    onClick={addAction}
                                >
                                    <Plus className="mr-2 h-4 w-4" />
                                    Add Action
                                </Button>
                            </div>
                        </CardHeader>
                        <CardContent className="space-y-4">
                            {actions.map((action, index) => (
                                <div
                                    key={index}
                                    className="space-y-3 rounded-lg border p-4"
                                >
                                    <div className="flex items-start justify-between">
                                        <Badge variant="outline">
                                            Action {index + 1}
                                        </Badge>
                                        {actions.length > 1 && (
                                            <Button
                                                type="button"
                                                variant="ghost"
                                                size="sm"
                                                onClick={() =>
                                                    removeAction(index)
                                                }
                                            >
                                                <Trash2 className="h-4 w-4 text-destructive" />
                                            </Button>
                                        )}
                                    </div>
                                    <div>
                                        <Label>Action Type</Label>
                                        <Select
                                            value={action.type}
                                            onValueChange={(value) =>
                                                updateAction(
                                                    index,
                                                    'type',
                                                    value,
                                                )
                                            }
                                        >
                                            <SelectTrigger>
                                                <SelectValue placeholder="Select an action" />
                                            </SelectTrigger>
                                            <SelectContent>
                                                {actionTypes.map((type) => (
                                                    <SelectItem
                                                        key={type.value}
                                                        value={type.value}
                                                    >
                                                        {type.label}
                                                    </SelectItem>
                                                ))}
                                            </SelectContent>
                                        </Select>
                                        {errors[`actions.${index}.type`] && (
                                            <p className="mt-1 text-sm text-destructive">
                                                {
                                                    errors[
                                                    `actions.${index}.type`
                                                    ][0]
                                                }
                                            </p>
                                        )}
                                    </div>
                                    {action.type &&
                                        renderActionConfig(action, index)}
                                </div>
                            ))}
                            {errors.actions && (
                                <p className="text-sm text-destructive">
                                    {errors.actions[0]}
                                </p>
                            )}
                        </CardContent>
                    </Card>

                    {/* Submit */}
                    <div className="flex justify-end gap-3">
                        <Button
                            type="button"
                            variant="outline"
                            onClick={() => navigate('/admin/workflows')}
                        >
                            Cancel
                        </Button>
                        <Button type="submit" disabled={loading}>
                            {loading ? 'Creating...' : 'Create Workflow'}
                        </Button>
                    </div>
                </form>
            </div>
        </AppLayout>
    );
}
