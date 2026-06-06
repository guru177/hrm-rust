// Head removed - use document.title instead
import axios from '@/lib/axios';
import { Users, Plus, UserCheck, UserX, Ban, Shield, Key } from 'lucide-react';
import { useState, useEffect } from 'react';

import RoleTable from '@/components/roles/role-table';
import { StatCard } from '@/components/stat-card';
import { Button } from '@/components/ui/button';
import {
    Dialog,
    DialogContent,
    DialogDescription,
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
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import UserTable from '@/components/users/user-table';
import AppLayout from '@/layouts/app-layout';
import { handleApiError, handleApiResponse } from '@/lib/toast';

export default function UsersIndex() {
    const [refreshKey, setRefreshKey] = useState(0);
    const [activeTab, setActiveTab] = useState('users');
    const [showCreateModal, setShowCreateModal] = useState(false);
    const [creating, setCreating] = useState(false);
    const [createForm, setCreateForm] = useState({
        name: '',
        email: '',
        employee_id: '',
        phone: '',
        password: '',
        password_confirmation: '',
        status: 'active',
    });
    const [errors, setErrors] = useState<Record<string, string[]>>({});
    const [stats, setStats] = useState({
        total: 0,
        active: 0,
        inactive: 0,
        suspended: 0,
    });
    const [roleStats, setRoleStats] = useState({
        total: 0,
    });
    const [roles, setRoles] = useState<any[]>([]);
    const [permissions, setPermissions] = useState<any[]>([]);

    useEffect(() => {
        fetchStats();
        if (activeTab === 'roles') {
            fetchRoles();
            fetchRoleStats();
        }
    }, [refreshKey, activeTab]);

    const fetchStats = async () => {
        try {
            const response = await axios.get('/admin/users/stats');
            if (response.data.success) {
                setStats(response.data.data);
            }
        } catch (error) {
            handleApiError(error);
        }
    };

    const fetchRoleStats = async () => {
        try {
            const response = await axios.get('/admin/roles/stats');
            if (response.data.success) {
                setRoleStats(response.data.data);
            }
        } catch (error) {
            handleApiError(error);
        }
    };

    const fetchRoles = async () => {
        try {
            const response = await axios.get('/admin/roles/list');
            if (response.data.success) {
                setRoles(response.data.data);
            }
        } catch (error) {
            handleApiError(error);
        }
    };

    const fetchPermissions = async () => {
        try {
            const response = await axios.get('/admin/permissions');
            if (response.data.success) {
                setPermissions(response.data.data);
            }
        } catch (error) {
            handleApiError(error);
        }
    };

    useEffect(() => {
        fetchPermissions();
    }, []);

    const breadcrumbs = [
        { title: 'Users', href: '/admin/users' },
    ];

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
                                <Users className="h-6 w-6 text-[#071b3a] dark:text-blue-300" />
                            </div>
                            <div>
                                <h1 className="text-xl font-bold tracking-tight text-[#001f3f] dark:text-white">
                                    Users & Access Management
                                </h1>
                                <p className="text-sm text-[#1e3a5f]/60 dark:text-blue-200/60">
                                    Manage user accounts, roles, and permissions
                                </p>
                            </div>
                        </div>
                        {activeTab === 'users' && (
                            <Button
                                onClick={() => setShowCreateModal(true)}
                                className="shrink-0 bg-gradient-to-r from-[#071b3a] to-[#0d4a8a] hover:from-[#040f22] hover:to-[#0a3272] text-white shadow-md shadow-blue-500/25 dark:shadow-blue-900/40 rounded-xl gap-2"
                            >
                                <Plus className="h-4 w-4" />
                                Add User
                            </Button>
                        )}
                    </div>
                </div>

                <Tabs value={activeTab} onValueChange={setActiveTab}>
                    {/* Glassy pill tabs */}
                    <TabsList className="bg-white/60 dark:bg-white/5 backdrop-blur-sm border border-white/80 dark:border-white/10 shadow-sm rounded-xl h-11 p-1 gap-1">
                        <TabsTrigger
                            value="users"
                            className="gap-2 rounded-lg data-[state=active]:bg-gradient-to-r data-[state=active]:from-[#071b3a] data-[state=active]:to-[#0d4a8a] data-[state=active]:text-white data-[state=active]:shadow-md data-[state=active]:shadow-blue-500/25 dark:data-[state=active]:from-[#2a7fd9] dark:data-[state=active]:to-[#3a9bff] transition-all duration-200"
                        >
                            <Users className="h-4 w-4" />
                            Users
                        </TabsTrigger>
                        <TabsTrigger
                            value="roles"
                            className="gap-2 rounded-lg data-[state=active]:bg-gradient-to-r data-[state=active]:from-[#071b3a] data-[state=active]:to-[#0d4a8a] data-[state=active]:text-white data-[state=active]:shadow-md data-[state=active]:shadow-blue-500/25 dark:data-[state=active]:from-[#2a7fd9] dark:data-[state=active]:to-[#3a9bff] transition-all duration-200"
                        >
                            <Shield className="h-4 w-4" />
                            Roles
                        </TabsTrigger>
                    </TabsList>

                    {/* Users Tab */}
                    <TabsContent value="users" className="space-y-6 mt-4">
                        {/* Stats Cards */}
                        <div className="grid gap-4 grid-cols-1 sm:grid-cols-2 xl:grid-cols-4">
                            <StatCard
                                title="Total Users"
                                value={stats.total}
                                description="All users in the system"
                                icon={Users}
                            />
                            <StatCard
                                title="Active"
                                value={stats.active}
                                description="Currently active"
                                icon={UserCheck}
                                iconClassName="text-emerald-500 dark:text-emerald-400"
                            />
                            <StatCard
                                title="Inactive"
                                value={stats.inactive}
                                description="Currently inactive"
                                icon={UserX}
                                iconClassName="text-slate-500 dark:text-slate-400"
                            />
                            <StatCard
                                title="Suspended"
                                value={stats.suspended}
                                description="Account suspended"
                                icon={Ban}
                                iconClassName="text-red-500 dark:text-red-400"
                            />
                        </div>

                        {/* Users Table */}
                        <UserTable
                            key={refreshKey}
                            onRefresh={() => setRefreshKey((prev) => prev + 1)}
                        />
                    </TabsContent>

                    {/* Roles Tab */}
                    <TabsContent value="roles" className="space-y-6 mt-4">
                        {/* Stats Cards */}
                        <div className="grid gap-4 grid-cols-1 sm:grid-cols-2 xl:grid-cols-4">
                            <StatCard
                                title="Total Roles"
                                value={roleStats.total}
                                description="All roles in the system"
                                icon={Shield}
                            />
                            <StatCard
                                title="Permissions"
                                value={permissions.length}
                                description="Available permissions"
                                icon={Key}
                            />
                        </div>

                        {/* Roles Table */}
                        <RoleTable
                            initialRoles={roles}
                            allPermissions={permissions}
                            onRoleUpdated={() => {
                                fetchRoles();
                                fetchRoleStats();
                            }}
                        />
                    </TabsContent>
                </Tabs>
            </div>


            {/* Create User Modal */}
            <Dialog open={showCreateModal} onOpenChange={setShowCreateModal}>
                <DialogContent className="max-w-2xl">
                    <DialogHeader>
                        <DialogTitle>Create New User</DialogTitle>
                        <DialogDescription>
                            Add a new user account to the system
                        </DialogDescription>
                    </DialogHeader>
                    <form
                        onSubmit={async (e) => {
                            e.preventDefault();
                            setCreating(true);
                            setErrors({});

                            try {
                                const response = await axios.post('/admin/users', createForm);
                                handleApiResponse(response);
                                if (response.data.success) {
                                    setShowCreateModal(false);
                                    setCreateForm({
                                        name: '',
                                        email: '',
                                        employee_id: '',
                                        phone: '',
                                        password: '',
                                        password_confirmation: '',
                                        status: 'active',
                                    });
                                    setRefreshKey((prev) => prev + 1);
                                }
                            } catch (error: any) {
                                if (error.response?.data?.errors) {
                                    setErrors(error.response.data.errors);
                                }
                                handleApiError(error);
                            } finally {
                                setCreating(false);
                            }
                        }}
                        className="space-y-4"
                    >
                        <div className="grid grid-cols-2 gap-4">
                            <div className="space-y-2">
                                <Label htmlFor="name">
                                    Name <span className="text-destructive">*</span>
                                </Label>
                                <Input
                                    id="name"
                                    value={createForm.name}
                                    onChange={(e) =>
                                        setCreateForm({ ...createForm, name: e.target.value })
                                    }
                                    placeholder="Full name"
                                />
                                {errors.name && (
                                    <p className="text-sm text-destructive">{errors.name[0]}</p>
                                )}
                            </div>

                            <div className="space-y-2">
                                <Label htmlFor="email">
                                    Email <span className="text-destructive">*</span>
                                </Label>
                                <Input
                                    id="email"
                                    type="email"
                                    value={createForm.email}
                                    onChange={(e) =>
                                        setCreateForm({ ...createForm, email: e.target.value })
                                    }
                                    placeholder="user@example.com"
                                />
                                {errors.email && (
                                    <p className="text-sm text-destructive">{errors.email[0]}</p>
                                )}
                            </div>

                            <div className="space-y-2">
                                <Label htmlFor="employee_id">Employee ID</Label>
                                <Input
                                    id="employee_id"
                                    value={createForm.employee_id}
                                    onChange={(e) =>
                                        setCreateForm({ ...createForm, employee_id: e.target.value })
                                    }
                                    placeholder="EMP001"
                                />
                                {errors.employee_id && (
                                    <p className="text-sm text-destructive">{errors.employee_id[0]}</p>
                                )}
                            </div>

                            <div className="space-y-2">
                                <Label htmlFor="phone">Phone</Label>
                                <Input
                                    id="phone"
                                    value={createForm.phone}
                                    onChange={(e) =>
                                        setCreateForm({ ...createForm, phone: e.target.value })
                                    }
                                    placeholder="Phone number"
                                />
                                {errors.phone && (
                                    <p className="text-sm text-destructive">{errors.phone[0]}</p>
                                )}
                            </div>

                            <div className="space-y-2">
                                <Label htmlFor="password">
                                    Password <span className="text-destructive">*</span>
                                </Label>
                                <Input
                                    id="password"
                                    type="password"
                                    value={createForm.password}
                                    onChange={(e) =>
                                        setCreateForm({ ...createForm, password: e.target.value })
                                    }
                                    placeholder="••••••••"
                                />
                                {errors.password && (
                                    <p className="text-sm text-destructive">{errors.password[0]}</p>
                                )}
                            </div>

                            <div className="space-y-2">
                                <Label htmlFor="password_confirmation">
                                    Confirm Password <span className="text-destructive">*</span>
                                </Label>
                                <Input
                                    id="password_confirmation"
                                    type="password"
                                    value={createForm.password_confirmation}
                                    onChange={(e) =>
                                        setCreateForm({
                                            ...createForm,
                                            password_confirmation: e.target.value,
                                        })
                                    }
                                    placeholder="••••••••"
                                />
                            </div>

                            <div className="space-y-2">
                                <Label htmlFor="status">Status</Label>
                                <Select
                                    value={createForm.status}
                                    onValueChange={(value) =>
                                        setCreateForm({ ...createForm, status: value })
                                    }
                                >
                                    <SelectTrigger id="status">
                                        <SelectValue placeholder="Select status" />
                                    </SelectTrigger>
                                    <SelectContent>
                                        <SelectItem value="active">Active</SelectItem>
                                        <SelectItem value="inactive">Inactive</SelectItem>
                                        <SelectItem value="suspended">Suspended</SelectItem>
                                    </SelectContent>
                                </Select>
                            </div>
                        </div>

                        <div className="flex justify-end gap-3 pt-4">
                            <Button
                                type="button"
                                variant="outline"
                                onClick={() => setShowCreateModal(false)}
                                disabled={creating}
                            >
                                Cancel
                            </Button>
                            <Button type="submit" disabled={creating}>
                                {creating ? 'Creating...' : 'Create User'}
                            </Button>
                        </div>
                    </form>
                </DialogContent>
            </Dialog>
        </AppLayout>
    );
}
