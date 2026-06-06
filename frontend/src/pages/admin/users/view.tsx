import { useNavigate } from 'react-router-dom';
import axios from '@/lib/axios';
import {
    User as UserIcon,
    Mail,
    Phone,
    Calendar,
    Briefcase,
    Building2,
    ArrowLeft,
    Edit,
    Save,
    X,
    Shield,
    CreditCard,
    IdCard,
    Users,
    Upload,
    Camera,
    Banknote,
} from 'lucide-react';
import { useState, useRef, useEffect } from 'react';
import { useParams } from 'react-router-dom';

import { Avatar, AvatarFallback, AvatarImage } from '@/components/ui/avatar';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Separator } from '@/components/ui/separator';
import AppLayout from '@/layouts/app-layout';
import { handleApiResponse, handleApiError } from '@/lib/toast';

interface User {
    id: number;
    name: string;
    email: string;
    employee_id?: string;
    phone?: string;
    department_id?: number;
    designation_id?: number;
    department?: {
        id: number;
        name: string;
        slug: string;
    };
    designation?: {
        id: number;
        name: string;
        level: string;
    };
    status: string;
    created_at: string;
    updated_at: string;
    photo?: string;
    reporting_manager_id?: number;
    account_number?: string;
    ifsc_code?: string;
    bank_name?: string;
    pan_number?: string;
    esi_number?: string;
    pf_number?: string;
    aadhar_number?: string;
    date_of_joining?: string;
    date_of_exit?: string;
    reporting_manager?: {
        id: number;
        name: string;
    };
}

interface Manager {
    id: number;
    name: string;
    designation_id?: number;
    designation?: {
        id: number;
        name: string;
        level: string;
    };
}

interface Department {
    id: number;
    name: string;
}

interface Designation {
    id: number;
    name: string;
    level: string;
}

interface SalaryStructure {
    id: number;
    user_id: number;
    basic_salary: string;
    hra: string;
    transport_allowance: string;
    other_allowances: string;
    pf_deduction: string;
    esi_deduction: string;
    tds: string;
    effective_from: string;
    gross_salary?: number;
    total_deductions?: number;
    net_salary?: number;
}

interface Props {
    user?: User;
    availableManagers?: Manager[];
    departments?: Department[];
    designations?: Designation[];
    salaryStructure?: SalaryStructure | null;
}

export default function ViewUser() {
    const navigate = useNavigate();
    const { id } = useParams();
    const [user, setUser] = useState<User | null>(null);
    const [availableManagers, setAvailableManagers] = useState<Manager[]>([]);
    const [departments, setDepartments] = useState<Department[]>([]);
    const [designations, setDesignations] = useState<Designation[]>([]);
    
    const [isEditing, setIsEditing] = useState(false);
    const [salaryStructure, setSalaryStructure] = useState<SalaryStructure | null>(null);
    const [isSalaryEditing, setIsSalaryEditing] = useState(false);
    const [savingSalary, setSavingSalary] = useState(false);
    const [loading, setLoading] = useState(true);

    const [salaryForm, setSalaryForm] = useState({
        basic_salary: '',
        hra: '',
        transport_allowance: '',
        other_allowances: '',
        pf_deduction: '',
        esi_deduction: '',
        tds: '',
        effective_from: new Date().toISOString().split('T')[0],
    });
    
    const [photoPreview, setPhotoPreview] = useState<string | null>(null);
    const fileInputRef = useRef<HTMLInputElement>(null);
    const [editForm, setEditForm] = useState({
        name: '',
        email: '',
        employee_id: '',
        phone: '',
        department_id: '',
        designation_id: '',
        reporting_manager_id: '',
        account_number: '',
        ifsc_code: '',
        bank_name: '',
        pan_number: '',
        esi_number: '',
        pf_number: '',
        aadhar_number: '',
        date_of_joining: '',
        date_of_exit: '',
        password: '',
        photo: null as File | null,
    });
    const [updating, setUpdating] = useState(false);

    useEffect(() => {
        const fetchData = async () => {
            try {
                const [userRes, managersRes, deptsRes, desigsRes] = await Promise.all([
                    axios.get(`/admin/users/${id}`),
                    axios.get('/admin/users/list'),
                    axios.get('/admin/departments/list'),
                    axios.get('/admin/designations/list')
                ]);
                
                const userData = userRes.data.data;
                setUser(userData);
                
                setEditForm({
                    name: userData.name,
                    email: userData.email,
                    employee_id: userData.employee_id || '',
                    phone: userData.phone || '',
                    department_id: userData.department_id?.toString() || '',
                    designation_id: userData.designation_id?.toString() || '',
                    reporting_manager_id: userData.reporting_manager_id?.toString() || '',
                    account_number: userData.account_number || '',
                    ifsc_code: userData.ifsc_code || '',
                    bank_name: userData.bank_name || '',
                    pan_number: userData.pan_number || '',
                    esi_number: userData.esi_number || '',
                    pf_number: userData.pf_number || '',
                    aadhar_number: userData.aadhar_number || '',
                    date_of_joining: userData.date_of_joining || '',
                    date_of_exit: userData.date_of_exit || '',
                    password: '',
                    photo: null,
                });
                
                setPhotoPreview(
                    userData.photo
                        ? userData.photo.startsWith('http')
                            ? userData.photo
                            : `/storage/${userData.photo}`
                        : null
                );
                
                setAvailableManagers(managersRes.data.data);
                setDepartments(deptsRes.data.data);
                setDesignations(desigsRes.data.data);
            } catch (error) {
                console.error('Failed to load user data:', error);
                handleApiError(error);
                navigate('/admin/users');
            } finally {
                setLoading(false);
            }
        };
        if (id) fetchData();
    }, [id, navigate]);

    const handlePhotoChange = (e: React.ChangeEvent<HTMLInputElement>) => {
        const file = e.target.files?.[0];
        if (file) {
            setEditForm({ ...editForm, photo: file });
            const reader = new FileReader();
            reader.onloadend = () => {
                setPhotoPreview(reader.result as string);
            };
            reader.readAsDataURL(file);
        }
    };

    const handleSaveEdit = async () => {
        setUpdating(true);
        try {
            const formData = new FormData();
            formData.append('name', editForm.name);
            formData.append('email', editForm.email);
            formData.append('employee_id', editForm.employee_id || '');
            formData.append('phone', editForm.phone || '');
            formData.append('department_id', editForm.department_id || '');
            formData.append('designation_id', editForm.designation_id || '');
            formData.append('reporting_manager_id', editForm.reporting_manager_id || '');
            formData.append('account_number', editForm.account_number || '');
            formData.append('ifsc_code', editForm.ifsc_code || '');
            formData.append('bank_name', editForm.bank_name || '');
            formData.append('pan_number', editForm.pan_number || '');
            formData.append('esi_number', editForm.esi_number || '');
            formData.append('pf_number', editForm.pf_number || '');
            formData.append('aadhar_number', editForm.aadhar_number || '');
            formData.append('date_of_joining', editForm.date_of_joining || '');
            formData.append('date_of_exit', editForm.date_of_exit || '');

            if (editForm.password) {
                formData.append('password', editForm.password);
            }

            if (editForm.photo) {
                formData.append('photo', editForm.photo);
            }

            const response = await axios.post(`/admin/users/${user.id}`, formData);

            handleApiResponse(response);
            if (response.data.success) {
                setUser(response.data.data);
                setIsEditing(false);
                setEditForm({ ...editForm, password: '', photo: null });
                if (response.data.data.photo) {
                    const photo = response.data.data.photo;
                    setPhotoPreview(
                        photo.startsWith('http') ? photo : `/storage/${photo}`
                    );
                }
            }
        } catch (error) {
            handleApiError(error);
        } finally {
            setUpdating(false);
        }
    };

    const handleCancelEdit = () => {
        if (!user) return;
        setEditForm({
            name: user.name,
            email: user.email,
            employee_id: user.employee_id || '',
            phone: user.phone || '',
            department_id: user.department_id?.toString() || '',
            designation_id: user.designation_id?.toString() || '',
            reporting_manager_id: user.reporting_manager_id?.toString() || '',
            account_number: user.account_number || '',
            ifsc_code: user.ifsc_code || '',
            bank_name: user.bank_name || '',
            pan_number: user.pan_number || '',
            esi_number: user.esi_number || '',
            pf_number: user.pf_number || '',
            aadhar_number: user.aadhar_number || '',
            date_of_joining: user.date_of_joining || '',
            date_of_exit: user.date_of_exit || '',
            password: '',
            photo: null,
        });
        setPhotoPreview(
            user.photo
                ? user.photo.startsWith('http')
                    ? user.photo
                    : `/storage/${user.photo}`
                : null
        );
        setIsEditing(false);
    };

    const handleStatusChange = async (status: string) => {
        if (!user) return;
        setUpdating(true);
        try {
            const response = await axios.put(`/admin/users/${user.id}`, { status });
            handleApiResponse(response);
            if (response.data.success) {
                setUser(response.data.data);
            }
        } catch (error) {
            handleApiError(error);
        } finally {
            setUpdating(false);
        }
    };

    const handleSaveSalary = async () => {
        if (!user) return;
        setSavingSalary(true);
        try {
            const response = await axios.post(`/admin/users/${user.id}/salary-structure`, salaryForm);
            handleApiResponse(response);
            if (response.data.success) {
                setSalaryStructure(response.data.data);
                setIsSalaryEditing(false);
            }
        } catch (error) {
            handleApiError(error);
        } finally {
            setSavingSalary(false);
        }
    };

    const getStatusBadge = (status: string) => {
        const variants: Record<string, any> = {
            active: { variant: 'success', label: 'Active' },
            inactive: { variant: 'secondary', label: 'Inactive' },
            suspended: { variant: 'destructive', label: 'Suspended' },
        };
        const config = variants[status] || variants.active;
        return <Badge variant={config.variant}>{config.label}</Badge>;
    };

    if (loading) {
        return (
            <AppLayout breadcrumbs={[{ title: 'Users', href: '/admin/users' }, { title: 'Loading...', href: '' }]}>
                <div className="flex h-[400px] items-center justify-center">
                    <div className="h-8 w-8 animate-spin rounded-full border-4 border-primary border-t-transparent"></div>
                </div>
            </AppLayout>
        );
    }

    if (!user) {
        return (
            <AppLayout breadcrumbs={[{ title: 'Users', href: '/admin/users' }, { title: 'Not Found', href: '' }]}>
                <div className="flex h-[400px] flex-col items-center justify-center gap-4">
                    <h2 className="text-xl font-semibold">User not found</h2>
                    <Button onClick={() => navigate('/admin/users')}>Back to Users</Button>
                </div>
            </AppLayout>
        );
    }

    const breadcrumbs = [
        { title: 'Users', href: '/admin/users' },
        { title: user.name, href: '' },
    ];

    return (
        <AppLayout breadcrumbs={breadcrumbs}>

            <div className="space-y-6">
                {/* Header */}
                <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
                    <div className="flex items-center gap-3">
                        <Button
                            variant="outline"
                            size="icon"
                            onClick={() => navigate('/admin/users')}
                        >
                            <ArrowLeft className="h-4 w-4" />
                        </Button>
                        <div>
                            <h1 className="text-3xl font-bold tracking-tight">{user.name}</h1>
                            <p className="text-muted-foreground">
                                User #{user.id} • Created{' '}
                                {new Date(user.created_at).toLocaleDateString()}
                            </p>
                        </div>
                    </div>
                    <div className="flex gap-2">
                        {isEditing ? (
                            <>
                                <Button onClick={handleSaveEdit} disabled={updating}>
                                    <Save className="mr-2 h-4 w-4" />
                                    {updating ? 'Saving...' : 'Save'}
                                </Button>
                                <Button variant="outline" onClick={handleCancelEdit} disabled={updating}>
                                    <X className="mr-2 h-4 w-4" />
                                    Cancel
                                </Button>
                            </>
                        ) : (
                            <Button variant="outline" onClick={() => setIsEditing(true)}>
                                <Edit className="mr-2 h-4 w-4" />
                                Edit
                            </Button>
                        )}
                    </div>
                </div>

                <div className="grid gap-6 grid-cols-1 sm:grid-cols-2 lg:grid-cols-3">
                    {/* Main Info */}
                    <div className="md:col-span-2 space-y-6">
                        {/* Personal Information */}
                        <Card>
                            <CardHeader>
                                <CardTitle className="flex items-center gap-2">
                                    <UserIcon className="h-5 w-5" />
                                    Personal Information
                                </CardTitle>
                                <CardDescription>
                                    Basic account and contact information
                                </CardDescription>
                            </CardHeader>
                            <CardContent className="space-y-6">
                                {/* Photo Upload */}
                                <div className="flex items-center gap-6">
                                    <Avatar className="h-24 w-24">
                                        <AvatarImage src={photoPreview || ''} alt={user.name} />
                                        <AvatarFallback className="text-2xl">
                                            {user.name.charAt(0).toUpperCase()}
                                        </AvatarFallback>
                                    </Avatar>
                                    {isEditing && (
                                        <div>
                                            <input
                                                type="file"
                                                ref={fileInputRef}
                                                onChange={handlePhotoChange}
                                                accept="image/*"
                                                className="hidden"
                                            />
                                            <Button
                                                variant="outline"
                                                size="sm"
                                                onClick={() => fileInputRef.current?.click()}
                                            >
                                                <Camera className="mr-2 h-4 w-4" />
                                                Change Photo
                                            </Button>
                                            <p className="text-xs text-muted-foreground mt-2">
                                                JPG, PNG or GIF (Max 2MB)
                                            </p>
                                        </div>
                                    )}
                                </div>

                                <Separator />

                                <div className="grid gap-4 sm:grid-cols-2">
                                    <div className="space-y-2">
                                        <Label className="text-sm text-muted-foreground">Full Name</Label>
                                        {isEditing ? (
                                            <Input
                                                value={editForm.name}
                                                onChange={(e) =>
                                                    setEditForm({ ...editForm, name: e.target.value })
                                                }
                                            />
                                        ) : (
                                            <p className="font-medium">{user.name}</p>
                                        )}
                                    </div>

                                    <div className="space-y-2">
                                        <Label className="text-sm text-muted-foreground">Email</Label>
                                        {isEditing ? (
                                            <Input
                                                type="email"
                                                value={editForm.email}
                                                onChange={(e) =>
                                                    setEditForm({ ...editForm, email: e.target.value })
                                                }
                                            />
                                        ) : (
                                            <a
                                                href={`mailto:${user.email}`}
                                                className="font-medium text-primary hover:underline block"
                                            >
                                                {user.email}
                                            </a>
                                        )}
                                    </div>

                                    <div className="space-y-2">
                                        <Label className="text-sm text-muted-foreground">Employee ID</Label>
                                        {isEditing ? (
                                            <Input
                                                value={editForm.employee_id}
                                                onChange={(e) =>
                                                    setEditForm({ ...editForm, employee_id: e.target.value })
                                                }
                                                placeholder="EMP001"
                                            />
                                        ) : user.employee_id ? (
                                            <p className="font-medium">{user.employee_id}</p>
                                        ) : (
                                            <p className="text-muted-foreground">Not assigned</p>
                                        )}
                                    </div>

                                    <div className="space-y-2">
                                        <Label className="text-sm text-muted-foreground">Phone</Label>
                                        {isEditing ? (
                                            <Input
                                                value={editForm.phone}
                                                onChange={(e) =>
                                                    setEditForm({ ...editForm, phone: e.target.value })
                                                }
                                                placeholder="Optional"
                                            />
                                        ) : user.phone ? (
                                            <a
                                                href={`tel:${user.phone}`}
                                                className="font-medium text-primary hover:underline block"
                                            >
                                                {user.phone}
                                            </a>
                                        ) : (
                                            <p className="text-muted-foreground">Not provided</p>
                                        )}
                                    </div>

                                    {isEditing && (
                                        <div className="space-y-2">
                                            <Label className="text-sm text-muted-foreground">
                                                Change Password
                                            </Label>
                                            <Input
                                                type="password"
                                                value={editForm.password}
                                                onChange={(e) =>
                                                    setEditForm({ ...editForm, password: e.target.value })
                                                }
                                                placeholder="Leave blank to keep current"
                                            />
                                        </div>
                                    )}
                                </div>
                            </CardContent>
                        </Card>

                        {/* Employment Details */}
                        <Card>
                            <CardHeader>
                                <CardTitle className="flex items-center gap-2">
                                    <Briefcase className="h-5 w-5" />
                                    Employment Details
                                </CardTitle>
                                <CardDescription>
                                    Job role, department, and reporting structure
                                </CardDescription>
                            </CardHeader>
                            <CardContent>
                                <div className="grid gap-4 sm:grid-cols-2">
                                    <div className="space-y-2">
                                        <Label className="text-sm text-muted-foreground">Department</Label>
                                        {isEditing ? (
                                            <Select
                                                value={editForm.department_id || 'none'}
                                                onValueChange={(value) =>
                                                    setEditForm({
                                                        ...editForm,
                                                        department_id: value === 'none' ? '' : value,
                                                    })
                                                }
                                            >
                                                <SelectTrigger>
                                                    <SelectValue placeholder="Select department" />
                                                </SelectTrigger>
                                                <SelectContent>
                                                    <SelectItem value="none">No Department</SelectItem>
                                                    {departments.map((dept) => (
                                                        <SelectItem key={dept.id} value={dept.id.toString()}>
                                                            {dept.name}
                                                        </SelectItem>
                                                    ))}
                                                </SelectContent>
                                            </Select>
                                        ) : user.department ? (
                                            <p className="font-medium">{user.department.name}</p>
                                        ) : (
                                            <p className="text-muted-foreground">Not assigned</p>
                                        )}
                                    </div>

                                    <div className="space-y-2">
                                        <Label className="text-sm text-muted-foreground">Designation</Label>
                                        {isEditing ? (
                                            <Select
                                                value={editForm.designation_id || 'none'}
                                                onValueChange={(value) =>
                                                    setEditForm({
                                                        ...editForm,
                                                        designation_id: value === 'none' ? '' : value,
                                                    })
                                                }
                                            >
                                                <SelectTrigger>
                                                    <SelectValue placeholder="Select designation" />
                                                </SelectTrigger>
                                                <SelectContent>
                                                    <SelectItem value="none">No Designation</SelectItem>
                                                    {designations.map((desig) => (
                                                        <SelectItem key={desig.id} value={desig.id.toString()}>
                                                            {desig.name} ({desig.level})
                                                        </SelectItem>
                                                    ))}
                                                </SelectContent>
                                            </Select>
                                        ) : user.designation ? (
                                            <p className="font-medium">{user.designation.name} <span className="text-sm text-muted-foreground">({user.designation.level})</span></p>
                                        ) : (
                                            <p className="text-muted-foreground">Not assigned</p>
                                        )}
                                    </div>

                                    <div className="space-y-2">
                                        <Label className="text-sm text-muted-foreground">
                                            Reporting Manager
                                        </Label>
                                        {isEditing ? (
                                            <Select
                                                value={editForm.reporting_manager_id || 'none'}
                                                onValueChange={(value) =>
                                                    setEditForm({
                                                        ...editForm,
                                                        reporting_manager_id:
                                                            value === 'none' ? '' : value,
                                                    })
                                                }
                                            >
                                                <SelectTrigger>
                                                    <SelectValue placeholder="Select manager" />
                                                </SelectTrigger>
                                                <SelectContent>
                                                    <SelectItem value="none">No Manager</SelectItem>
                                                    {availableManagers.map((manager) => (
                                                        <SelectItem
                                                            key={manager.id}
                                                            value={manager.id.toString()}
                                                        >
                                                            {manager.name}
                                                            {manager.designation?.name &&
                                                                ` (${manager.designation.name})`}
                                                        </SelectItem>
                                                    ))}
                                                </SelectContent>
                                            </Select>
                                        ) : user.reporting_manager ? (
                                            <p className="font-medium">{user.reporting_manager.name}</p>
                                        ) : (
                                            <p className="text-muted-foreground">No manager assigned</p>
                                        )}
                                    </div>

                                    <div className="space-y-2">
                                        <Label className="text-sm text-muted-foreground">
                                            Date of Joining
                                        </Label>
                                        {isEditing ? (
                                            <Input
                                                type="date"
                                                value={editForm.date_of_joining}
                                                onChange={(e) =>
                                                    setEditForm({
                                                        ...editForm,
                                                        date_of_joining: e.target.value,
                                                    })
                                                }
                                            />
                                        ) : user.date_of_joining ? (
                                            <p className="font-medium">
                                                {new Date(user.date_of_joining).toLocaleDateString()}
                                            </p>
                                        ) : (
                                            <p className="text-muted-foreground">Not provided</p>
                                        )}
                                    </div>

                                    <div className="space-y-2">
                                        <Label className="text-sm text-muted-foreground">Date of Exit</Label>
                                        {isEditing ? (
                                            <Input
                                                type="date"
                                                value={editForm.date_of_exit}
                                                onChange={(e) =>
                                                    setEditForm({
                                                        ...editForm,
                                                        date_of_exit: e.target.value,
                                                    })
                                                }
                                            />
                                        ) : user.date_of_exit ? (
                                            <p className="font-medium">
                                                {new Date(user.date_of_exit).toLocaleDateString()}
                                            </p>
                                        ) : (
                                            <p className="text-muted-foreground">Currently employed</p>
                                        )}
                                    </div>
                                </div>
                            </CardContent>
                        </Card>

                        {/* Bank Details */}
                        <Card>
                            <CardHeader>
                                <CardTitle className="flex items-center gap-2">
                                    <CreditCard className="h-5 w-5" />
                                    Bank Details
                                </CardTitle>
                                <CardDescription>
                                    Account information for salary payments
                                </CardDescription>
                            </CardHeader>
                            <CardContent>
                                <div className="grid gap-4 sm:grid-cols-2">
                                    <div className="space-y-2">
                                        <Label className="text-sm text-muted-foreground">
                                            Account Number
                                        </Label>
                                        {isEditing ? (
                                            <Input
                                                value={editForm.account_number}
                                                onChange={(e) =>
                                                    setEditForm({
                                                        ...editForm,
                                                        account_number: e.target.value,
                                                    })
                                                }
                                                placeholder="Enter account number"
                                            />
                                        ) : user.account_number ? (
                                            <p className="font-medium font-mono">{user.account_number}</p>
                                        ) : (
                                            <p className="text-muted-foreground">Not provided</p>
                                        )}
                                    </div>

                                    <div className="space-y-2">
                                        <Label className="text-sm text-muted-foreground">IFSC Code</Label>
                                        {isEditing ? (
                                            <Input
                                                value={editForm.ifsc_code}
                                                onChange={(e) =>
                                                    setEditForm({
                                                        ...editForm,
                                                        ifsc_code: e.target.value.toUpperCase(),
                                                    })
                                                }
                                                placeholder="e.g., SBIN0001234"
                                                maxLength={11}
                                            />
                                        ) : user.ifsc_code ? (
                                            <p className="font-medium font-mono">{user.ifsc_code}</p>
                                        ) : (
                                            <p className="text-muted-foreground">Not provided</p>
                                        )}
                                    </div>

                                    <div className="space-y-2 sm:col-span-2">
                                        <Label className="text-sm text-muted-foreground">Bank Name</Label>
                                        {isEditing ? (
                                            <Input
                                                value={editForm.bank_name}
                                                onChange={(e) =>
                                                    setEditForm({ ...editForm, bank_name: e.target.value })
                                                }
                                                placeholder="e.g., State Bank of India"
                                            />
                                        ) : user.bank_name ? (
                                            <p className="font-medium">{user.bank_name}</p>
                                        ) : (
                                            <p className="text-muted-foreground">Not provided</p>
                                        )}
                                    </div>
                                </div>
                            </CardContent>
                        </Card>

                        {/* Government IDs */}
                        <Card>
                            <CardHeader>
                                <CardTitle className="flex items-center gap-2">
                                    <IdCard className="h-5 w-5" />
                                    Government IDs
                                </CardTitle>
                                <CardDescription>
                                    Tax, insurance, and identification numbers
                                </CardDescription>
                            </CardHeader>
                            <CardContent>
                                <div className="grid gap-4 sm:grid-cols-2">
                                    <div className="space-y-2">
                                        <Label className="text-sm text-muted-foreground">PAN Number</Label>
                                        {isEditing ? (
                                            <Input
                                                value={editForm.pan_number}
                                                onChange={(e) =>
                                                    setEditForm({
                                                        ...editForm,
                                                        pan_number: e.target.value.toUpperCase(),
                                                    })
                                                }
                                                placeholder="e.g., ABCDE1234F"
                                                maxLength={10}
                                            />
                                        ) : user.pan_number ? (
                                            <p className="font-medium font-mono">{user.pan_number}</p>
                                        ) : (
                                            <p className="text-muted-foreground">Not provided</p>
                                        )}
                                    </div>

                                    <div className="space-y-2">
                                        <Label className="text-sm text-muted-foreground">
                                            Aadhar Number
                                        </Label>
                                        {isEditing ? (
                                            <Input
                                                value={editForm.aadhar_number}
                                                onChange={(e) =>
                                                    setEditForm({
                                                        ...editForm,
                                                        aadhar_number: e.target.value,
                                                    })
                                                }
                                                placeholder="12-digit number"
                                                maxLength={12}
                                            />
                                        ) : user.aadhar_number ? (
                                            <p className="font-medium font-mono">{user.aadhar_number}</p>
                                        ) : (
                                            <p className="text-muted-foreground">Not provided</p>
                                        )}
                                    </div>

                                    <div className="space-y-2">
                                        <Label className="text-sm text-muted-foreground">ESI Number</Label>
                                        {isEditing ? (
                                            <Input
                                                value={editForm.esi_number}
                                                onChange={(e) =>
                                                    setEditForm({
                                                        ...editForm,
                                                        esi_number: e.target.value,
                                                    })
                                                }
                                                placeholder="Enter ESI number"
                                            />
                                        ) : user.esi_number ? (
                                            <p className="font-medium font-mono">{user.esi_number}</p>
                                        ) : (
                                            <p className="text-muted-foreground">Not provided</p>
                                        )}
                                    </div>

                                    <div className="space-y-2">
                                        <Label className="text-sm text-muted-foreground">PF Number</Label>
                                        {isEditing ? (
                                            <Input
                                                value={editForm.pf_number}
                                                onChange={(e) =>
                                                    setEditForm({ ...editForm, pf_number: e.target.value })
                                                }
                                                placeholder="Enter PF number"
                                            />
                                        ) : user.pf_number ? (
                                            <p className="font-medium font-mono">{user.pf_number}</p>
                                        ) : (
                                            <p className="text-muted-foreground">Not provided</p>
                                        )}
                                    </div>
                                </div>
                            </CardContent>
                        </Card>

                        {/* Salary Structure */}
                        <Card>
                            <CardHeader className="flex flex-row items-center justify-between">
                                <div>
                                    <CardTitle className="flex items-center gap-2">
                                        <Banknote className="h-5 w-5" />
                                        Salary Structure
                                    </CardTitle>
                                    <CardDescription>Monthly compensation breakdown</CardDescription>
                                </div>
                                {!isSalaryEditing ? (
                                    <Button variant="outline" size="sm" onClick={() => setIsSalaryEditing(true)}>
                                        <Edit className="mr-2 h-4 w-4" />
                                        {salaryStructure ? 'Edit' : 'Set Up'}
                                    </Button>
                                ) : (
                                    <div className="flex gap-2">
                                        <Button size="sm" onClick={handleSaveSalary} disabled={savingSalary}>
                                            <Save className="mr-2 h-4 w-4" />
                                            {savingSalary ? 'Saving…' : 'Save'}
                                        </Button>
                                        <Button variant="outline" size="sm" onClick={() => setIsSalaryEditing(false)} disabled={savingSalary}>
                                            <X className="mr-2 h-4 w-4" />
                                            Cancel
                                        </Button>
                                    </div>
                                )}
                            </CardHeader>
                            <CardContent>
                                {isSalaryEditing ? (
                                    <div className="space-y-4">
                                        <div className="grid gap-4 sm:grid-cols-2">
                                            {[
                                                { key: 'basic_salary', label: 'Basic Salary (₹)', placeholder: '0.00' },
                                                { key: 'hra', label: 'HRA (₹)', placeholder: '0.00' },
                                                { key: 'transport_allowance', label: 'Transport Allowance (₹)', placeholder: '0.00' },
                                                { key: 'other_allowances', label: 'Other Allowances (₹)', placeholder: '0.00' },
                                                { key: 'pf_deduction', label: 'PF Deduction (₹)', placeholder: '0.00' },
                                                { key: 'esi_deduction', label: 'ESI Deduction (₹)', placeholder: '0.00' },
                                                { key: 'tds', label: 'TDS (₹)', placeholder: '0.00' },
                                            ].map(({ key, label, placeholder }) => (
                                                <div key={key} className="space-y-2">
                                                    <Label className="text-sm text-muted-foreground">{label}</Label>
                                                    <Input
                                                        type="number"
                                                        min="0"
                                                        step="0.01"
                                                        value={salaryForm[key as keyof typeof salaryForm]}
                                                        onChange={(e) => setSalaryForm({ ...salaryForm, [key]: e.target.value })}
                                                        placeholder={placeholder}
                                                    />
                                                </div>
                                            ))}
                                            <div className="space-y-2">
                                                <Label className="text-sm text-muted-foreground">Effective From</Label>
                                                <Input
                                                    type="date"
                                                    value={salaryForm.effective_from}
                                                    onChange={(e) => setSalaryForm({ ...salaryForm, effective_from: e.target.value })}
                                                />
                                            </div>
                                        </div>
                                        {/* Live preview */}
                                        {salaryForm.basic_salary && (
                                            <div className="rounded-md bg-muted p-3 text-sm space-y-1">
                                                {(() => {
                                                    const gross = ['basic_salary','hra','transport_allowance','other_allowances'].reduce((s, k) => s + (parseFloat(salaryForm[k as keyof typeof salaryForm] as string) || 0), 0);
                                                    const deductions = ['pf_deduction','esi_deduction','tds'].reduce((s, k) => s + (parseFloat(salaryForm[k as keyof typeof salaryForm] as string) || 0), 0);
                                                    return <>
                                                        <div className="flex justify-between"><span className="text-muted-foreground">Gross Salary</span><span className="font-medium">₹ {gross.toLocaleString('en-IN', { minimumFractionDigits: 2 })}</span></div>
                                                        <div className="flex justify-between"><span className="text-muted-foreground">Total Deductions</span><span className="font-medium text-destructive">₹ {deductions.toLocaleString('en-IN', { minimumFractionDigits: 2 })}</span></div>
                                                        <Separator />
                                                        <div className="flex justify-between font-semibold"><span>Net Salary</span><span className="text-green-600">₹ {Math.max(0, gross - deductions).toLocaleString('en-IN', { minimumFractionDigits: 2 })}</span></div>
                                                    </>;
                                                })()}
                                            </div>
                                        )}
                                    </div>
                                ) : salaryStructure ? (
                                    <div className="space-y-4">
                                        <div className="grid gap-3 sm:grid-cols-2">
                                            <div>
                                                <p className="text-sm text-muted-foreground">Basic Salary</p>
                                                <p className="font-medium">₹ {parseFloat(salaryStructure.basic_salary).toLocaleString('en-IN', { minimumFractionDigits: 2 })}</p>
                                            </div>
                                            <div>
                                                <p className="text-sm text-muted-foreground">HRA</p>
                                                <p className="font-medium">₹ {parseFloat(salaryStructure.hra).toLocaleString('en-IN', { minimumFractionDigits: 2 })}</p>
                                            </div>
                                            <div>
                                                <p className="text-sm text-muted-foreground">Transport Allowance</p>
                                                <p className="font-medium">₹ {parseFloat(salaryStructure.transport_allowance).toLocaleString('en-IN', { minimumFractionDigits: 2 })}</p>
                                            </div>
                                            <div>
                                                <p className="text-sm text-muted-foreground">Other Allowances</p>
                                                <p className="font-medium">₹ {parseFloat(salaryStructure.other_allowances).toLocaleString('en-IN', { minimumFractionDigits: 2 })}</p>
                                            </div>
                                        </div>
                                        <Separator />
                                        <div className="grid gap-3 sm:grid-cols-3">
                                            <div className="rounded-md bg-green-50 dark:bg-green-950 p-3 text-center">
                                                <p className="text-xs text-muted-foreground">Gross Salary</p>
                                                <p className="font-semibold text-green-700 dark:text-green-400">₹ {(salaryStructure.gross_salary ?? 0).toLocaleString('en-IN', { minimumFractionDigits: 2 })}</p>
                                            </div>
                                            <div className="rounded-md bg-red-50 dark:bg-red-950 p-3 text-center">
                                                <p className="text-xs text-muted-foreground">Deductions</p>
                                                <p className="font-semibold text-red-700 dark:text-red-400">₹ {(salaryStructure.total_deductions ?? 0).toLocaleString('en-IN', { minimumFractionDigits: 2 })}</p>
                                            </div>
                                            <div className="rounded-md bg-primary/10 p-3 text-center">
                                                <p className="text-xs text-muted-foreground">Net Salary</p>
                                                <p className="font-semibold text-primary">₹ {(salaryStructure.net_salary ?? 0).toLocaleString('en-IN', { minimumFractionDigits: 2 })}</p>
                                            </div>
                                        </div>
                                        <p className="text-xs text-muted-foreground">Effective from: {new Date(salaryStructure.effective_from).toLocaleDateString()}</p>
                                    </div>
                                ) : (
                                    <p className="text-sm text-muted-foreground">No salary structure configured. Click "Set Up" to add one.</p>
                                )}
                            </CardContent>
                        </Card>
                    </div>

                    {/* Sidebar */}
                    <div className="space-y-6">
                        {/* Status Card */}
                        <Card>
                            <CardHeader>
                                <CardTitle className="flex items-center gap-2">
                                    <Shield className="h-5 w-5" />
                                    Status
                                </CardTitle>
                            </CardHeader>
                            <CardContent className="space-y-4">
                                {isEditing ? (
                                    <Select
                                        value={user.status}
                                        onValueChange={handleStatusChange}
                                        disabled={updating}
                                    >
                                        <SelectTrigger>
                                            <SelectValue />
                                        </SelectTrigger>
                                        <SelectContent>
                                            <SelectItem value="active">Active</SelectItem>
                                            <SelectItem value="inactive">Inactive</SelectItem>
                                            <SelectItem value="suspended">Suspended</SelectItem>
                                        </SelectContent>
                                    </Select>
                                ) : (
                                    <div>{getStatusBadge(user.status)}</div>
                                )}
                            </CardContent>
                        </Card>

                        {/* Timeline Card */}
                        <Card>
                            <CardHeader>
                                <CardTitle className="flex items-center gap-2">
                                    <Calendar className="h-5 w-5" />
                                    Timeline
                                </CardTitle>
                            </CardHeader>
                            <CardContent className="space-y-4">
                                <div className="flex items-start gap-3">
                                    <div className="flex h-9 w-9 items-center justify-center rounded-full bg-primary/10">
                                        <Calendar className="h-4 w-4 text-primary" />
                                    </div>
                                    <div>
                                        <p className="text-sm font-medium">Created</p>
                                        <p className="text-sm text-muted-foreground">
                                            {new Date(user.created_at).toLocaleString()}
                                        </p>
                                    </div>
                                </div>
                                <Separator />
                                <div className="flex items-start gap-3">
                                    <div className="flex h-9 w-9 items-center justify-center rounded-full bg-primary/10">
                                        <Calendar className="h-4 w-4 text-primary" />
                                    </div>
                                    <div>
                                        <p className="text-sm font-medium">Last Updated</p>
                                        <p className="text-sm text-muted-foreground">
                                            {new Date(user.updated_at).toLocaleString()}
                                        </p>
                                    </div>
                                </div>
                            </CardContent>
                        </Card>
                    </div>
                </div>
            </div>
        </AppLayout>
    );
}