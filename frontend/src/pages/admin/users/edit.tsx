import { useNavigate } from 'react-router-dom';
import axios from '@/lib/axios';
import { ArrowLeft, Banknote, Briefcase, Building2, Camera, Save, Trash2, Upload, Users } from 'lucide-react';
import { useRef, useState, useEffect } from 'react';
import { useParams } from 'react-router-dom';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Checkbox } from '@/components/ui/checkbox';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
    Select,
    SelectContent,
    SelectItem,
    SelectTrigger,
    SelectValue,
} from '@/components/ui/select';
import AppLayout from '@/layouts/app-layout';
import { SalaryTabsPanel } from '@/components/salary-tabs-panel';
import { handleApiError, handleApiResponse } from '@/lib/toast';

interface Role {
    id: number;
    name: string;
    slug: string;
    description: string | null;
}

interface Department {
    id: number;
    name: string;
}

interface Designation {
    id: number;
    name: string;
}

interface User {
    id: number;
    name: string;
    email: string;
    employee_id?: string;
    phone?: string;
    photo?: string;
    department_id?: number;
    designation_id?: number;
    status: string;
    roles: Role[];
    created_at: string;
    // Employment
    date_of_joining?: string;
    work_location?: string;
    // Bank
    bank_name?: string;
    account_number?: string;
    ifsc_code?: string;
    account_type?: string;
}

interface Center {
    id: string;
    name: string;
    address_line1?: string;
    city?: string;
    state?: string;
}

interface EditUserPageProps {
    user?: User;
    roles?: Role[];
    departments?: Department[];
    designations?: Designation[];
    centers?: Center[];
}

export default function EditUserPage() {
    const navigate = useNavigate();
    const { id } = useParams();
    const isSuperAdmin = false; // managed via Settings > Centers

    const [user, setUser] = useState<User | null>(null);
    const [roles, setRoles] = useState<Role[]>([]);
    const [departments, setDepartments] = useState<Department[]>([]);
    const [designations, setDesignations] = useState<Designation[]>([]);
    const [centers, setCenters] = useState<Center[]>([]);
    const [loading, setLoading] = useState(true);

    const [formData, setFormData] = useState({
        name: '',
        email: '',
        employee_id: '',
        phone: '',
        department_id: '' as string | number,
        designation_id: '' as string | number,
        status: 'active',
        roles: [] as number[],
        date_of_joining: '',
        work_location: '',
        bank_name: '',
        account_number: '',
        ifsc_code: '',
        account_type: '',
    });
    const [photoFile, setPhotoFile] = useState<File | null>(null);
    const [photoPreview, setPhotoPreview] = useState<string | null>(null);
    const [removePhoto, setRemovePhoto] = useState(false);
    const fileInputRef = useRef<HTMLInputElement>(null);
    const webcamVideoRef = useRef<HTMLVideoElement | null>(null);
    const webcamStreamRef = useRef<MediaStream | null>(null);
    const [errors, setErrors] = useState<Record<string, string[]>>({});
    const [saving, setSaving] = useState(false);
    const [saved, setSaved] = useState(false);
    const [webcamOpen, setWebcamOpen] = useState(false);
    const [webcamStarting, setWebcamStarting] = useState(false);
    const [webcamError, setWebcamError] = useState<string | null>(null);

    useEffect(() => {
        const fetchData = async () => {
            try {
                const [userRes, rolesRes, deptsRes, desigsRes, centersRes] = await Promise.all([
                    axios.get(`/admin/users/${id}`),
                    axios.get('/admin/roles/list'),
                    axios.get('/admin/departments/list'),
                    axios.get('/admin/designations/list'),
                    axios.get('/admin/settings/centers')
                ]);

                const userData = userRes.data.data;
                setUser(userData);
                setRoles(rolesRes.data.data);
                setDepartments(deptsRes.data.data);
                setDesignations(desigsRes.data.data);
                setCenters(centersRes.data.data);

                setFormData({
                    name: userData.name,
                    email: userData.email,
                    employee_id: userData.employee_id || '',
                    phone: userData.phone || '',
                    department_id: userData.department_id || '',
                    designation_id: userData.designation_id || '',
                    status: userData.status || 'active',
                    roles: userData.roles?.map((r: any) => r.id) || [],
                    date_of_joining: userData.date_of_joining || '',
                    work_location: userData.work_location || '',
                    bank_name: userData.bank_name || '',
                    account_number: userData.account_number || '',
                    ifsc_code: userData.ifsc_code || '',
                    account_type: userData.account_type || '',
                });

                if (userData.photo) {
                    setPhotoPreview(
                        userData.photo.startsWith('http')
                            ? userData.photo
                            : `/storage/${userData.photo}`
                    );
                }
            } catch (error) {
                console.error('Failed to fetch data:', error);
                handleApiError(error);
                navigate('/admin/users');
            } finally {
                setLoading(false);
            }
        };

        if (id) fetchData();
    }, [id, navigate]);

    useEffect(() => {
        return () => {
            stopWebcam();
        };
    }, []);

    const handlePhotoChange = (e: React.ChangeEvent<HTMLInputElement>) => {
        const file = e.target.files?.[0];
        if (file) {
            if (file.size > 2 * 1024 * 1024) {
                setErrors({ ...errors, photo: ['Photo must be less than 2MB'] });
                return;
            }
            setPhotoFile(file);
            setRemovePhoto(false);
            setWebcamError(null);
            const reader = new FileReader();
            reader.onload = (ev) => setPhotoPreview(ev.target?.result as string);
            reader.readAsDataURL(file);
        }
    };

    const handleRemovePhoto = () => {
        setPhotoFile(null);
        setPhotoPreview(null);
        setRemovePhoto(true);
        setWebcamError(null);
        if (fileInputRef.current) fileInputRef.current.value = '';
    };

    const stopWebcam = () => {
        if (webcamStreamRef.current) {
            webcamStreamRef.current.getTracks().forEach((track) => track.stop());
            webcamStreamRef.current = null;
        }
        if (webcamVideoRef.current) {
            webcamVideoRef.current.srcObject = null;
        }
    };

    const startWebcam = async () => {
        setWebcamStarting(true);
        setWebcamError(null);
        try {
            const stream = await navigator.mediaDevices.getUserMedia({
                video: { facingMode: 'user' },
                audio: false,
            });
            webcamStreamRef.current = stream;
            setWebcamOpen(true);
            requestAnimationFrame(() => {
                if (webcamVideoRef.current) {
                    webcamVideoRef.current.srcObject = stream;
                    void webcamVideoRef.current.play();
                }
            });
        } catch {
            setWebcamError('Unable to access webcam. Please allow camera permission and retry.');
            setWebcamOpen(false);
        } finally {
            setWebcamStarting(false);
        }
    };

    const closeWebcam = () => {
        stopWebcam();
        setWebcamOpen(false);
    };

    const captureFromWebcam = async () => {
        const video = webcamVideoRef.current;
        if (!video || !video.videoWidth || !video.videoHeight) {
            setWebcamError('Camera is not ready yet. Please wait and try again.');
            return;
        }

        const canvas = document.createElement('canvas');
        canvas.width = video.videoWidth;
        canvas.height = video.videoHeight;
        const ctx = canvas.getContext('2d');
        if (!ctx) {
            setWebcamError('Failed to capture image from webcam.');
            return;
        }

        ctx.drawImage(video, 0, 0, canvas.width, canvas.height);
        const blob = await new Promise<Blob | null>((resolve) => canvas.toBlob(resolve, 'image/jpeg', 0.9));
        if (!blob) {
            setWebcamError('Unable to create captured image.');
            return;
        }
        if (blob.size > 2 * 1024 * 1024) {
            setWebcamError('Captured photo is larger than 2MB. Move closer and try again.');
            return;
        }

        const capturedFile = new File([blob], `webcam-user-${user?.id ?? 'photo'}.jpg`, {
            type: 'image/jpeg',
        });
        setPhotoFile(capturedFile);
        setRemovePhoto(false);
        setPhotoPreview(URL.createObjectURL(blob));
        setWebcamError(null);
        closeWebcam();
    };

    const handleSubmit = async (e: React.FormEvent) => {
        e.preventDefault();
        if (!user) return;
        setSaving(true);
        setErrors({});
        setSaved(false);

        try {
            let response;

            if (photoFile || removePhoto) {
                const fd = new FormData();
                fd.append('name', formData.name);
                fd.append('email', formData.email);
                fd.append('employee_id', formData.employee_id);
                fd.append('phone', formData.phone);
                fd.append('status', formData.status);
                fd.append('department_id', formData.department_id || '');
                fd.append('designation_id', formData.designation_id || '');
                fd.append('date_of_joining', formData.date_of_joining);
                fd.append('work_location', formData.work_location);
                fd.append('bank_name', formData.bank_name);
                fd.append('account_number', formData.account_number);
                fd.append('ifsc_code', formData.ifsc_code);
                fd.append('account_type', formData.account_type);
                fd.append('roles', JSON.stringify(formData.roles));
                if (photoFile) fd.append('photo', photoFile);
                if (removePhoto) fd.append('remove_photo', '1');

                response = await axios.post(`/admin/users/${user.id}`, fd);
            } else {
                const payload = {
                    name: formData.name,
                    email: formData.email,
                    employee_id: formData.employee_id,
                    phone: formData.phone,
                    status: formData.status,
                    department_id: formData.department_id ? Number(formData.department_id) : null,
                    designation_id: formData.designation_id ? Number(formData.designation_id) : null,
                    date_of_joining: formData.date_of_joining,
                    work_location: formData.work_location,
                    bank_name: formData.bank_name,
                    account_number: formData.account_number,
                    ifsc_code: formData.ifsc_code,
                    account_type: formData.account_type,
                    roles: formData.roles,
                };
                response = await axios.put(`/admin/users/${user.id}`, payload);
            }

            handleApiResponse(response);

            if (response.data?.data?.photo) {
                const photo = response.data.data.photo as string;
                setPhotoPreview(
                    photo.startsWith('http') ? photo : `/storage/${photo}`,
                );
                setPhotoFile(null);
                setRemovePhoto(false);
            } else if (removePhoto) {
                setPhotoPreview(null);
                setPhotoFile(null);
                setRemovePhoto(false);
            }

            setSaved(true);
            setTimeout(() => setSaved(false), 3000);
        } catch (error: any) {
            if (error.response?.data?.errors) {
                setErrors(error.response.data.errors);
            }
            handleApiError(error);
        } finally {
            setSaving(false);
        }
    };

    const handleRoleToggle = (roleId: number) => {
        setFormData((prev) => ({
            ...prev,
            roles: prev.roles.includes(roleId)
                ? prev.roles.filter((id) => id !== roleId)
                : [...prev.roles, roleId],
        }));
    };

    if (loading) {
        return (
            <AppLayout breadcrumbs={[{ label: 'Users', href: '/admin/users' }, { label: 'Loading...' }]}>
                <div className="flex h-[400px] items-center justify-center">
                    <div className="h-8 w-8 animate-spin rounded-full border-4 border-primary border-t-transparent"></div>
                </div>
            </AppLayout>
        );
    }

    if (!user) {
        return (
            <AppLayout breadcrumbs={[{ label: 'Users', href: '/admin/users' }, { label: 'Not Found' }]}>
                <div className="flex h-[400px] flex-col items-center justify-center gap-4">
                    <h2 className="text-xl font-semibold">User not found</h2>
                    <Button onClick={() => navigate('/admin/users')}>Back to Users</Button>
                </div>
            </AppLayout>
        );
    }

    const breadcrumbs = [
        { label: 'Users', href: '/admin/users' },
        { label: user.name, href: `/admin/users/${user.id}` },
        { label: 'Edit', href: '#' },
    ];

    return (
        <AppLayout breadcrumbs={breadcrumbs}>

            <div className="space-y-6">
                {/* Header */}
                <div className="flex items-center justify-between">
                    <div className="space-y-1">
                        <div className="flex items-center gap-3">
                            <Button
                                variant="ghost"
                                size="icon"
                                onClick={() => navigate('/admin/users')}
                            >
                                <ArrowLeft className="h-4 w-4" />
                            </Button>
                            <div>
                                <h1 className="text-3xl font-bold tracking-tight flex items-center gap-2">
                                    <Users className="h-8 w-8 text-primary" />
                                    Edit User
                                </h1>
                                <p className="text-muted-foreground">
                                    Update user information and assign roles
                                </p>
                            </div>
                        </div>
                    </div>
                    <div className="flex items-center gap-2">
                        <Badge variant={formData.status === 'active' ? 'default' : 'secondary'}>
                            {formData.status}
                        </Badge>
                    </div>
                </div>

                <form onSubmit={handleSubmit} className="space-y-6">
                    {/* Profile Photo */}
                    <Card>
                        <CardHeader>
                            <CardTitle>Profile Photo</CardTitle>
                            <CardDescription>
                                Upload a profile photo for this user (max 2MB, JPG/PNG)
                            </CardDescription>
                        </CardHeader>
                        <CardContent>
                            <div className="flex items-center gap-6">
                                <div className="relative group">
                                    <div className="h-24 w-24 rounded-full overflow-hidden border-2 border-muted bg-muted flex items-center justify-center">
                                        {photoPreview ? (
                                            <img
                                                src={photoPreview}
                                                alt={formData.name}
                                                className="h-full w-full object-cover"
                                            />
                                        ) : (
                                            <Camera className="h-8 w-8 text-muted-foreground" />
                                        )}
                                    </div>
                                    <button
                                        type="button"
                                        onClick={() => fileInputRef.current?.click()}
                                        className="absolute inset-0 rounded-full bg-black/40 opacity-0 group-hover:opacity-100 transition-opacity flex items-center justify-center cursor-pointer"
                                    >
                                        <Upload className="h-5 w-5 text-white" />
                                    </button>
                                </div>
                                <div className="space-y-2">
                                    <input
                                        ref={fileInputRef}
                                        type="file"
                                        accept="image/jpeg,image/png,image/gif,image/webp"
                                        onChange={handlePhotoChange}
                                        className="hidden"
                                    />
                                    <div className="flex gap-2">
                                        <Button
                                            type="button"
                                            variant="outline"
                                            size="sm"
                                            onClick={() => fileInputRef.current?.click()}
                                        >
                                            <Upload className="mr-2 h-4 w-4" />
                                            {photoPreview ? 'Change Photo' : 'Upload Photo'}
                                        </Button>
                                        <Button
                                            type="button"
                                            variant="outline"
                                            size="sm"
                                            onClick={startWebcam}
                                            disabled={webcamStarting}
                                        >
                                            <Camera className="mr-2 h-4 w-4" />
                                            {webcamStarting ? 'Opening...' : 'Open Webcam'}
                                        </Button>
                                        {photoPreview && (
                                            <Button
                                                type="button"
                                                variant="outline"
                                                size="sm"
                                                onClick={handleRemovePhoto}
                                                className="text-destructive hover:text-destructive"
                                            >
                                                <Trash2 className="mr-2 h-4 w-4" />
                                                Remove
                                            </Button>
                                        )}
                                    </div>
                                    <p className="text-xs text-muted-foreground">
                                        Accepted formats: JPG, PNG, GIF, WebP. Max size: 2MB.
                                    </p>
                                    {webcamError && (
                                        <p className="text-sm text-destructive">{webcamError}</p>
                                    )}
                                    {errors.photo && (
                                        <p className="text-sm text-destructive">{errors.photo[0]}</p>
                                    )}
                                </div>
                            </div>
                            {webcamOpen && (
                                <div className="mt-4 rounded-lg border p-3 space-y-3">
                                    <p className="text-sm font-medium">Webcam Capture</p>
                                    <div className="max-w-md overflow-hidden rounded-md border bg-muted">
                                        <video
                                            ref={webcamVideoRef}
                                            className="w-full h-auto"
                                            autoPlay
                                            muted
                                            playsInline
                                        />
                                    </div>
                                    <div className="flex gap-2">
                                        <Button type="button" size="sm" onClick={captureFromWebcam}>
                                            Capture Photo
                                        </Button>
                                        <Button type="button" size="sm" variant="outline" onClick={closeWebcam}>
                                            Cancel Webcam
                                        </Button>
                                    </div>
                                </div>
                            )}
                        </CardContent>
                    </Card>

                    {/* Basic Information */}
                    <Card>
                        <CardHeader>
                            <CardTitle>Basic Information</CardTitle>
                            <CardDescription>
                                Update user details
                            </CardDescription>
                        </CardHeader>
                        <CardContent className="space-y-4">
                            <div className="grid grid-cols-2 gap-4">
                                <div className="space-y-2">
                                    <Label htmlFor="name">
                                        Name <span className="text-destructive">*</span>
                                    </Label>
                                    <Input
                                        id="name"
                                        value={formData.name}
                                        onChange={(e) =>
                                            setFormData({ ...formData, name: e.target.value })
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
                                        value={formData.email}
                                        onChange={(e) =>
                                            setFormData({ ...formData, email: e.target.value })
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
                                        value={formData.employee_id}
                                        onChange={(e) =>
                                            setFormData({ ...formData, employee_id: e.target.value })
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
                                        value={formData.phone}
                                        onChange={(e) =>
                                            setFormData({ ...formData, phone: e.target.value })
                                        }
                                        placeholder="Phone number"
                                    />
                                    {errors.phone && (
                                        <p className="text-sm text-destructive">{errors.phone[0]}</p>
                                    )}
                                </div>

                                <div className="space-y-2">
                                    <Label htmlFor="status">Status</Label>
                                    <Select
                                        value={formData.status}
                                        onValueChange={(value) =>
                                            setFormData({ ...formData, status: value })
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
                                    {errors.status && (
                                        <p className="text-sm text-destructive">
                                            {errors.status[0]}
                                        </p>
                                    )}
                                </div>

                                <div className="space-y-2">
                                    <Label htmlFor="department">Department</Label>
                                    <Select
                                        value={String(formData.department_id) || ''}
                                        onValueChange={(value) =>
                                            setFormData({
                                                ...formData,
                                                department_id: value ? parseInt(value) : '',
                                            })
                                        }
                                    >
                                        <SelectTrigger id="department">
                                            <SelectValue placeholder="Select department" />
                                        </SelectTrigger>
                                        <SelectContent>
                                            {departments.map((dept) => (
                                                <SelectItem key={dept.id} value={String(dept.id)}>
                                                    {dept.name}
                                                </SelectItem>
                                            ))}
                                        </SelectContent>
                                    </Select>
                                    {errors.department_id && (
                                        <p className="text-sm text-destructive">
                                            {errors.department_id[0]}
                                        </p>
                                    )}
                                </div>

                                <div className="space-y-2">
                                    <Label htmlFor="designation">Designation</Label>
                                    <Select
                                        value={String(formData.designation_id) || ''}
                                        onValueChange={(value) =>
                                            setFormData({
                                                ...formData,
                                                designation_id: value ? parseInt(value) : '',
                                            })
                                        }
                                    >
                                        <SelectTrigger id="designation">
                                            <SelectValue placeholder="Select designation" />
                                        </SelectTrigger>
                                        <SelectContent>
                                            {designations.map((desig) => (
                                                <SelectItem key={desig.id} value={String(desig.id)}>
                                                    {desig.name}
                                                </SelectItem>
                                            ))}
                                        </SelectContent>
                                    </Select>
                                    {errors.designation_id && (
                                        <p className="text-sm text-destructive">
                                            {errors.designation_id[0]}
                                        </p>
                                    )}
                                </div>
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
                            <CardDescription>Work-related information for this employee</CardDescription>
                        </CardHeader>
                        <CardContent className="space-y-4">
                            <div className="grid grid-cols-2 gap-4">
                                <div className="space-y-2">
                                    <Label htmlFor="date_of_joining">Date of Joining</Label>
                                    <Input
                                        id="date_of_joining"
                                        type="date"
                                        value={formData.date_of_joining as string}
                                        onChange={(e) => setFormData({ ...formData, date_of_joining: e.target.value })}
                                    />
                                    {errors.date_of_joining && (
                                        <p className="text-sm text-destructive">{errors.date_of_joining[0]}</p>
                                    )}
                                </div>

                                {/* Center Dropdown */}
                                <div className="space-y-2">
                                    <Label htmlFor="work_location">Center</Label>
                                    {centers.length === 0 ? (
                                        <p className="text-sm text-muted-foreground italic py-2">
                                            No centers configured. Contact admin to add centers via Settings → General Settings.
                                        </p>
                                    ) : (
                                        <Select
                                            value={formData.work_location as string}
                                            onValueChange={(v) => setFormData({ ...formData, work_location: v })}
                                        >
                                            <SelectTrigger id="work_location">
                                                <SelectValue placeholder="Select center" />
                                            </SelectTrigger>
                                            <SelectContent>
                                                {centers.map((center) => (
                                                    <SelectItem key={center.id} value={center.id}>
                                                        {center.name}{center.city ? ` — ${center.city}` : ''}
                                                    </SelectItem>
                                                ))}
                                            </SelectContent>
                                        </Select>
                                    )}
                                    {errors.work_location && (
                                        <p className="text-sm text-destructive">{errors.work_location[0]}</p>
                                    )}
                                </div>
                            </div>
                        </CardContent>
                    </Card>

                    {/* Bank Details */}
                    <Card>
                        <CardHeader>
                            <CardTitle className="flex items-center gap-2">
                                <Building2 className="h-5 w-5" />
                                Bank Details
                            </CardTitle>
                            <CardDescription>Bank account details for salary transfer</CardDescription>
                        </CardHeader>
                        <CardContent className="space-y-4">
                            <div className="grid grid-cols-2 gap-4">
                                <div className="space-y-2">
                                    <Label htmlFor="bank_name">Bank Name</Label>
                                    <Input
                                        id="bank_name"
                                        value={formData.bank_name as string}
                                        onChange={(e) => setFormData({ ...formData, bank_name: e.target.value })}
                                        placeholder="e.g. HDFC Bank"
                                    />
                                    {errors.bank_name && (
                                        <p className="text-sm text-destructive">{errors.bank_name[0]}</p>
                                    )}
                                </div>

                                <div className="space-y-2">
                                    <Label htmlFor="account_type">Account Type</Label>
                                    <Select
                                        value={formData.account_type as string}
                                        onValueChange={(v) => setFormData({ ...formData, account_type: v })}
                                    >
                                        <SelectTrigger id="account_type">
                                            <SelectValue placeholder="Select account type" />
                                        </SelectTrigger>
                                        <SelectContent>
                                            <SelectItem value="savings">Savings</SelectItem>
                                            <SelectItem value="current">Current</SelectItem>
                                            <SelectItem value="salary">Salary Account</SelectItem>
                                        </SelectContent>
                                    </Select>
                                </div>

                                <div className="space-y-2">
                                    <Label htmlFor="account_number">Account Number</Label>
                                    <Input
                                        id="account_number"
                                        value={formData.account_number as string}
                                        onChange={(e) => setFormData({ ...formData, account_number: e.target.value })}
                                        placeholder="Bank account number"
                                    />
                                    {errors.account_number && (
                                        <p className="text-sm text-destructive">{errors.account_number[0]}</p>
                                    )}
                                </div>

                                <div className="space-y-2">
                                    <Label htmlFor="ifsc_code">IFSC Code</Label>
                                    <Input
                                        id="ifsc_code"
                                        value={formData.ifsc_code as string}
                                        onChange={(e) => setFormData({ ...formData, ifsc_code: e.target.value.toUpperCase() })}
                                        placeholder="e.g. HDFC0001234"
                                        maxLength={11}
                                        className="uppercase"
                                    />
                                    {errors.ifsc_code && (
                                        <p className="text-sm text-destructive">{errors.ifsc_code[0]}</p>
                                    )}
                                </div>
                            </div>
                        </CardContent>
                    </Card>

                    {/* Roles Assignment */}
                    <Card>
                        <CardHeader>
                            <div className="flex items-center justify-between">
                                <div>
                                    <CardTitle>Assign Roles</CardTitle>
                                    <CardDescription>
                                        Select roles for this user
                                    </CardDescription>
                                </div>
                                <Badge variant="outline">
                                    {formData.roles.length} role
                                    {formData.roles.length !== 1 ? 's' : ''} assigned
                                </Badge>
                            </div>
                        </CardHeader>
                        <CardContent className="space-y-4">
                            <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
                                {roles.map((role) => (
                                    <div key={role.id} className="flex items-start gap-3 p-3 border rounded-lg hover:bg-muted/50 transition-colors">
                                        <Checkbox
                                            id={`role-${role.id}`}
                                            checked={formData.roles.includes(role.id)}
                                            onCheckedChange={() => handleRoleToggle(role.id)}
                                            className="mt-1"
                                        />
                                        <div className="flex-1 min-w-0">
                                            <Label
                                                htmlFor={`role-${role.id}`}
                                                className="font-medium cursor-pointer"
                                            >
                                                {role.name}
                                            </Label>
                                            {role.description && (
                                                <p className="text-xs text-muted-foreground line-clamp-2">
                                                    {role.description}
                                                </p>
                                            )}
                                        </div>
                                    </div>
                                ))}
                            </div>
                            {errors.roles && (
                                <p className="text-sm text-destructive">{errors.roles[0]}</p>
                            )}
                            {roles.length === 0 && (
                                <p className="text-sm text-muted-foreground text-center py-8">
                                    No roles available. Create roles first.
                                </p>
                            )}
                        </CardContent>
                    </Card>

                    {/* Salary Structure */}
                    <Card>
                        <CardHeader>
                            <CardTitle className="flex items-center gap-2">
                                <Banknote className="h-5 w-5" />
                                Salary Structure
                            </CardTitle>
                            <CardDescription>Monthly compensation from CTC split or manual components</CardDescription>
                        </CardHeader>
                        <CardContent>
                            <SalaryTabsPanel userId={user.id} />
                        </CardContent>
                    </Card>

                    {/* Actions */}
                    <div className="flex items-center justify-between gap-3">
                        <div>
                            {saved && (
                                <div className="text-sm text-green-600 font-medium">
                                    ✓ Changes saved successfully
                                </div>
                            )}
                        </div>
                        <div className="flex gap-3">
                            <Button
                                type="button"
                                variant="outline"
                                onClick={() => navigate('/admin/users')}
                                disabled={saving}
                            >
                                Back to Users
                            </Button>
                            <Button type="submit" disabled={saving}>
                                {saving ? (
                                    <>Saving...</>
                                ) : saved ? (
                                    <>
                                        <span className="mr-2">✓</span>
                                        Saved
                                    </>
                                ) : (
                                    <>
                                        <Save className="mr-2 h-4 w-4" />
                                        Save Changes
                                    </>
                                )}
                            </Button>
                        </div>
                    </div>
                </form>
            </div>
        </AppLayout>
    );
}