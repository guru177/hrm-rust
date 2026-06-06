import sqlite3
c = sqlite3.connect(r"c:\Users\ASUS\Pictures\HRM\database\database.sqlite")
for row in c.execute("SELECT id,name,calculation_type,amount,deduction_type FROM salary_components ORDER BY id"):
    print(row)
