import { useEffect, useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import axios from '@/lib/axios';
import EmployeePayslipsPage from '@/pages/admin/salaries/employee-payslips';
import { handleApiError } from '@/lib/toast';

interface Employee {
    id: number;
    name: string;
    email: string;
    employee_id: string | null;
}

export default function EmployeePayslipsRoute() {
    const { id } = useParams<{ id: string }>();
    const navigate = useNavigate();
    const [employee, setEmployee] = useState<Employee | null>(null);
    const [loading, setLoading] = useState(true);

    useEffect(() => {
        if (!id) return;
        (async () => {
            try {
                const res = await axios.get(`/admin/users/${id}`);
                const u = res.data.data;
                setEmployee({
                    id: u.id,
                    name: u.name,
                    email: u.email,
                    employee_id: u.employee_id ?? null,
                });
            } catch (e) {
                handleApiError(e);
                navigate('/admin/salaries/employees');
            } finally {
                setLoading(false);
            }
        })();
    }, [id, navigate]);

    if (loading) {
        return (
            <div className="flex min-h-[300px] items-center justify-center">
                <div className="h-8 w-8 animate-spin rounded-full border-4 border-primary border-t-transparent" />
            </div>
        );
    }
    if (!employee) return null;
    return <EmployeePayslipsPage employee={employee} />;
}
